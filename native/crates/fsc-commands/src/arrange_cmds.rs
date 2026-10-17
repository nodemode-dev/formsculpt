use fsc_core::{Aabb, ObjectId};
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn ids(s: &Session, args: &Value, all_if_missing: bool) -> Result<Vec<ObjectId>, CommandError> {
    match args.get("ids") {
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            for v in a {
                let id = v.as_u64().ok_or_else(|| bad("ids are object ids"))? as ObjectId;
                if s.scene.get(id).is_none() {
                    return Err(CommandError::new("not_found", format!("no object {id}; get_scene lists object ids")));
                }
                if !out.contains(&id) {
                    out.push(id);
                }
            }
            Ok(out)
        }
        None | Some(Value::Null) if all_if_missing => Ok(s.scene.objects.iter().map(|o| o.id).collect()),
        None | Some(Value::Null) => Err(bad("ids is required")),
        Some(_) => Err(bad("ids must be an array")),
    }
}

fn bounds(s: &Session, id: ObjectId) -> Aabb {
    let mut b = Aabb::EMPTY;
    for &p in &s.scene.get(id).expect("checked").mesh.positions {
        b.grow(Vec3::from(p));
    }
    b
}

fn translate(s: &mut Session, id: ObjectId, by: Vec3) {
    if by.length_squared() == 0.0 {
        return;
    }
    let o = s.scene.get_mut(id).expect("checked");
    for p in o.mesh.positions.iter_mut() {
        *p = (Vec3::from(*p) + by).to_array();
    }
    if let Some(m) = o.multires.as_mut() {
        m.translate(by);
    }
    o.bvh.refit(&o.mesh);
    o.revision += 1;
}

pub(crate) fn arrange_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ids": {"type": "array", "minItems": 2, "items": {"type": "integer", "minimum": 1}, "description": "The objects to arrange; every object when omitted."},
            "axis": {"type": "string", "enum": ["x", "y", "z"], "default": "x"},
            "mode": {"type": "string", "enum": ["min", "center", "max", "distribute"], "default": "center", "description": "min, center and max line the objects' bounds up on the axis (at the first object's); distribute spaces them evenly between the outermost two, with equal gaps between their bounds."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn arrange(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let ids = ids(s, args, true)?;
    if ids.len() < 2 {
        return Err(bad("arranging needs at least two objects"));
    }
    let axis = match args.get("axis").and_then(Value::as_str).unwrap_or("x") {
        "x" => 0,
        "y" => 1,
        "z" => 2,
        other => return Err(bad(format!("axis must be x, y or z, not {other}"))),
    };
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("center").to_string();
    if !["min", "center", "max", "distribute"].contains(&mode.as_str()) {
        return Err(bad(format!("unknown mode {mode}")));
    }
    let boxes: Vec<(ObjectId, Aabb)> = ids.iter().map(|&id| (id, bounds(s, id))).collect();
    let unit = |by: f32| {
        let mut v = Vec3::ZERO;
        v[axis] = by;
        v
    };
    let mut moves: Vec<(ObjectId, Vec3)> = Vec::new();
    if mode == "distribute" {
        let mut sorted = boxes.clone();
        sorted.sort_by(|a, b| a.1.center()[axis].partial_cmp(&b.1.center()[axis]).unwrap_or(std::cmp::Ordering::Equal));
        let start = sorted[0].1.min[axis];
        let end = sorted[sorted.len() - 1].1.max[axis];
        let sizes: f32 = sorted.iter().map(|(_, b)| b.max[axis] - b.min[axis]).sum();
        let gap = (end - start - sizes) / (sorted.len() - 1) as f32;
        let mut at = start;
        for (id, b) in &sorted {
            moves.push((*id, unit(at - b.min[axis])));
            at += b.max[axis] - b.min[axis] + gap;
        }
    } else {
        let key = |b: &Aabb| match mode.as_str() {
            "min" => b.min[axis],
            "max" => b.max[axis],
            _ => b.center()[axis],
        };
        let goal = key(&boxes[0].1);
        for (id, b) in &boxes {
            moves.push((*id, unit(goal - key(b))));
        }
    }
    s.history.push(Edit::Scene { label: if mode == "distribute" { "Distribute".into() } else { "Align".into() }, objects: s.scene.objects.clone(), active: s.scene.active });
    let mut moved = 0;
    for (id, by) in moves {
        if by.length() > 1e-7 {
            moved += 1;
        }
        translate(s, id, by);
    }
    let axis_name = ["x", "y", "z"][axis];
    Ok(json!({"moved": moved, "axis": axis_name, "mode": mode}))
}

pub(crate) fn visibility_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["hide", "show", "isolate", "show_all"], "description": "hide or show the ids; isolate shows only the ids (or the active object) and hides the rest; show_all shows everything."},
            "ids": {"type": "array", "items": {"type": "integer", "minimum": 1}, "description": "The objects; the active object when omitted."},
            "folder": {"type": "string", "description": "Instead of ids: every object in this folder."}
        },
        "required": ["action"],
        "additionalProperties": false
    })
}

