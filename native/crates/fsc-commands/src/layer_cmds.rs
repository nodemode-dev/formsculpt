use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn layer_guard(s: &Session, id: u32, label: &str) -> Result<(), CommandError> {
    let object = s.scene.get(id).expect("checked");
    if !object.layers.list.is_empty() {
        return Err(CommandError::new(
            "has_layers",
            format!("{label} would change the vertex count under the object's {} sculpt layers; layer_apply or layer_delete them first", object.layers.list.len()),
        ));
    }
    Ok(())
}

fn remember(s: &mut Session, id: u32, label: &str) {
    let object = s.scene.get(id).expect("checked");
    s.history.push(Edit::LayerState { label: label.into(), object: id, layers: object.layers.clone(), positions: object.mesh.positions.clone() });
}

fn index(s: &Session, id: u32, args: &Value) -> Result<usize, CommandError> {
    let i = args.get("index").and_then(Value::as_u64).ok_or_else(|| bad("index is required"))? as usize;
    let n = s.scene.get(id).expect("checked").layers.list.len();
    if i >= n {
        return Err(bad(format!("no layer {i}; the object has {n}")));
    }
    Ok(i)
}

pub(crate) fn index_schema(extra: Value) -> Value {
    let mut props = json!({
        "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
        "index": {"type": "integer", "minimum": 0, "description": "Layer position, 0 first; get_scene lists each object's layers."}
    });
    if let (Some(p), Some(e)) = (props.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            p.insert(k.clone(), v.clone());
        }
    }
    json!({"type": "object", "properties": props, "required": ["index"], "additionalProperties": false})
}

pub(crate) fn add_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "name": {"type": "string", "minLength": 1, "maxLength": 80}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn add(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let n = s.scene.get(id).expect("checked").layers.list.len();
    let name = args.get("name").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("Layer {}", n + 1));
    remember(s, id, "Add Layer");
    let object = s.scene.get_mut(id).expect("checked");
    let at = object.add_layer(name);
    Ok(json!({"object": summary(object), "index": at}))
}

pub(crate) fn select_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "index": {"type": ["integer", "null"], "minimum": 0, "description": "The layer strokes record into; null records into the base mesh."}
        },
        "required": ["index"],
        "additionalProperties": false
    })
}

pub(crate) fn select(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let pick = match args.get("index") {
        Some(Value::Null) | None => None,
        Some(_) => Some(index(s, id, args)?),
    };
    remember(s, id, "Select Layer");
    let object = s.scene.get_mut(id).expect("checked");
    object.layers.active = pick;
    object.revision += 1;
    Ok(summary(object))
}

pub(crate) fn strength_schema() -> Value {
    index_schema(json!({"strength": {"type": "number", "minimum": -1, "maximum": 2, "description": "0 hides the layer, 1 is as sculpted, negative inverts it."}}))
}

pub(crate) fn strength(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let at = index(s, id, args)?;
    let value = args.get("strength").and_then(Value::as_f64).ok_or_else(|| bad("strength is required"))? as f32;
    if !(-1.0..=2.0).contains(&value) {
        return Err(bad("strength must be -1 to 2"));
    }
    remember(s, id, "Layer Strength");
    let object = s.scene.get_mut(id).expect("checked");
    object.set_layer_strength(at, value).map_err(bad)?;
    Ok(summary(object))
}

pub(crate) fn remove(s: &mut Session, args: &Value, keep: bool) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let at = index(s, id, args)?;
    remember(s, id, if keep { "Apply Layer" } else { "Delete Layer" });
    let object = s.scene.get_mut(id).expect("checked");
    object.remove_layer(at, keep).map_err(bad)?;
    Ok(summary(object))
}

pub(crate) fn delete(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    remove(s, args, false)
}

pub(crate) fn apply(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    remove(s, args, true)
}

pub(crate) fn index_only_schema() -> Value {
    index_schema(json!({}))
}
