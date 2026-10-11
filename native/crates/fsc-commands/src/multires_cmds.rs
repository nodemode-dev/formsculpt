use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

const MAX_TRIANGLES: usize = 8_000_000;

fn failed(msg: String) -> CommandError {
    CommandError::new("bad_level", msg)
}

pub(crate) fn guard(s: &Session, id: u32, label: &str) -> Result<(), CommandError> {
    let object = s.scene.get(id).expect("checked");
    if object.multires.is_some() {
        let (_, n) = object.levels();
        return Err(CommandError::new(
            "has_levels",
            format!("{label} would discard the object's {n} subdivision levels; run multires_delete_lower and multires_delete_higher to keep one level first"),
        ));
    }
    Ok(())
}

pub(crate) fn divide_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "count": {"type": "integer", "minimum": 1, "maximum": 4, "default": 1, "description": "Levels to add; each multiplies the triangles by four."},
            "smooth": {"type": "boolean", "default": true, "description": "Loop subdivision (smooths the shape); false splits flat."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn divide(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let count = args.get("count").and_then(Value::as_u64).unwrap_or(1);
    if !(1..=4).contains(&count) {
        return Err(CommandError::new("bad_arguments", "count must be 1 to 4"));
    }
    let smooth = args.get("smooth").and_then(Value::as_bool).unwrap_or(true);
    let object = s.scene.get_mut(id).expect("checked");
    let (cur, n) = object.levels();
    if cur + 1 != n {
        return Err(failed(format!("step up to the highest level ({}) before dividing", n - 1)));
    }
    let projected = object.mesh.triangle_count() * 4usize.pow(count as u32);
    if projected > MAX_TRIANGLES {
        return Err(CommandError::new("bad_arguments", format!("that would make {projected} triangles; the limit is {MAX_TRIANGLES}")));
    }
    for _ in 0..count {
        let out = fsc_topo::loop_subdivide(&object.mesh, smooth);
        object.divide(out.mesh, out.stencil).map_err(failed)?;
    }
    let after = summary(object);
    s.history.push(Edit::Divide { object: id, count: count as usize, parked: Vec::new() });
    Ok(json!({"object": after}))
}

pub(crate) fn level_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "level": {"type": "integer", "minimum": 0, "description": "The level to sculpt on, 0 being the lowest; get_scene reports level and levels."},
            "step": {"type": "string", "enum": ["up", "down"], "description": "One level higher or lower instead of a level number."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn set_level(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get_mut(id).expect("checked");
    let (cur, n) = object.levels();
    if n == 1 {
        return Err(failed("the object has no subdivision levels; multires_divide adds them".into()));
    }
    let to = match (args.get("level").and_then(Value::as_u64), args.get("step").and_then(Value::as_str)) {
        (Some(l), _) => l as usize,
        (None, Some("up")) => (cur + 1).min(n - 1),
        (None, Some("down")) => cur.saturating_sub(1),
        _ => return Err(CommandError::new("bad_arguments", "give level or step")),
    };
    let from = object.set_level(to).map_err(failed)?;
    let after = summary(object);
    if from != to {
        s.history.push(Edit::Level { object: id, from, to });
    }
    Ok(json!({"object": after, "from": from}))
}

fn prune(s: &mut Session, args: &Value, higher: bool) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get_mut(id).expect("checked");
    let before = object.multires.clone();
    let removed = if higher { object.delete_higher() } else { object.delete_lower() }.map_err(failed)?;
    let after = summary(object);
    if removed > 0 {
        let label = if higher { "Delete Higher Levels" } else { "Delete Lower Levels" };
        s.history.push(Edit::Levels { label: label.into(), object: id, multires: before });
    }
    Ok(json!({"object": after, "levels_removed": removed}))
}

pub(crate) fn delete_higher(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    prune(s, args, true)
}

pub(crate) fn delete_lower(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    prune(s, args, false)
}
