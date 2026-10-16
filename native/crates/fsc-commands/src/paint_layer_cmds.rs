//! Paint layer commands: add, change (name, blend, opacity, visibility, order), select, delete, merge down and
//! flatten. Each is one undo step that swaps the object's whole paint stack and composite.

use fsc_core::paint_layers::{ChannelBlend, CHANNEL_NAMES};
use fsc_core::{Blend, ObjectId, PaintLayer, COVER, PAINT_LEN};
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

/// Runs `change` on the object's paint stack, recomposites, and records one undo step labelled `label`.
fn step(s: &mut Session, id: ObjectId, label: &str, change: impl FnOnce(&mut fsc_core::Object) -> Result<(), CommandError>) -> Result<Value, CommandError> {
    let object = s.scene.get_mut(id).ok_or_else(|| CommandError::new("not_found", format!("no object {id}")))?;
    object.check_paint_layers();
    let layers = object.paint_layers.clone();
    let paint = object.mesh.paint.clone();
    change(object)?;
    object.composite_paint(None);
    object.revision += 1;
    let after = summary(object);
    s.history.push(Edit::PaintLayers { label: label.into(), object: id, layers, paint });
    Ok(json!({"object": after}))
}

fn index(object: &fsc_core::Object, args: &Value) -> Result<usize, CommandError> {
    let n = object.paint_layers.list.len();
    if n == 0 {
        return Err(CommandError::new("no_layer", "the object has no paint layers; paint_layer_add first"));
    }
    match args.get("index") {
        None | Some(Value::Null) => object.paint_layers.active.ok_or_else(|| bad("give index (the base is not a layer)")),
        Some(v) => {
            let i = v.as_u64().ok_or_else(|| bad("index is a layer number from 0"))? as usize;
            if i >= n {
                return Err(bad(format!("index must be below {n}")));
            }
            Ok(i)
        }
    }
}

fn blend(args: &Value) -> Result<Option<Blend>, CommandError> {
    match args.get("blend").and_then(Value::as_str) {
        None => Ok(None),
        Some(name) => Blend::parse(name).map(Some).ok_or_else(|| bad(format!("blend must be one of {}", Blend::NAMES.join(", ")))),
    }
}

/// The `channel_blend` argument: None when absent, otherwise the whole new override set (null or {} clears it).
fn channel_blend(args: &Value) -> Result<Option<ChannelBlend>, CommandError> {
    let names = || CHANNEL_NAMES.join(", ");
    match args.get("channel_blend") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(ChannelBlend::default())),
        Some(Value::Object(map)) => {
            let mut out = ChannelBlend::default();
            for (key, value) in map {
                let c = CHANNEL_NAMES.iter().position(|n| n == key).ok_or_else(|| bad(format!("channel_blend keys must be among {}", names())))?;
                out[c] = match value {
                    Value::Null => None,
                    Value::String(name) => Some(Blend::parse(name).ok_or_else(|| bad(format!("blend must be one of {}", Blend::NAMES.join(", "))))?),
                    _ => return Err(bad("channel_blend values are blend names")),
                };
            }
            Ok(Some(out))
        }
        Some(_) => Err(bad("channel_blend is an object from channel name to blend, or null to clear")),
    }
}

fn opacity(args: &Value) -> Result<Option<f32>, CommandError> {
    match args.get("opacity") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => {
            let x = v.as_f64().ok_or_else(|| bad("opacity is a number from 0 to 1"))?;
            if !(0.0..=1.0).contains(&x) {
                return Err(bad("opacity must be between 0 and 1"));
            }
            Ok(Some(x as f32))
        }
    }
}

