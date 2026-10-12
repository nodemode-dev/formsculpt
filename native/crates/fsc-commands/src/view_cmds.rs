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

pub(crate) fn animation_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "minLength": 1, "description": "Where to write the animation, ending in .png (an animated PNG that browsers play in a loop)."},
            "keys": {"type": "array", "minItems": 2, "maxItems": 64, "items": {"type": "object", "properties": {
                "time": {"type": "number", "minimum": 0, "maximum": 60, "description": "Seconds from the start; keys are sorted by it."},
                "yaw": {"type": "number", "minimum": -3600, "maximum": 3600, "description": "Camera degrees around the up axis (0 is the front); turns past 360 keep going round."},
                "pitch": {"type": "number", "minimum": -89, "maximum": 89, "description": "Camera degrees above the horizon."},
                "zoom": {"type": "number", "minimum": 0.1, "maximum": 20, "description": "1 frames the scene; 2 is twice as close."},
                "layers": {"type": "array", "items": {"type": "object", "properties": {"id": {"type": "integer", "minimum": 1}, "layer": {"type": "integer", "minimum": 0}, "strength": {"type": "number", "minimum": -1, "maximum": 2}}, "required": ["id", "layer", "strength"], "additionalProperties": false}, "description": "Sculpt layer strengths at this key (a morph: a smile, a blink, a swell)."}
            }, "required": ["time"], "additionalProperties": false}, "description": "Keyframes. Each value holds from its key to the next key that sets it, eased between them; values a key leaves out carry on from the keys around it."},
            "fps": {"type": "integer", "minimum": 1, "maximum": 60, "default": 24},
            "size": {"type": "integer", "minimum": 64, "maximum": 1024, "default": 384, "description": "Width and height in pixels."},
            "ease": {"type": "string", "enum": ["smooth", "linear"], "default": "smooth"},
            "shading": {"type": "string", "enum": Shading::NAMES, "default": "clay"},
            "background": {"type": "string", "enum": ["dark", "light", "transparent"], "default": "dark"},
            "ids": {"type": "array", "items": {"type": "integer", "minimum": 1}, "description": "Only these objects; every visible object when omitted."}
        },
        "required": ["path", "keys"],
        "additionalProperties": false
    })
}

/// One animated channel: (time, value) keys, sampled with easing; a channel with no keys is None.
fn channel(keys: &[(f32, f32)], t: f32, smooth: bool) -> Option<f32> {
    let first = keys.first()?;
    if t <= first.0 {
        return Some(first.1);
    }
    for w in keys.windows(2) {
        if t <= w[1].0 {
            let span = (w[1].0 - w[0].0).max(1e-6);
            let mut f = ((t - w[0].0) / span).clamp(0.0, 1.0);
            if smooth {
                f = f * f * (3.0 - 2.0 * f);
            }
            return Some(w[0].1 + (w[1].1 - w[0].1) * f);
        }
    }
    keys.last().map(|k| k.1)
}

