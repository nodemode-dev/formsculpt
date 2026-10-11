use fsc_core::ObjectId;
use fsc_sculpt::{BrushKind, BrushSettings, Falloff, Stroke, StrokeEdit, Symmetry, VertexEdit};
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn f(args: &Value, key: &str, default: f32, lo: f32, hi: f32) -> Result<f32, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let x = v.as_f64().ok_or_else(|| bad(format!("{key} must be a number")))? as f32;
            if !(lo..=hi).contains(&x) {
                return Err(bad(format!("{key} must be between {lo} and {hi}")));
            }
            Ok(x)
        }
    }
}

pub fn parse_brush(args: &Value) -> Result<BrushSettings, CommandError> {
    let kind_name = args.get("brush").and_then(Value::as_str).unwrap_or("draw");
    let kind = BrushKind::parse(kind_name).ok_or_else(|| bad(format!("unknown brush {kind_name}; use one of {}", BrushKind::NAMES.join(", "))))?;
    let falloff_name = args.get("falloff").and_then(Value::as_str).unwrap_or("smooth");
    let falloff = Falloff::parse(falloff_name).ok_or_else(|| bad(format!("unknown falloff {falloff_name}; use one of {}", Falloff::NAMES.join(", "))))?;
    let sym = args.get("symmetry").cloned().unwrap_or(Value::Null);
    let flag = |k: &str| sym.get(k).and_then(Value::as_bool).unwrap_or(false);
    let radial = sym.get("radial").and_then(Value::as_u64).unwrap_or(1).clamp(1, 150) as u32;
    let size = match args.get("dyntopo_size") {
        None | Some(Value::Null) => None,
        Some(_) => Some(f(args, "dyntopo_size", 0.01, 1e-5, 1e4)?),
    };
    let dyntopo = match args.get("dyntopo") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => None,
        Some(Value::Bool(true)) => Some(0.25),
        Some(_) => Some(f(args, "dyntopo", 0.25, 0.05, 1.0)?),
    };
    Ok(BrushSettings {
        kind,
        radius: f(args, "radius", 0.2, 1e-5, 1e4)?,
        strength: f(args, "strength", 0.5, 0.0, 1.0)?,
        invert: args.get("invert").and_then(Value::as_bool).unwrap_or(false),
        falloff,
        spacing: f(args, "spacing", 0.15, 0.02, 2.0)?,
        lazy: f(args, "lazy", 0.0, 0.0, 0.95)?,
        symmetry: Symmetry { x: flag("x"), y: flag("y"), z: flag("z"), radial },
        dyntopo: dyntopo.or(size.map(|_| 0.25)),
        dyntopo_size: size,
    })
}

pub(crate) fn brush_props() -> Value {
    json!({
        "brush": {"type": "string", "enum": BrushKind::NAMES, "default": "draw"},
        "radius": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "default": 0.2, "description": "World units; compare with the object's bounds from get_scene."},
        "strength": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.5},
        "invert": {"type": "boolean", "default": false, "description": "Carve instead of build, deflate, unpinch, erase mask."},
        "falloff": {"type": "string", "enum": Falloff::NAMES, "default": "smooth"},
        "spacing": {"type": "number", "minimum": 0.02, "maximum": 2, "default": 0.15, "description": "Distance between dabs as a fraction of the radius."},
        "lazy": {"type": "number", "minimum": 0, "maximum": 0.95, "default": 0},
        "symmetry": {"type": "object", "properties": {"x": {"type": "boolean"}, "y": {"type": "boolean"}, "z": {"type": "boolean"}, "radial": {"type": "integer", "minimum": 1, "maximum": 150}}, "additionalProperties": false},
        "dyntopo": {"type": ["number", "boolean"], "description": "Dynamic topology: split edges longer than this fraction of the radius (0.05 to 1; true means 0.25) under the brush as it moves, adding detail where you sculpt, and collapse edges under 0.4 of that length. Off when omitted. Not used by grab or mask, nor on objects with subdivision levels."},
        "dyntopo_size": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "description": "Constant detail: the target edge length in world units, whatever the radius. Turns dynamic topology on and overrides the dyntopo fraction."}
    })
}

pub(crate) fn stroke_schema() -> Value {
    let mut props = brush_props();
    let p = props.as_object_mut().expect("object");
    p.insert("id".into(), json!({"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."}));
    p.insert("points".into(), json!({"type": "array", "minItems": 1, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 4}, "description": "The stroke path as [x, y, z] or [x, y, z, pressure] points on or near the surface, in object space. Grab moves the vertices around the first point by the offset to the last."}));
    json!({"type": "object", "properties": props, "required": ["points"], "additionalProperties": false})
}

pub(crate) fn target(s: &Session, args: &Value) -> Result<ObjectId, CommandError> {
    let id = match args.get("id").and_then(Value::as_u64) {
        Some(id) => id as ObjectId,
        None => s.scene.active.ok_or_else(|| CommandError::new("no_object", "the scene is empty; add_primitive first"))?,
    };
    if s.scene.get(id).is_none() {
        return Err(CommandError::new("not_found", format!("no object {id}; get_scene lists object ids")));
    }
    Ok(id)
}

