use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::{push_vertices, target};
use fsc_sculpt::VertexEdit;
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
    s.history.push(Edit::LayerState { label: label.into(), object: id, layers: object.layers.clone(), positions: object.mesh.positions.clone(), paint: object.mesh.paint.clone(), mask: object.mask.clone() });
}

fn which(s: &Session, id: u32, args: &Value) -> Result<usize, CommandError> {
    let i = args.get("layer").or_else(|| args.get("index")).and_then(Value::as_u64).ok_or_else(|| bad("layer is required"))? as usize;
    let n = s.scene.get(id).expect("checked").layers.list.len();
    if i >= n {
        return Err(bad(format!("no layer {i}; the object has {n}")));
    }
    Ok(i)
}

fn factor(args: &Value, key: &str, lo: f64, hi: f64) -> Result<Option<f32>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_f64() {
            Some(x) if (lo..=hi).contains(&x) => Ok(Some(x as f32)),
            _ => Err(bad(format!("{key} must be a number from {lo} to {hi}"))),
        },
    }
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

pub(crate) fn set_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "layer": {"type": "integer", "minimum": 0, "description": "Layer position, 0 first; get_scene lists each object's layers."},
            "strength": {"type": "number", "minimum": -1, "maximum": 2, "description": "0 hides the layer, 1 is as sculpted, negative inverts it; multiplies the three factors."},
            "shape_factor": {"type": "number", "minimum": -2, "maximum": 2, "description": "Scales the layer's shape on top of strength; 1 is as recorded, -1 inverts the sculpt."},
            "paint_factor": {"type": "number", "minimum": -2, "maximum": 2, "description": "Scales the layer's paint on top of strength; 0 gives back the base paint."},
            "mask_factor": {"type": "number", "minimum": -2, "maximum": 2, "description": "Scales the layer's mask on top of strength; 0 gives back the base mask."},
            "name": {"type": "string", "minLength": 1, "maxLength": 80}
        },
        "required": ["layer"],
        "additionalProperties": false
    })
}

pub(crate) fn set(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let at = which(s, id, args)?;
    let strength = factor(args, "strength", -1.0, 2.0)?;
    let shape = factor(args, "shape_factor", -2.0, 2.0)?;
    let paint = factor(args, "paint_factor", -2.0, 2.0)?;
    let mask = factor(args, "mask_factor", -2.0, 2.0)?;
    let name = args.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).map(str::to_string);
    if strength.is_none() && shape.is_none() && paint.is_none() && mask.is_none() && name.is_none() {
        return Err(bad("give strength, shape_factor, paint_factor, mask_factor or name"));
    }
    remember(s, id, "Set Layer");
    let object = s.scene.get_mut(id).expect("checked");
    object.set_layer(at, strength, shape, paint, mask).map_err(bad)?;
    if let Some(n) = name {
        object.layers.list[at].name = n;
        object.revision += 1;
    }
    Ok(summary(object))
}

pub(crate) fn offset_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "distance": {"type": "number", "minimum": -10000, "maximum": 10000, "description": "How far every vertex moves along its normal, in world units; negative moves inward."},
            "masked_only": {"type": "boolean", "default": false, "description": "Move only the masked vertices, each by its mask value; otherwise every vertex moves and the mask is ignored."}
        },
        "required": ["distance"],
        "additionalProperties": false
    })
}

pub(crate) fn offset(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let distance = match args.get("distance").and_then(Value::as_f64) {
        Some(d) if d.is_finite() && d.abs() <= 10000.0 => d as f32,
        _ => return Err(bad("distance is required, a number from -10000 to 10000")),
    };
    let masked_only = args.get("masked_only").and_then(Value::as_bool).unwrap_or(false);
    let object = s.scene.get_mut(id).expect("checked");
    let at = object.layers.active.filter(|&a| a < object.layers.list.len()).ok_or_else(|| CommandError::new("no_active_layer", "layer_offset records into the active sculpt layer; layer_add or layer_select one first"))?;
    if object.layers.list[at].shape_scale().abs() < 1e-4 {
        return Err(bad("the active layer's strength times shape_factor is about 0, so an offset would not show; raise one with layer_set first"));
    }
    let mut edit = VertexEdit::default();
    for v in 0..object.mesh.vertex_count() {
        let w = if masked_only { object.mask[v] } else { 1.0 };
        if w <= 0.0 {
            continue;
        }
        let n = glam::Vec3::from(object.mesh.normals[v]).normalize_or_zero();
        let p = object.mesh.positions[v];
        let q = (glam::Vec3::from(p) + n * (distance * w)).to_array();
        if q == p {
            continue;
        }
        edit.indices.push(v as u32);
        edit.before.push(p);
        edit.after.push(q);
        edit.mask_before.push(object.mask[v]);
        edit.mask_after.push(object.mask[v]);
    }
    let moved = edit.indices.len();
    if moved == 0 {
        return Ok(json!({"object": summary(object), "vertices_moved": 0}));
    }
    edit.apply(object, true);
    let after = summary(object);
    push_vertices(s, id, "Layer Offset".into(), edit);
    Ok(json!({"object": after, "vertices_moved": moved, "layer": at}))
}

pub(crate) fn bake_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "layer": {"type": "integer", "minimum": 0, "description": "Layer position, 0 first; get_scene lists each object's layers."}
        },
        "required": ["layer"],
        "additionalProperties": false
    })
}