pub(crate) fn add(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let blend = blend(args)?.unwrap_or(Blend::Normal);
    let channel_blend = channel_blend(args)?.unwrap_or_default();
    let opacity = opacity(args)?.unwrap_or(1.0);
    let name = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).map(str::to_string);
    step(s, id, "Add Paint Layer", |o| {
        let vc = o.mesh.vertex_count();
        if o.paint_layers.is_empty() {
            // The first layer: the paint so far becomes the base under it.
            o.mesh.ensure_paint();
            o.paint_layers.base = o.mesh.paint.clone();
        }
        let n = o.paint_layers.list.len();
        let name = name.unwrap_or_else(|| format!("Paint {}", n + 1));
        // New layers go above the active one (on top when the base is active).
        let at = o.paint_layers.active.map_or(n, |a| a + 1);
        o.paint_layers.list.insert(at, PaintLayer { name, blend, channel_blend, opacity, visible: true, data: vec![[0.0; PAINT_LEN + 1]; vc] });
        o.paint_layers.active = Some(at);
        Ok(())
    })
}

pub(crate) fn set(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get(id).ok_or_else(|| CommandError::new("not_found", format!("no object {id}")))?;
    let i = index(object, args)?;
    let blend = blend(args)?;
    let channel_blend = channel_blend(args)?;
    let opacity = opacity(args)?;
    let name = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).map(str::to_string);
    let visible = args.get("visible").and_then(Value::as_bool);
    let shift: i64 = match args.get("move").and_then(Value::as_str) {
        None => 0,
        Some("up") => 1,
        Some("down") => -1,
        Some(other) => return Err(bad(format!("move must be up or down, not {other}"))),
    };
    let n = object.paint_layers.list.len() as i64;
    let to = (i as i64 + shift).clamp(0, n - 1) as usize;
    step(s, id, "Paint Layer Settings", |o| {
        let layer = &mut o.paint_layers.list[i];
        if let Some(b) = blend {
            layer.blend = b;
        }
        if let Some(cb) = channel_blend {
            layer.channel_blend = cb;
        }
        if let Some(x) = opacity {
            layer.opacity = x;
        }
        if let Some(v) = visible {
            layer.visible = v;
        }
        if let Some(name) = name {
            layer.name = name;
        }
        if to != i {
            o.paint_layers.list.swap(i, to);
            o.paint_layers.active = o.paint_layers.active.map(|a| if a == i { to } else if a == to { i } else { a });
        }
        Ok(())
    })
}

pub(crate) fn select(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get_mut(id).ok_or_else(|| CommandError::new("not_found", format!("no object {id}")))?;
    object.check_paint_layers();
    let pick = match args.get("index") {
        None | Some(Value::Null) => None,
        Some(_) => Some(index(object, args)?),
    };
    if object.paint_layers.is_empty() {
        return Err(CommandError::new("no_layer", "the object has no paint layers; paint_layer_add first"));
    }
    // Choosing what to paint into is not an undo step.
    object.paint_layers.active = pick;
    Ok(json!({"object": summary(object)}))
}

pub(crate) fn delete(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get(id).ok_or_else(|| CommandError::new("not_found", format!("no object {id}")))?;
    let i = index(object, args)?;
    step(s, id, "Delete Paint Layer", |o| {
        o.paint_layers.list.remove(i);
        o.paint_layers.active = match o.paint_layers.active {
            Some(a) if a == i => i.checked_sub(1).or(if o.paint_layers.list.is_empty() { None } else { Some(0) }),
            Some(a) if a > i => Some(a - 1),
            other => other,
        };
        if o.paint_layers.list.is_empty() {
            // The last layer gone: the base is the paint again.
            o.mesh.paint = std::mem::take(&mut o.paint_layers.base);
            o.paint_layers = Default::default();
        }
        Ok(())
    })
}