/// Timeline: renders keyframed camera moves and sculpt layer strengths to an animated PNG. Layer strengths are put
/// back as they were afterwards, so the scene and its undo history are unchanged.
pub(crate) fn record_animation(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let path = args.get("path").and_then(Value::as_str).ok_or_else(|| bad("path is required"))?.to_string();
    if !path.to_ascii_lowercase().ends_with(".png") {
        return Err(bad("path must end in .png"));
    }
    let mut keys: Vec<&Value> = args.get("keys").and_then(Value::as_array).ok_or_else(|| bad("keys is a list of keyframes"))?.iter().collect();
    if keys.len() < 2 {
        return Err(bad("give at least two keys"));
    }
    keys.sort_by(|a, b| a["time"].as_f64().unwrap_or(0.0).total_cmp(&b["time"].as_f64().unwrap_or(0.0)));
    let fps = int(args, "fps", 24, 1, 60)?;
    let size = int(args, "size", 384, 64, 1024)?;
    let smooth = args.get("ease").and_then(Value::as_str) != Some("linear");
    let duration = keys.last().and_then(|k| k["time"].as_f64()).unwrap_or(0.0) as f32;
    if duration <= 0.0 {
        return Err(bad("the last key must come after time 0"));
    }
    let frames = ((duration * fps as f32).round() as usize).max(2);
    if frames > 600 {
        return Err(bad("at most 600 frames; shorten the keys or lower fps"));
    }
    let track = |name: &str| -> Vec<(f32, f32)> { keys.iter().filter_map(|k| Some((k["time"].as_f64()? as f32, k.get(name)?.as_f64()? as f32))).collect() };
    let (yaw, pitch, zoom) = (track("yaw"), track("pitch"), track("zoom"));
    // Layer tracks keyed by (object, layer index).
    let mut layer_tracks: std::collections::BTreeMap<(u32, usize), Vec<(f32, f32)>> = std::collections::BTreeMap::new();
    for k in &keys {
        let time = k["time"].as_f64().unwrap_or(0.0) as f32;
        for l in k.get("layers").and_then(Value::as_array).into_iter().flatten() {
            let (Some(id), Some(at), Some(v)) = (l["id"].as_u64(), l["layer"].as_u64(), l["strength"].as_f64()) else {
                return Err(bad("each layer key is {id, layer, strength}"));
            };
            let o = s.scene.get(id as u32).ok_or_else(|| CommandError::new("not_found", format!("no object {id}")))?;
            if at as usize >= o.layers.list.len() {
                return Err(CommandError::new("not_found", format!("object {id} has no layer {at}; layer_list shows them")));
            }
            layer_tracks.entry((id as u32, at as usize)).or_default().push((time, (v as f32).clamp(-1.0, 2.0)));
        }
    }
    let originals: Vec<((u32, usize), f32)> = layer_tracks.keys().map(|&(id, at)| ((id, at), s.scene.get(id).expect("checked").layers.list[at].strength)).collect();
    let started = std::time::Instant::now();
    let mut shots = Vec::with_capacity(frames);
    let mut failure = None;
    for f in 0..frames {
        let t = duration * f as f32 / (frames - 1) as f32;
        for (&(id, at), track) in &layer_tracks {
            if let Some(v) = channel(track, t, smooth) {
                let _ = s.scene.get_mut(id).expect("checked").set_layer_strength(at, v);
            }
        }
        let mut frame_args = json!({"width": size, "height": size, "frame": "all", "yaw": channel(&yaw, t, smooth).unwrap_or(30.0), "pitch": channel(&pitch, t, smooth).unwrap_or(15.0), "zoom": channel(&zoom, t, smooth).unwrap_or(1.0)});
        for key in ["shading", "background", "ids"] {
            if let Some(v) = args.get(key) {
                frame_args[key] = v.clone();
            }
        }
        match render_pixels(s, &frame_args) {
            Ok((pixels, _)) => shots.push(pixels),
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
    }
    for ((id, at), v) in originals {
        let _ = s.scene.get_mut(id).expect("checked").set_layer_strength(at, v);
    }
    if let Some(e) = failure {
        return Err(e);
    }
    let bytes = fsc_paint::apng_rgba(size, size, &shots, fps as u16).map_err(|e| CommandError::new("render_failed", e))?;
    std::fs::write(&path, &bytes).map_err(|e| CommandError::new("write_failed", format!("could not write {path}: {e}")))?;
    Ok(json!({"path": path, "frames": frames, "seconds": duration, "size": size, "bytes": bytes.len(), "layer_tracks": layer_tracks.len(), "ms": started.elapsed().as_millis()}))
}

pub(crate) fn history_movie_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "minLength": 1, "description": "Where to write the movie, ending in .png (an animated PNG)."},
            "fps": {"type": "integer", "minimum": 1, "maximum": 30, "default": 4, "description": "Undo steps shown per second."},
            "size": {"type": "integer", "minimum": 64, "maximum": 1024, "default": 384},
            "view": {"type": "string", "enum": ["front", "back", "left", "right", "top", "bottom", "three_quarter"], "default": "three_quarter"},
            "turn": {"type": "number", "minimum": -720, "maximum": 720, "default": 0, "description": "Degrees the camera turns over the whole movie, so the build is seen from around it."},
            "shading": {"type": "string", "enum": Shading::NAMES, "default": "clay"},
            "background": {"type": "string", "enum": ["dark", "light", "transparent"], "default": "dark"}
        },
        "required": ["path"],
        "additionalProperties": false
    })
}