pub(crate) fn bake(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let at = which(s, id, args)?;
    let layer = &s.scene.get(id).expect("checked").layers.list[at];
    let baked = json!({"shape": layer.shape_scale() != 0.0, "paint": layer.paint.is_some() && layer.paint_scale() != 0.0, "mask": layer.mask.is_some() && layer.mask_scale() != 0.0});
    remember(s, id, "Bake Layer");
    let object = s.scene.get_mut(id).expect("checked");
    object.remove_layer(at, true).map_err(bad)?;
    Ok(json!({"object": summary(object), "baked": baked}))
}

pub(crate) fn extract_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "layer": {"type": "integer", "minimum": 0, "description": "Layer position, 0 first; get_scene lists each object's layers."},
            "mode": {"type": "string", "enum": ["difference", "mask", "displaced_only"], "default": "difference", "description": "difference: the base mesh (all layers taken off) with only this layer applied at full strength, with its paint and mask. mask: a copy of the current mesh whose mask is the layer's mask delta (positive part). displaced_only: only the vertices this layer moves, thickened into a shell (as extract_mask does) from the current mesh."},
            "thickness": {"type": "number", "minimum": -10000, "maximum": 10000, "description": "displaced_only: shell thickness in world units; 2 percent of the object's size when omitted."}
        },
        "required": ["layer"],
        "additionalProperties": false
    })
}

pub(crate) fn extract(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let at = which(s, id, args)?;
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("difference");
    let thickness = factor(args, "thickness", -10000.0, 10000.0)?;
    let object = s.scene.get(id).expect("checked");
    let layer = &object.layers.list[at];
    let size = object.mesh.bounds().extent().length();
    let (mesh, mask) = match mode {
        "difference" => {
            let mut mesh = object.mesh.clone();
            let mut mask = object.mask.clone();
            for l in &object.layers.list {
                let k = l.shape_scale();
                for (p, d) in mesh.positions.iter_mut().zip(&l.delta) {
                    *p = (glam::Vec3::from(*p) - glam::Vec3::from(*d) * k).to_array();
                }
                if let (true, Some(pd)) = (mesh.has_paint(), &l.paint) {
                    let k = l.paint_scale();
                    for (p, d) in mesh.paint.iter_mut().zip(pd) {
                        for c in 0..p.len() {
                            p[c] = (p[c] - d[c] * k).clamp(0.0, 1.0);
                        }
                    }
                }
                if let Some(md) = &l.mask {
                    let k = l.mask_scale();
                    for (m, d) in mask.iter_mut().zip(md) {
                        *m = (*m - d * k).clamp(0.0, 1.0);
                    }
                }
            }
            for (p, d) in mesh.positions.iter_mut().zip(&layer.delta) {
                *p = (glam::Vec3::from(*p) + glam::Vec3::from(*d)).to_array();
            }
            if let Some(pd) = &layer.paint {
                mesh.ensure_paint();
                for (p, d) in mesh.paint.iter_mut().zip(pd) {
                    for c in 0..p.len() {
                        p[c] = (p[c] + d[c]).clamp(0.0, 1.0);
                    }
                }
            }
            if let Some(md) = &layer.mask {
                for (m, d) in mask.iter_mut().zip(md) {
                    *m = (*m + d).clamp(0.0, 1.0);
                }
            }
            mesh.recompute_normals();
            (mesh, Some(mask))
        }
        "mask" => {
            let md = layer.mask.as_ref().ok_or_else(|| CommandError::new("no_layer_mask", "the layer has no mask delta; paint the mask with a mask stroke while the layer is active"))?;
            let mask: Vec<f32> = md.iter().map(|d| d.clamp(0.0, 1.0)).collect();
            if !mask.iter().any(|&m| m > 0.0) {
                return Err(CommandError::new("no_layer_mask", "the layer's mask delta masks nothing (it only lowers the mask)"));
            }
            (object.mesh.clone(), Some(mask))
        }
        "displaced_only" => {
            let eps = (size * 1e-6).max(f32::MIN_POSITIVE);
            let moved: Vec<f32> = layer.delta.iter().map(|d| if glam::Vec3::from(*d).length() > eps { 1.0 } else { 0.0 }).collect();
            if !moved.iter().any(|&m| m > 0.0) {
                return Err(CommandError::new("no_displacement", "the layer moves no vertex"));
            }
            let thickness = thickness.unwrap_or(size * 0.02);
            let mesh = fsc_topo::extract_shell(&object.mesh, &moved, thickness).ok_or_else(|| CommandError::new("no_displacement", "the vertices the layer moves bound no face"))?;
            (mesh, None)
        }
        other => return Err(bad(format!("unknown mode {other}; use difference, mask or displaced_only"))),
    };
    let name = s.scene.unique_name(&format!("{} {}", layer.name, mode.replace('_', " ")));
    let new_id = s.scene.add(name, mesh);
    if let Some(mask) = mask {
        s.scene.get_mut(new_id).expect("added").mask = mask;
    }
    let index = s.scene.index_of(new_id).expect("added");
    s.history.push(Edit::Add { id: new_id, index, parked: None });
    Ok(summary(s.scene.get(new_id).expect("added")))
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