/// Bakes a layer into what lies below it (the layer under it, or the base).
pub(crate) fn merge(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get(id).ok_or_else(|| CommandError::new("not_found", format!("no object {id}")))?;
    let i = index(object, args)?;
    step(s, id, "Merge Paint Layer", |o| {
        let vc = o.mesh.vertex_count();
        if i == 0 {
            // Into the base: the base becomes the composite of itself and this layer alone.
            let base: Vec<_> = (0..vc).map(|v| o.paint_layers.below(v, Some(1))).collect();
            o.paint_layers.base = base;
        } else {
            // Into the layer below: that layer takes the result of both over what lies under them, at full coverage
            // where either covered.
            let under = i - 1;
            for v in 0..vc {
                let a_below = o.paint_layers.list[under].data[v][COVER] * o.paint_layers.list[under].opacity;
                let a_this = o.paint_layers.list[i].data[v][COVER] * o.paint_layers.list[i].opacity;
                if a_below <= 0.0 && a_this <= 0.0 {
                    continue;
                }
                let start = o.paint_layers.below(v, Some(under));
                let end = o.paint_layers.below(v, Some(i + 1));
                let cover = 1.0 - (1.0 - a_below.min(1.0)) * (1.0 - a_this.min(1.0));
                // Coverage `cover` over `start` must give `end`: solve for the layer's value.
                let mut d = [0.0; PAINT_LEN + 1];
                for c in 0..PAINT_LEN {
                    d[c] = if cover > 1e-4 { (start[c] + (end[c] - start[c]) / cover).clamp(0.0, 1.0) } else { end[c] };
                }
                d[COVER] = cover;
                o.paint_layers.list[under].data[v] = d;
            }
            let l = &mut o.paint_layers.list[under];
            l.blend = Blend::Normal;
            l.channel_blend = ChannelBlend::default();
            l.opacity = 1.0;
        }
        o.paint_layers.list.remove(i);
        o.paint_layers.active = (i > 0 && !o.paint_layers.list.is_empty()).then(|| i - 1);
        if o.paint_layers.list.is_empty() {
            o.mesh.paint = std::mem::take(&mut o.paint_layers.base);
            o.paint_layers = Default::default();
        }
        Ok(())
    })
}

/// Bakes every visible layer into plain paint and removes the stack.
pub(crate) fn flatten(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get(id).ok_or_else(|| CommandError::new("not_found", format!("no object {id}")))?;
    if object.paint_layers.is_empty() {
        return Err(CommandError::new("no_layer", "the object has no paint layers to flatten"));
    }
    step(s, id, "Flatten Paint Layers", |o| {
        o.composite_paint(None);
        o.paint_layers = Default::default();
        Ok(())
    })
}

fn channel_blend_schema() -> Value {
    let mode = json!({"type": "string", "enum": Blend::NAMES});
    let mut props = serde_json::Map::new();
    for name in CHANNEL_NAMES {
        props.insert(name.to_string(), mode.clone());
    }
    json!({"type": ["object", "null"], "properties": props, "additionalProperties": false, "description": "Blend per paint channel (color, roughness, metalness, opacity, emissive), replacing the layer's earlier overrides. A named channel uses its mode; color falls back to blend and the rest to normal. null or {} clears the overrides."})
}

pub(crate) fn add_schema() -> Value {
    json!({"type": "object", "properties": {
        "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
        "name": {"type": "string", "maxLength": 64},
        "blend": {"type": "string", "enum": Blend::NAMES, "default": "normal", "description": "How the layer mixes with what lies below. hue, saturation, color and luminosity work on the colour only; on roughness, metalness, opacity and emissive they act as normal."},
        "channel_blend": channel_blend_schema(),
        "opacity": {"type": "number", "minimum": 0, "maximum": 1, "default": 1}
    }, "additionalProperties": false})
}

pub(crate) fn set_schema() -> Value {
    json!({"type": "object", "properties": {
        "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
        "index": {"type": "integer", "minimum": 0, "description": "Layer number from the bottom (0); the active layer when omitted."},
        "name": {"type": "string", "maxLength": 64},
        "blend": {"type": "string", "enum": Blend::NAMES, "description": "How the layer mixes with what lies below (colour; the other channels mix normally unless channel_blend names them). hue, saturation, color and luminosity work on the colour only."},
        "channel_blend": channel_blend_schema(),
        "opacity": {"type": "number", "minimum": 0, "maximum": 1},
        "visible": {"type": "boolean"},
        "move": {"type": "string", "enum": ["up", "down"], "description": "Move the layer one place up or down the stack."}
    }, "additionalProperties": false})
}

pub(crate) fn index_schema() -> Value {
    json!({"type": "object", "properties": {
        "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
        "index": {"type": ["integer", "null"], "minimum": 0, "description": "Layer number from the bottom (0); for paint_layer_select, null paints the base."}
    }, "additionalProperties": false})
}
