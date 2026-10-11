use std::collections::{HashMap, HashSet};

use fsc_core::{Mesh, ObjectId};
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::{mask_edit, target};
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

const ID: &str = "Object id; the active object when omitted.";

/// Swaps in a copy of the mesh with new groups or hidden flags as one undo step (the same swap topology tools use).
fn commit(s: &mut Session, id: ObjectId, label: &str, mesh: Mesh) -> Value {
    let object = s.scene.get_mut(id).expect("checked");
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    s.history.push(Edit::Topology { label: label.into(), object: id, mesh: old_mesh, mask: old_mask });
    summary(s.scene.get(id).expect("checked"))
}

fn groups_of(mesh: &Mesh) -> Vec<u32> {
    if mesh.has_groups() {
        mesh.groups.clone()
    } else {
        vec![0; mesh.triangle_count()]
    }
}

fn masked_faces(mesh: &Mesh, mask: &[f32]) -> Vec<bool> {
    (0..mesh.triangle_count()).map(|t| mesh.triangle(t).iter().all(|&v| mask[v as usize] > 0.5)).collect()
}

fn point(args: &Value) -> Result<Option<Vec3>, CommandError> {
    match args.get("point") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() == 3 => {
            let v: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
            if v.iter().any(|x| !x.is_finite()) {
                return Err(bad("point values must be numbers"));
            }
            Ok(Some(Vec3::new(v[0], v[1], v[2])))
        }
        Some(_) => Err(bad("point is [x, y, z]")),
    }
}

/// The group named by `group`, or the group of the triangle nearest `point`.
fn chosen_group(s: &Session, id: ObjectId, args: &Value) -> Result<u32, CommandError> {
    let o = s.scene.get(id).expect("checked");
    if !o.mesh.has_groups() {
        return Err(CommandError::new("no_groups", "the object has no face groups; make some with group_from_mask or group_auto"));
    }
    if let Some(g) = args.get("group").and_then(Value::as_u64) {
        if !o.mesh.groups.contains(&(g as u32)) {
            return Err(CommandError::new("not_found", format!("the object has no face group {g}")));
        }
        return Ok(g as u32);
    }
    let p = point(args)?.ok_or_else(|| bad("give group or point"))?;
    let (_, t, _, _) = fsc_topo::nearest_feature(&o.bvh, &o.mesh, p, f32::INFINITY).ok_or_else(|| bad("no surface near the point"))?;
    Ok(o.mesh.groups[t as usize])
}

pub(crate) fn from_mask_schema() -> Value {
    json!({"type": "object", "properties": {"id": {"type": "integer", "minimum": 1, "description": ID}}, "required": [], "additionalProperties": false})
}

pub(crate) fn group_from_mask(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let o = s.scene.get(id).expect("checked");
    let sel = masked_faces(&o.mesh, &o.mask);
    let n = sel.iter().filter(|&&b| b).count();
    if n == 0 {
        return Err(CommandError::new("empty_mask", "nothing is masked; mask the area first (faces whose three corners are at least half masked count)"));
    }
    let mut mesh = o.mesh.clone();
    let mut groups = groups_of(&mesh);
    let new = groups.iter().max().copied().unwrap_or(0) + 1;
    for (t, &on) in sel.iter().enumerate() {
        if on {
            groups[t] = new;
        }
    }
    mesh.groups = groups;
    let object = commit(s, id, "Group from Mask", mesh);
    Ok(json!({"object": object, "group": new, "faces": n}))
}

pub(crate) fn auto_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": ID},
            "by": {"type": "string", "enum": ["angle", "parts"], "default": "angle", "description": "angle starts a new group wherever neighbouring faces turn by more than `angle` degrees (the planes of a hard-surface model, the sides of a cube); parts gives each loose piece its own group."},
            "angle": {"type": "number", "minimum": 1, "maximum": 179, "default": 30}
        },
        "required": [],
        "additionalProperties": false
    })
}

fn face_normal(mesh: &Mesh, t: usize) -> Vec3 {
    let [a, b, c] = mesh.triangle(t);
    let (pa, pb, pc) = (mesh.position(a), mesh.position(b), mesh.position(c));
    (pb - pa).cross(pc - pa).normalize_or_zero()
}

pub(crate) fn group_auto(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let by = args.get("by").and_then(Value::as_str).unwrap_or("angle").to_string();
    let angle = args.get("angle").and_then(Value::as_f64).unwrap_or(30.0);
    if !(1.0..=179.0).contains(&angle) {
        return Err(bad("angle must be 1 to 179"));
    }
    let cos = (angle as f32).to_radians().cos();
    let o = s.scene.get(id).expect("checked");
    let mesh = &o.mesh;
    let tris = mesh.triangle_count();
    let normals: Vec<Vec3> = (0..tris).map(|t| face_normal(mesh, t)).collect();
    // Faces sharing an edge (angle) or a vertex (parts) are neighbours.
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); tris];
    match by.as_str() {
        "angle" => {
            for faces in fsc_topo::edge_faces(mesh).values() {
                for &a in faces {
                    for &b in faces {
                        if a != b && normals[a as usize].dot(normals[b as usize]) >= cos {
                            neighbours[a as usize].push(b);
                        }
                    }
                }
            }
        }
        "parts" => {
            let mut by_vertex: HashMap<u32, Vec<u32>> = HashMap::new();
            for t in 0..tris {
                for v in mesh.triangle(t) {
                    by_vertex.entry(v).or_default().push(t as u32);
                }
            }
            for faces in by_vertex.values() {
                for w in faces.windows(2) {
                    neighbours[w[0] as usize].push(w[1]);
                    neighbours[w[1] as usize].push(w[0]);
                }
            }
        }
        other => return Err(bad(format!("unknown by {other}; use angle or parts"))),
    }
    let mut groups = vec![u32::MAX; tris];
    let mut next = 0u32;
    let mut stack = Vec::new();
    for start in 0..tris {
        if groups[start] != u32::MAX {
            continue;
        }
        groups[start] = next;
        stack.push(start as u32);
        while let Some(t) = stack.pop() {
            for &n in &neighbours[t as usize] {
                if groups[n as usize] == u32::MAX {
                    groups[n as usize] = next;
                    stack.push(n);
                }
            }
        }
        next += 1;
    }
    let mut mesh = mesh.clone();
    mesh.groups = groups;
    let label = if by == "angle" { "Group by Angle" } else { "Group by Parts" };
    let object = commit(s, id, label, mesh);
    Ok(json!({"object": object, "groups": next}))
}

