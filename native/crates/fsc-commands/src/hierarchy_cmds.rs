//! The scene hierarchy: parents and empty groups, as in Nomad Sculpt's scene tree (a child hangs under an object or a
//! group; moving the parent moves it; Delete keeps the children, Delete+ takes them too).

use fsc_core::ObjectId;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn node(s: &Session, v: &Value, what: &str) -> Result<ObjectId, CommandError> {
    let id = v.as_u64().ok_or_else(|| bad(format!("{what} is an object or group id")))? as ObjectId;
    if !s.scene.exists(id) {
        return Err(CommandError::new("not_found", format!("no object or group {id}; get_scene lists both")));
    }
    Ok(id)
}

/// The groups as get_scene reports them: each with its resolved parent and how many objects hang below it.
pub(crate) fn groups_report(s: &Session) -> Value {
    json!(s.scene.groups.iter().map(|g| json!({"id": g.id, "name": g.name, "parent": s.scene.parent_of(g.id), "objects": s.scene.descendants(g.id).len()})).collect::<Vec<_>>())
}

pub(crate) fn add_group_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {"type": "string", "maxLength": 120, "description": "The group's name; Group (numbered if taken) when omitted."},
            "parent": {"type": "integer", "minimum": 1, "description": "An object or group to hang the new group under; where the first child was, or the top, when omitted."},
            "children": {"type": "array", "items": {"type": "integer", "minimum": 1}, "maxItems": 4096, "description": "Objects or groups to move under the new group (grouping a selection)."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Makes an empty group, optionally gathering children under it. One undo step.
pub(crate) fn add_group(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let mut children = Vec::new();
    for v in args.get("children").and_then(Value::as_array).into_iter().flatten() {
        let c = node(s, v, "each child")?;
        if !children.contains(&c) {
            children.push(c);
        }
    }
    let parent = match args.get("parent").filter(|v| !v.is_null()) {
        Some(v) => Some(node(s, v, "parent")?),
        None => children.first().and_then(|&c| s.scene.parent_of(c)),
    };
    if let Some(p) = parent {
        if let Some(&c) = children.iter().find(|&&c| s.scene.is_ancestor(c, p)) {
            return Err(bad(format!("the parent {p} is {c} or below it, so {c} cannot go under the new group")));
        }
    }
    let base = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Group");
    let taken = |n: &str| s.scene.groups.iter().any(|g| g.name == n);
    let name: String = if taken(base) { (2..).map(|k| format!("{base} {k}")).find(|n| !taken(n)).unwrap_or_else(|| base.to_string()) } else { base.to_string() };
    let before = s.scene.hierarchy();
    let id = s.scene.add_group(name.chars().take(120).collect::<String>(), parent);
    for &c in &children {
        s.scene.set_parent(c, Some(id)).map_err(bad)?;
    }
    s.history.push(Edit::Hierarchy { label: "Add Group".into(), state: before });
    Ok(json!({"group": {"id": id, "name": name, "parent": parent, "objects": s.scene.descendants(id).len()}, "groups": groups_report(s)}))
}

pub(crate) fn set_parent_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ids": {"type": "array", "items": {"type": "integer", "minimum": 1}, "minItems": 1, "maxItems": 4096, "description": "Objects or groups to move in the hierarchy."},
            "parent": {"type": ["integer", "null"], "minimum": 1, "description": "The object or group they hang under from now on (moving it moves them); null puts them at the top."}
        },
        "required": ["ids", "parent"],
        "additionalProperties": false
    })
}

/// Hangs objects or groups under a parent, or at the top. Refuses a parent that is one of them or below one. One undo step.
pub(crate) fn set_parent(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let ids: Vec<ObjectId> = args.get("ids").and_then(Value::as_array).ok_or_else(|| bad("ids is required"))?.iter().map(|v| node(s, v, "each id")).collect::<Result<_, _>>()?;
    if ids.is_empty() {
        return Err(bad("ids needs at least one id"));
    }
    let parent = match args.get("parent") {
        Some(Value::Null) => None,
        Some(v) => Some(node(s, v, "parent")?),
        None => return Err(bad("parent is required (null for the top)")),
    };
    let before = s.scene.hierarchy();
    for &id in &ids {
        if let Err(e) = s.scene.set_parent(id, parent) {
            s.scene.swap_hierarchy(before);
            return Err(bad(e));
        }
    }
    s.history.push(Edit::Hierarchy { label: if parent.is_some() { "Set Parent".into() } else { "Clear Parent".into() }, state: before });
    Ok(json!({"moved": ids, "parent": parent, "groups": groups_report(s)}))
}

/// Removes an object or group. With `children`, every object and group below it goes too (Delete+); without, its
/// children move up to its own parent. One undo step either way.
pub(crate) fn remove_node(s: &mut Session, root: ObjectId, children: bool) -> Result<Value, CommandError> {
    if !s.scene.exists(root) {
        return Err(CommandError::new("not_found", format!("no object or group {root}; get_scene lists both")));
    }
    let before = s.scene.hierarchy();
    let up = s.scene.parent_of(root);
    let below_groups: Vec<ObjectId> = s.scene.groups.iter().map(|g| g.id).filter(|&g| g != root && s.scene.is_ancestor(root, g)).collect();
    let mut objects: Vec<ObjectId> = s.scene.get(root).map(|o| o.id).into_iter().collect();
    let mut drop_groups: Vec<ObjectId> = s.scene.group(root).map(|g| g.id).into_iter().collect();
    if children {
        objects.extend(s.scene.descendants(root));
        drop_groups.extend(below_groups);
    } else {
        let direct: Vec<ObjectId> = s.scene.objects.iter().map(|o| o.id).chain(s.scene.groups.iter().map(|g| g.id)).filter(|&n| n != root && s.scene.parent_of(n) == Some(root)).collect();
        for n in direct {
            s.scene.set_parent(n, up).map_err(bad)?;
        }
    }
    s.scene.groups.retain(|g| !drop_groups.contains(&g.id));
    let mut edits = vec![Edit::Hierarchy { label: "Hierarchy".into(), state: before }];
    let mut names = Vec::new();
    for id in &objects {
        let index = s.scene.index_of(*id).expect("listed above");
        let removed = s.scene.remove(*id).expect("listed above");
        names.push(removed.name.clone());
        edits.push(Edit::Delete { id: *id, index, parked: Some(removed) });
    }
    let label = if children { "Delete With Children" } else if objects.is_empty() { "Delete Group" } else { "Delete Object" };
    s.history.push(Edit::Many { label: label.into(), edits });
    Ok(json!({"removed": objects, "removed_names": names, "removed_groups": drop_groups, "active": s.scene.active, "groups": groups_report(s)}))
}

pub(crate) fn delete_group_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "A group id from get_scene."},
            "children": {"type": "boolean", "default": false, "description": "Also delete every object and group below it; otherwise they move up to the group's parent."}
        },
        "required": ["id"],
        "additionalProperties": false
    })
}

pub(crate) fn delete_group(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = args.get("id").and_then(Value::as_u64).ok_or_else(|| bad("id is required"))? as ObjectId;
    if s.scene.group(id).is_none() {
        return Err(CommandError::new("not_found", format!("no group {id}; get_scene lists groups (delete_object removes objects)")));
    }
    remove_node(s, id, args.get("children").and_then(Value::as_bool).unwrap_or(false))
}
