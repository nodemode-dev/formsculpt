use base64::Engine as _;
use fsc_core::Aabb;
use fsc_paint::{RenderObject, Shading, View};
use glam::Vec3;
use serde_json::{json, Value};

use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

const VIEWS: [(&str, f32, f32); 7] = [("three_quarter", 30.0, 20.0), ("front", 0.0, 0.0), ("back", 180.0, 0.0), ("right", 90.0, 0.0), ("left", -90.0, 0.0), ("top", 0.0, 89.9), ("bottom", 0.0, -89.9)];

pub(crate) fn view_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "view": {"type": "string", "enum": VIEWS.iter().map(|v| v.0).collect::<Vec<_>>(), "default": "three_quarter", "description": "Camera direction; three_quarter is front-right and a little above, as the window opens. Front looks along -z."},
            "yaw": {"type": "number", "minimum": -360, "maximum": 360, "description": "Degrees around the vertical axis from the front; overrides view."},
            "pitch": {"type": "number", "minimum": -90, "maximum": 90, "description": "Degrees above the horizon; overrides view."},
            "width": {"type": "integer", "minimum": 64, "maximum": 2048, "default": 640},
            "height": {"type": "integer", "minimum": 64, "maximum": 2048, "default": 640},
            "shading": {"type": "string", "enum": Shading::NAMES, "default": "clay", "description": "The window's matcaps, or pbr for painted roughness and metalness under three lights."},
            "ids": {"type": "array", "items": {"type": "integer", "minimum": 1}, "description": "Only these objects; every visible object when omitted."},
            "frame": {"type": "string", "enum": ["all", "active"], "default": "all", "description": "Fit the camera to all drawn objects or to the active one (others still drawn)."},
            "background": {"type": "string", "enum": ["dark", "light", "transparent"], "default": "dark"}
        },
        "required": [],
        "additionalProperties": false
    })
}

fn int(args: &Value, key: &str, default: u64, lo: u64, hi: u64) -> Result<usize, CommandError> {
    let v = args.get(key).and_then(Value::as_u64).unwrap_or(default);
    if !(lo..=hi).contains(&v) {
        return Err(bad(format!("{key} must be {lo} to {hi}")));
    }
    Ok(v as usize)
}

pub(crate) fn turntable_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "minLength": 1, "description": "Where to write the animation, ending in .png (an animated PNG that browsers play in a loop)."},
            "frames": {"type": "integer", "minimum": 8, "maximum": 180, "default": 48, "description": "Frames for one full turn."},
            "fps": {"type": "integer", "minimum": 1, "maximum": 60, "default": 24},
            "size": {"type": "integer", "minimum": 64, "maximum": 1024, "default": 384, "description": "Width and height in pixels."},
            "pitch": {"type": "number", "minimum": -89, "maximum": 89, "default": 15, "description": "Degrees above the horizon."},
            "shading": {"type": "string", "enum": Shading::NAMES, "default": "clay"},
            "background": {"type": "string", "enum": ["dark", "light", "transparent"], "default": "dark"},
            "ids": {"type": "array", "items": {"type": "integer", "minimum": 1}, "description": "Only these objects; every visible object when omitted."}
        },
        "required": ["path"],
        "additionalProperties": false
    })
}

/// A turntable: the scene rendered headless from `frames` angles around the vertical axis (framed once, so it turns
/// in place), written as a looping animated PNG.
pub(crate) fn record_turntable(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let path = args.get("path").and_then(Value::as_str).ok_or_else(|| bad("path is required"))?.to_string();
    if !path.to_ascii_lowercase().ends_with(".png") {
        return Err(bad("path must end in .png"));
    }
    let frames = int(args, "frames", 48, 8, 180)?;
    let fps = int(args, "fps", 24, 1, 60)? as u16;
    let size = int(args, "size", 384, 64, 1024)?;
    let pitch = args.get("pitch").and_then(Value::as_f64).unwrap_or(15.0).clamp(-89.0, 89.0) as f32;
    let started = std::time::Instant::now();
    let mut shots = Vec::with_capacity(frames);
    for k in 0..frames {
        let mut frame_args = args.clone();
        let obj = frame_args.as_object_mut().expect("object");
        for key in ["path", "frames", "fps", "size", "pitch"] {
            obj.remove(key);
        }
        obj.insert("yaw".into(), json!(360.0 * k as f64 / frames as f64));
        obj.insert("pitch".into(), json!(pitch));
        obj.insert("width".into(), json!(size));
        obj.insert("height".into(), json!(size));
        obj.insert("frame".into(), json!("all"));
        let (pixels, _) = render_pixels(s, &frame_args)?;
        shots.push(pixels);
    }
    let bytes = fsc_paint::apng_rgba(size, size, &shots, fps).map_err(|e| CommandError::new("render_failed", e))?;
    std::fs::write(&path, &bytes).map_err(|e| CommandError::new("write_failed", format!("could not write {path}: {e}")))?;
    Ok(json!({"path": path, "frames": frames, "size": size, "bytes": bytes.len(), "seconds": frames as f64 / fps as f64, "ms": started.elapsed().as_millis()}))
}

