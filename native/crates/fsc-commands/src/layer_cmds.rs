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

pub(crate) fn noise_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "scale": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "description": "Feature size in world units; 2 percent of the object's size when omitted."},
            "amount": {"type": "number", "minimum": -10000, "maximum": 10000, "description": "Height of the bumps along the normals in world units (negative sinks them); 0.4 percent of the object's size when omitted."},
            "seed": {"type": "integer", "minimum": 0, "maximum": 1000000, "default": 0, "description": "Another seed gives another pattern."},
            "pattern": {"type": "string", "enum": fsc_sculpt::NoisePattern::NAMES, "default": "value", "description": "value: soft fractal bumps (skin, stone); ridged: sharp crests (rock, bark, veins); turbulence: creased troughs (wrinkles, erosion); cells: rounded cells (scales, hammered metal, craters with a negative amount); alpha: an alpha tiled over the surface from three sides (fabric, tiles, a pattern loaded with alpha_load)."},
            "octaves": {"type": "integer", "minimum": 1, "maximum": 8, "default": 3, "description": "Layers of finer detail on top of the base size."},
            "gain": {"type": "number", "minimum": 0.1, "maximum": 0.9, "default": 0.5, "description": "How strongly each finer layer shows."},
            "stretch": {"type": "array", "items": {"type": "number", "minimum": 0.05, "maximum": 20}, "minItems": 3, "maxItems": 3, "description": "Feature size multipliers along x, y and z (stretch wood grain or strata along one axis)."},
            "alpha": {"type": "string", "description": "The alpha pattern's alpha: a built-in name or one loaded with alpha_load."},
            "name": {"type": "string", "minLength": 1, "maxLength": 80, "default": "Noise"}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Fractal noise over the whole object along its normals, held by the mask, written into a new sculpt layer so its
/// strength stays adjustable (0 hides it, negative inverts it) until layer_apply bakes it in.
pub(crate) fn surface_noise(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get(id).expect("checked");
    let size = object.mesh.bounds().extent().length();
    let scale = args.get("scale").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(size * 0.02);
    let amount = args.get("amount").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(size * 0.004);
    if !(scale > 0.0 && scale.is_finite() && amount.is_finite()) {
        return Err(bad("scale must be above 0 and amount a number"));
    }
    let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(0) as f32;
    let offset = glam::Vec3::new(seed * 12.9898, seed * 78.233, seed * 37.719);
    let name = args.get("name").and_then(Value::as_str).unwrap_or("Noise").to_string();
    // The noise editor: a pattern, its octaves and gain, a stretch per axis, and for the alpha pattern the alpha.
    let pattern_name = args.get("pattern").and_then(Value::as_str).unwrap_or("value");
    let pattern = fsc_sculpt::NoisePattern::parse(pattern_name).ok_or_else(|| bad(format!("pattern must be one of {}", fsc_sculpt::NoisePattern::NAMES.join(", "))))?;
    let octaves = args.get("octaves").and_then(Value::as_u64).unwrap_or(3);
    if !(1..=8).contains(&octaves) {
        return Err(bad("octaves must be from 1 to 8"));
    }
    let gain = args.get("gain").and_then(Value::as_f64).unwrap_or(0.5) as f32;
    if !(0.1..=0.9).contains(&gain) {
        return Err(bad("gain must be from 0.1 to 0.9"));
    }
    let stretch = match args.get("stretch").and_then(Value::as_array) {
        None => glam::Vec3::ONE,
        Some(a) if a.len() == 3 && a.iter().all(|x| x.as_f64().is_some_and(|v| (0.05..=20.0).contains(&v))) => glam::Vec3::new(a[0].as_f64().unwrap() as f32, a[1].as_f64().unwrap() as f32, a[2].as_f64().unwrap() as f32),
        Some(_) => return Err(bad("stretch is [x, y, z], each from 0.05 to 20")),
    };
    let alpha = if pattern == fsc_sculpt::NoisePattern::Alpha {
        let a = crate::stroke_alpha(s, args)?.ok_or_else(|| bad("the alpha pattern needs alpha: a built-in name or one loaded with alpha_load"))?;
        Some(a)
    } else {
        None
    };
    let settings = fsc_sculpt::NoiseSettings { pattern, octaves: octaves as u32, gain };
    remember(s, id, "Surface Noise");
    let object = s.scene.get_mut(id).expect("checked");
    let at = object.add_layer(name);
    let delta: Vec<[f32; 3]> = (0..object.mesh.vertex_count())
        .map(|v| {
            let p = glam::Vec3::from(object.mesh.positions[v]);
            let n = glam::Vec3::from(object.mesh.normals[v]);
            let value = fsc_sculpt::noise_at(p / (stretch * scale) + offset, n, &settings, alpha.as_deref());
            (n * value * amount * (1.0 - object.mask[v])).to_array()
        })
        .collect();
    object.layers.list[at].delta = delta.clone();
    object.shift(&delta, 1.0);
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