pub(crate) fn folder_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ids": {"type": "array", "items": {"type": "integer", "minimum": 1}, "description": "The objects to file; the active object when omitted."},
            "folder": {"type": "string", "maxLength": 80, "description": "The folder name; an empty string takes them out of any folder."}
        },
        "required": ["folder"],
        "additionalProperties": false
    })
}

/// Files objects under a folder (one level). One undo step.
pub(crate) fn set_folder(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let folder: String = args.get("folder").and_then(Value::as_str).ok_or_else(|| bad("folder is required"))?.trim().chars().take(80).collect();
    let chosen: Vec<ObjectId> = match args.get("ids") {
        None | Some(Value::Null) => s.scene.active.into_iter().collect(),
        _ => ids(s, args, false)?,
    };
    if chosen.is_empty() {
        return Err(CommandError::new("no_object", "the scene is empty"));
    }
    s.history.push(Edit::Scene { label: "Folder".into(), objects: s.scene.objects.clone(), active: s.scene.active });
    for o in s.scene.objects.iter_mut().filter(|o| chosen.contains(&o.id)) {
        o.folder = folder.clone();
        o.revision += 1;
    }
    let mut folders: Vec<String> = s.scene.objects.iter().map(|o| o.folder.clone()).filter(|f| !f.is_empty()).collect();
    folders.sort();
    folders.dedup();
    Ok(json!({"filed": chosen, "folder": folder, "folders": folders}))
}

pub(crate) fn set_visibility(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let action = args.get("action").and_then(Value::as_str).ok_or_else(|| bad("action is required"))?.to_string();
    let chosen = match (args.get("ids"), args.get("folder").and_then(Value::as_str)) {
        (_, Some(folder)) => {
            let found: Vec<ObjectId> = s.scene.objects.iter().filter(|o| o.folder == folder).map(|o| o.id).collect();
            if found.is_empty() {
                return Err(CommandError::new("not_found", format!("no objects in a folder named {folder}")));
            }
            found
        }
        (None | Some(Value::Null), None) => s.scene.active.into_iter().collect(),
        _ => ids(s, args, false)?,
    };
    if chosen.is_empty() && action != "show_all" {
        return Err(CommandError::new("no_object", "the scene is empty"));
    }
    for o in s.scene.objects.iter_mut() {
        let pick = chosen.contains(&o.id);
        let visible = match action.as_str() {
            "hide" if pick => false,
            "show" if pick => true,
            "isolate" => pick,
            "show_all" => true,
            "hide" | "show" => o.visible,
            other => return Err(bad(format!("unknown action {other}"))),
        };
        if o.visible != visible {
            o.visible = visible;
            o.revision += 1;
        }
    }
    let shown: Vec<ObjectId> = s.scene.objects.iter().filter(|o| o.visible).map(|o| o.id).collect();
    Ok(json!({"visible": shown, "hidden": s.scene.objects.len() - shown.len()}))
}

pub(crate) fn pick_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "origin": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
            "direction": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}
        },
        "required": ["origin", "direction"],
        "additionalProperties": false
    })
}

fn vec3(args: &Value, key: &str) -> Result<Vec3, CommandError> {
    let a = args.get(key).and_then(Value::as_array).filter(|a| a.len() == 3).ok_or_else(|| bad(format!("{key} is [x, y, z]")))?;
    let v: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
    if v.iter().any(|x| !x.is_finite()) {
        return Err(bad(format!("{key} values must be numbers")));
    }
    Ok(Vec3::new(v[0], v[1], v[2]))
}

pub(crate) fn pick(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let origin = vec3(args, "origin")?;
    let dir = vec3(args, "direction")?.normalize_or(Vec3::NEG_Z);
    let mut best: Option<(f32, ObjectId, Vec3, Vec3)> = None;
    for o in s.scene.objects.iter().filter(|o| o.visible) {
        if let Some(hit) = o.bvh.raycast(&o.mesh, origin, dir) {
            if best.is_none_or_farther(hit.t) {
                let t = &o.mesh.indices[hit.triangle as usize * 3..hit.triangle as usize * 3 + 3];
                let (a, b, c) = (o.mesh.position(t[0]), o.mesh.position(t[1]), o.mesh.position(t[2]));
                best = Some((hit.t, o.id, hit.point, (b - a).cross(c - a).normalize_or(-dir)));
            }
        }
    }
    Ok(match best {
        Some((t, id, p, n)) => json!({"hit": true, "id": id, "point": p.to_array(), "normal": n.to_array(), "distance": t}),
        None => json!({"hit": false}),
    })
}

trait Farther {
    fn is_none_or_farther(&self, t: f32) -> bool;
}