pub(crate) fn sculpt_stroke(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let settings = parse_brush(args)?;
    let points = args.get("points").and_then(Value::as_array).ok_or_else(|| bad("points is required"))?;
    let mut parsed = Vec::with_capacity(points.len());
    for p in points {
        let a = p.as_array().ok_or_else(|| bad("each point is [x, y, z] or [x, y, z, pressure]"))?;
        let n = |i: usize| a.get(i).and_then(Value::as_f64).map(|x| x as f32);
        let (Some(x), Some(y), Some(z)) = (n(0), n(1), n(2)) else { return Err(bad("each point needs three numbers")) };
        parsed.push((Vec3::new(x, y, z), n(3).unwrap_or(1.0).clamp(0.0, 1.0)));
    }
    let object = s.scene.get_mut(id).expect("checked");
    if settings.dyntopo.is_some() && object.multires.is_some() {
        return Err(CommandError::new("has_levels", "dynamic topology cannot run on an object with subdivision levels; sculpt without dyntopo, or multires_delete_lower and multires_delete_higher first"));
    }
    let mut stroke = Stroke::new(settings);
    if settings.kind == BrushKind::Grab {
        let (first, _) = parsed[0];
        let (last, pressure) = *parsed.last().expect("non-empty");
        stroke.grab_between(object, first, last, pressure);
    } else {
        for (p, pressure) in parsed {
            stroke.sample_point(object, p, pressure);
        }
    }
    let dabs = stroke.dabs;
    let result = stroke.finish(object);
    let after = summary(object);
    let label = format!("{} Stroke", capital(BrushKind::NAMES[settings.kind as usize]));
    let moved = push_stroke(s, id, label, result);
    Ok(json!({"object": after, "dabs": dabs, "vertices_changed": moved}))
}

pub fn push_stroke(s: &mut Session, id: ObjectId, label: String, result: StrokeEdit) -> Value {
    match result {
        StrokeEdit::Vertices(edit) => {
            let n = edit.indices.len();
            s.history.push(Edit::Vertices { label, object: id, edit });
            json!(n)
        }
        StrokeEdit::Topology { mesh, mask } => {
            s.history.push(Edit::Topology { label, object: id, mesh, mask });
            json!("topology changed")
        }
        StrokeEdit::Nothing => json!(0),
    }
}

fn capital(s: &str) -> String {
    let mut c = s.to_string();
    if let Some(f) = c.get_mut(0..1) {
        f.make_ascii_uppercase();
    }
    c
}

pub(crate) fn id_opt_schema() -> Value {
    json!({"type": "object", "properties": {"id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."}}, "required": [], "additionalProperties": false})
}

fn mask_edit(s: &mut Session, args: &Value, label: &str, f: impl Fn(&fsc_core::Object, &[f32]) -> Vec<f32>) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get_mut(id).expect("checked");
    let before = object.mask.clone();
    let after = f(object, &before);
    let mut edit = VertexEdit::default();
    for (v, (&b, &a)) in before.iter().zip(after.iter()).enumerate() {
        if b != a {
            edit.indices.push(v as u32);
            edit.before.push(object.mesh.positions[v]);
            edit.after.push(object.mesh.positions[v]);
            edit.mask_before.push(b);
            edit.mask_after.push(a);
        }
    }
    object.mask = after;
    let changed = edit.indices.len();
    if changed > 0 {
        object.revision += 1;
        s.history.push(Edit::Vertices { label: label.into(), object: id, edit });
    }
    let masked = s.scene.get(id).map(|o| o.mask.iter().filter(|&&m| m > 0.5).count()).unwrap_or(0);
    Ok(json!({"id": id, "vertices_changed": changed, "masked_vertices": masked}))
}

pub(crate) fn mask_invert(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    mask_edit(s, args, "Invert Mask", |_, m| m.iter().map(|v| 1.0 - v).collect())
}

pub(crate) fn mask_clear(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    mask_edit(s, args, "Clear Mask", |_, m| vec![0.0; m.len()])
}

pub(crate) fn mask_blur(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    mask_edit(s, args, "Blur Mask", |o, m| {
        (0..m.len())
            .map(|v| {
                let nb = o.topology.neighbors_of(v as u32);
                if nb.is_empty() {
                    return m[v];
                }
                let avg = nb.iter().map(|&w| m[w as usize]).sum::<f32>() / nb.len() as f32;
                0.5 * m[v] + 0.5 * avg
            })
            .collect()
    })
}

pub(crate) fn mask_sharpen(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    mask_edit(s, args, "Sharpen Mask", |_, m| m.iter().map(|&v| ((v - 0.5) * 2.0 + 0.5).clamp(0.0, 1.0)).collect())
}

pub(crate) fn undo(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    let done = s.history.undo(&mut s.scene).map_err(|e| CommandError::new("history", e))?;
    let (u, r) = s.history.depth();
    Ok(json!({"undone": done, "undo_depth": u, "redo_depth": r}))
}

pub(crate) fn redo(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    let done = s.history.redo(&mut s.scene).map_err(|e| CommandError::new("history", e))?;
    let (u, r) = s.history.depth();
    Ok(json!({"redone": done, "undo_depth": u, "redo_depth": r}))
}