/// Undo history movie: steps back to the start of the history and renders one frame per step up to where it was,
/// then leaves the scene and history where they were.
pub(crate) fn record_history(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let path = args.get("path").and_then(Value::as_str).ok_or_else(|| bad("path is required"))?.to_string();
    if !path.to_ascii_lowercase().ends_with(".png") {
        return Err(bad("path must end in .png"));
    }
    let fps = int(args, "fps", 4, 1, 30)?;
    let size = int(args, "size", 384, 64, 1024)?;
    let turn = args.get("turn").and_then(Value::as_f64).unwrap_or(0.0).clamp(-720.0, 720.0) as f32;
    let view = args.get("view").and_then(Value::as_str).unwrap_or("three_quarter");
    let &(_, yaw0, pitch) = VIEWS.iter().find(|v| v.0 == view).ok_or_else(|| bad(format!("unknown view {view}")))?;
    let (_, done) = s.history.steps();
    if done == 0 {
        return Err(CommandError::new("no_history", "there are no steps to show yet"));
    }
    if done > 600 {
        return Err(bad("the history has more than 600 steps; that is too long for one movie"));
    }
    let started = std::time::Instant::now();
    let mut shots = Vec::with_capacity(done + 1);
    let mut failure = None;
    for k in 0..=done {
        if let Err(e) = s.history.go_to(&mut s.scene, k) {
            failure = Some(CommandError::new("history", e));
            break;
        }
        if s.scene.objects.iter().all(|o| !o.visible) {
            continue;
        }
        let mut frame_args = json!({"width": size, "height": size, "frame": "all", "yaw": yaw0 + turn * k as f32 / done as f32, "pitch": pitch});
        for key in ["shading", "background"] {
            if let Some(v) = args.get(key) {
                frame_args[key] = v.clone();
            }
        }
        match render_pixels(s, &frame_args) {
            Ok((pixels, _)) => shots.push(pixels),
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
    }
    let back = s.history.go_to(&mut s.scene, done);
    if let Some(e) = failure {
        return Err(e);
    }
    back.map_err(|e| CommandError::new("history", e))?;
    if shots.len() < 2 {
        return Err(CommandError::new("no_history", "fewer than two steps had anything to show"));
    }
    // Hold the finished model for a second at the end.
    let last = shots.last().cloned().expect("two or more");
    for _ in 0..fps {
        shots.push(last.clone());
    }
    let bytes = fsc_paint::apng_rgba(size, size, &shots, fps as u16).map_err(|e| CommandError::new("render_failed", e))?;
    std::fs::write(&path, &bytes).map_err(|e| CommandError::new("write_failed", format!("could not write {path}: {e}")))?;
    Ok(json!({"path": path, "steps": done, "frames": shots.len(), "size": size, "bytes": bytes.len(), "ms": started.elapsed().as_millis()}))
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
    // zoom (the animation recorder's camera distance) frames a box shrunk or grown about the framed one's centre.
    let zoom = args.get("zoom").and_then(Value::as_f64).unwrap_or(1.0).clamp(0.1, 20.0) as f32;
    let frame = if (zoom - 1.0).abs() > 1e-4 {
        let b = frame.unwrap_or_else(|| {
            let mut b = Aabb::EMPTY;
            for o in &drawn {
                for &p in &o.mesh.positions {
                    b.grow(Vec3::from(p));
                }
            }
            b
        });
        let (c, h) = ((b.min + b.max) * 0.5, (b.max - b.min) * 0.5 / zoom);
        let mut z = Aabb::EMPTY;
        z.grow(c - h);
        z.grow(c + h);
        Some(z)
    } else {
        frame
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