pub(crate) fn clear_schema() -> Value {
    from_mask_schema()
}

pub(crate) fn group_clear(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let mut mesh = s.scene.get(id).expect("checked").mesh.clone();
    mesh.groups.clear();
    Ok(json!({"object": commit(s, id, "Clear Groups", mesh)}))
}

pub(crate) fn mask_group_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": ID},
            "group": {"type": "integer", "minimum": 0, "description": "The face group to mask."},
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Or: mask the group of the face nearest this point."},
            "op": {"type": "string", "enum": ["replace", "add", "subtract", "intersect"], "default": "replace"}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn mask_by_group(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let g = chosen_group(s, id, args)?;
    let op = args.get("op").and_then(Value::as_str).unwrap_or("replace").to_string();
    if !["replace", "add", "subtract", "intersect"].contains(&op.as_str()) {
        return Err(bad(format!("unknown op {op}")));
    }
    let mut reply = mask_edit(s, args, "Mask by Group", |o, old| {
        let mut new = vec![0.0f32; old.len()];
        for t in 0..o.mesh.triangle_count() {
            if o.mesh.groups[t] == g {
                for v in o.mesh.triangle(t) {
                    new[v as usize] = 1.0;
                }
            }
        }
        old.iter()
            .zip(&new)
            .map(|(&a, &b)| match op.as_str() {
                "add" => a.max(b),
                "subtract" => (a - b).max(0.0),
                "intersect" => a.min(b),
                _ => b,
            })
            .collect()
    })?;
    reply["group"] = json!(g);
    Ok(reply)
}

pub(crate) fn hide_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": ID},
            "action": {"type": "string", "enum": ["hide", "show", "isolate", "invert", "show_all"], "description": "hide or show the chosen faces, isolate them (hide the rest), invert what is hidden, or show every face."},
            "by": {"type": "string", "enum": ["group", "mask"], "default": "group", "description": "Choose faces by face group (group or point) or by the mask (faces whose corners are at least half masked)."},
            "group": {"type": "integer", "minimum": 0},
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Choose the group of the face nearest this point."}
        },
        "required": ["action"],
        "additionalProperties": false
    })
}

pub(crate) fn hide_faces(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let action = args.get("action").and_then(Value::as_str).ok_or_else(|| bad("action is required"))?.to_string();
    let needs_choice = matches!(action.as_str(), "hide" | "show" | "isolate");
    let chosen: Vec<bool> = if needs_choice {
        match args.get("by").and_then(Value::as_str).unwrap_or("group") {
            "group" => {
                let g = chosen_group(s, id, args)?;
                s.scene.get(id).expect("checked").mesh.groups.iter().map(|&x| x == g).collect()
            }
            "mask" => {
                let o = s.scene.get(id).expect("checked");
                let sel = masked_faces(&o.mesh, &o.mask);
                if !sel.iter().any(|&b| b) {
                    return Err(CommandError::new("empty_mask", "nothing is masked; mask the area first"));
                }
                sel
            }
            other => return Err(bad(format!("unknown by {other}; use group or mask"))),
        }
    } else {
        Vec::new()
    };
    let o = s.scene.get(id).expect("checked");
    let tris = o.mesh.triangle_count();
    let current: Vec<bool> = (0..tris).map(|t| o.mesh.face_hidden(t)).collect();
    let next: Vec<bool> = match action.as_str() {
        "hide" => current.iter().zip(&chosen).map(|(&h, &c)| h || c).collect(),
        "show" => current.iter().zip(&chosen).map(|(&h, &c)| h && !c).collect(),
        "isolate" => chosen.iter().map(|&c| !c).collect(),
        "invert" => current.iter().map(|&h| !h).collect(),
        "show_all" => vec![false; tris],
        other => return Err(bad(format!("unknown action {other}"))),
    };
    if next.iter().all(|&h| h) {
        return Err(CommandError::new("all_hidden", "that would hide every face; show some first, or use set_visibility to hide the whole object"));
    }
    let hidden = next.iter().filter(|&&h| h).count();
    let mut mesh = o.mesh.clone();
    mesh.hidden = if hidden == 0 { Vec::new() } else { next };
    let label = match action.as_str() {
        "hide" => "Hide Faces",
        "show" => "Show Faces",
        "isolate" => "Isolate Faces",
        "invert" => "Invert Hidden",
        _ => "Show All Faces",
    };
    let object = commit(s, id, label, mesh);
    Ok(json!({"object": object, "hidden_faces": hidden}))
}

pub(crate) fn group_count(mesh: &Mesh) -> usize {
    if mesh.has_groups() {
        mesh.groups.iter().collect::<HashSet<_>>().len()
    } else {
        0
    }
}