impl Farther for Option<(f32, ObjectId, Vec3, Vec3)> {
    fn is_none_or_farther(&self, t: f32) -> bool {
        self.is_none_or(|b| t < b.0)
    }
}

pub(crate) fn history_schema() -> Value {
    json!({"type": "object", "properties": {}, "required": [], "additionalProperties": false})
}

fn camera_json(c: &Option<crate::StepCamera>) -> Value {
    match c {
        Some(c) => json!({"target": c.target, "distance": c.distance, "yaw": c.yaw, "pitch": c.pitch, "ortho": c.ortho}),
        None => Value::Null,
    }
}

const MB: usize = 1 << 20;

pub(crate) fn get_history(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    let (steps, done) = s.history.steps();
    let bytes = s.history.sizes();
    let cameras: Vec<Value> = s.history.cameras().iter().map(camera_json).collect();
    let (max_steps, max_bytes) = s.history.limits();
    Ok(json!({
        "steps": steps,
        "done": done,
        "bytes": bytes,
        "total_bytes": bytes.iter().sum::<usize>(),
        "cameras": cameras,
        "max_steps": max_steps,
        "max_memory_mb": max_bytes / MB,
        "recording_camera": s.history.camera.is_some(),
    }))
}

pub(crate) fn limits_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "max_steps": {"type": "integer", "minimum": 1, "maximum": crate::history::MAX_STEPS, "description": "Undo steps kept; the current limit when omitted."},
            "max_memory_mb": {"type": "integer", "minimum": 0, "maximum": 65536, "description": "Memory the steps may keep together, in MB; 0 is no limit; the current limit when omitted."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn set_history_limits(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let (steps0, bytes0) = s.history.limits();
    let steps = match args.get("max_steps") {
        Some(v) => v.as_u64().filter(|n| (1..=crate::history::MAX_STEPS as u64).contains(n)).ok_or_else(|| bad("max_steps must be 1 to 10000"))? as usize,
        None => steps0,
    };
    let bytes = match args.get("max_memory_mb") {
        Some(v) => v.as_u64().filter(|n| *n <= 65536).ok_or_else(|| bad("max_memory_mb must be 0 to 65536"))? as usize * MB,
        None => bytes0,
    };
    let dropped = s.history.set_limits(steps, bytes);
    let sizes = s.history.sizes();
    Ok(json!({"max_steps": steps, "max_memory_mb": bytes / MB, "dropped": dropped, "steps": sizes.len(), "total_bytes": sizes.iter().sum::<usize>()}))
}

pub(crate) fn camera_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "target": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The orbit target in world space."},
            "distance": {"type": "number", "exclusiveMinimum": 0, "description": "The eye's distance from the target."},
            "yaw": {"type": "number", "description": "Degrees around the vertical axis from the front."},
            "pitch": {"type": "number", "minimum": -90, "maximum": 90, "description": "Degrees above the horizon."},
            "ortho": {"type": "boolean", "default": false},
            "clear": {"type": "boolean", "default": false, "description": "Stop recording a camera with the next steps."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn set_history_camera(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    if args.get("clear").and_then(Value::as_bool).unwrap_or(false) {
        s.history.camera = None;
        return Ok(json!({"recording": false}));
    }
    let num = |k: &str| args.get(k).and_then(Value::as_f64).filter(|v| v.is_finite()).map(|v| v as f32).ok_or_else(|| bad(format!("{k} is required (or clear)")));
    let target = args.get("target").and_then(Value::as_array).filter(|a| a.len() == 3).ok_or_else(|| bad("target must be [x, y, z]"))?;
    let mut t = [0f32; 3];
    for (slot, v) in t.iter_mut().zip(target) {
        *slot = v.as_f64().filter(|v| v.is_finite()).ok_or_else(|| bad("target must be three numbers"))? as f32;
    }
    let distance = num("distance")?;
    if distance <= 0.0 {
        return Err(bad("distance must be above 0"));
    }
    let camera = crate::StepCamera { target: t, distance, yaw: num("yaw")?, pitch: num("pitch")?.clamp(-90.0, 90.0), ortho: args.get("ortho").and_then(Value::as_bool).unwrap_or(false) };
    s.history.camera = Some(camera);
    Ok(json!({"recording": true}))
}

pub(crate) fn goto_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"done": {"type": "integer", "minimum": 0, "description": "How many steps of get_history's list should be done; earlier numbers undo, later ones redo."}},
        "required": ["done"],
        "additionalProperties": false
    })
}

pub(crate) fn history_go_to(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let done = args.get("done").and_then(Value::as_u64).ok_or_else(|| bad("done is required"))? as usize;
    let moved = s.history.go_to(&mut s.scene, done).map_err(|e| CommandError::new("history", e))?;
    let (steps, done) = s.history.steps();
    let camera = s.history.cameras().get(done.max(1) - 1).copied().flatten();
    Ok(json!({"moved": moved, "done": done, "steps": steps.len(), "camera": camera_json(&camera)}))
}