pub(crate) fn get_view(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let (pixels, mut facts) = render_pixels(s, args)?;
    let (w, h) = (facts["width"].as_u64().unwrap_or(0) as usize, facts["height"].as_u64().unwrap_or(0) as usize);
    let png = fsc_paint::png_rgba_rect(w, h, &pixels).map_err(|e| CommandError::new("render_failed", e))?;
    facts["mime"] = json!("image/png");
    facts["png_base64"] = json!(base64::engine::general_purpose::STANDARD.encode(&png));
    Ok(facts)
}

fn render_pixels(s: &mut Session, args: &Value) -> Result<(Vec<[u8; 4]>, Value), CommandError> {
    let name = args.get("view").and_then(Value::as_str).unwrap_or("three_quarter");
    let &(_, yaw, pitch) = VIEWS.iter().find(|v| v.0 == name).ok_or_else(|| bad(format!("unknown view {name}")))?;
    let yaw = args.get("yaw").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(yaw);
    let pitch = args.get("pitch").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(pitch).clamp(-89.9, 89.9);
    let width = int(args, "width", 640, 64, 2048)?;
    let height = int(args, "height", 640, 64, 2048)?;
    let shading_name = args.get("shading").and_then(Value::as_str).unwrap_or("clay");
    let shading = Shading::named(shading_name).ok_or_else(|| bad(format!("shading must be one of {}", Shading::NAMES.join(", "))))?;
    let background = match args.get("background").and_then(Value::as_str).unwrap_or("dark") {
        "dark" => Some([0.11, 0.106, 0.114]),
        "light" => Some([0.906, 0.898, 0.867]),
        "transparent" => None,
        other => return Err(bad(format!("unknown background {other}"))),
    };
    let ids: Option<Vec<u32>> = match args.get("ids") {
        Some(Value::Array(a)) => Some(a.iter().map(|v| v.as_u64().map(|x| x as u32).ok_or_else(|| bad("ids are object ids"))).collect::<Result<_, _>>()?),
        Some(Value::Null) | None => None,
        Some(_) => return Err(bad("ids must be an array")),
    };
    if let Some(ids) = &ids {
        if let Some(missing) = ids.iter().find(|id| s.scene.get(**id).is_none()) {
            return Err(CommandError::new("not_found", format!("no object {missing}; get_scene lists object ids")));
        }
    }
    let active = s.scene.active;
    let drawn: Vec<&fsc_core::Object> = s.scene.objects.iter().filter(|o| ids.as_ref().map_or(o.visible, |ids| ids.contains(&o.id))).collect();
    let frame = match args.get("frame").and_then(Value::as_str).unwrap_or("all") {
        "all" => None,
        "active" => {
            let id = active.ok_or_else(|| CommandError::new("no_object", "the scene is empty; add_primitive first"))?;
            let mut b = Aabb::EMPTY;
            for &p in &s.scene.get(id).expect("active exists").mesh.positions {
                b.grow(Vec3::from(p));
            }
            Some(b)
        }
        other => return Err(bad(format!("unknown frame {other}"))),
    };
    let objects: Vec<RenderObject> = drawn.iter().map(|o| RenderObject { mesh: &o.mesh, mask: &o.mask, active: Some(o.id) == active }).collect();
    let view = View { yaw: yaw.to_radians(), pitch: pitch.to_radians(), width, height, shading, background, frame, supersample: if width.max(height) <= 1024 { 2 } else { 1 } };
    let out = fsc_paint::render(&objects, &view).map_err(|e| CommandError::new("render_failed", e))?;
    let facts = json!({
        "width": out.width,
        "height": out.height,
        "objects": objects.len(),
        "triangles": out.triangles,
        "coverage": (out.covered as f64 * 1000.0).round() / 1000.0,
        "view": {"yaw": yaw, "pitch": pitch, "shading": shading_name},
        "ms": out.ms
    });
    Ok((out.pixels, facts))
}
