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

/// One of get_view's added path render lights (kept apart from view_schema, whose json! would otherwise nest too deep).
fn light_schema() -> Value {
    let three = |text: &str| json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": text});
    json!({"type": "object", "properties": {
        "kind": {"type": "string", "enum": ["directional", "spot"], "default": "directional", "description": "directional shines from a direction everywhere alike; spot shines from a point in a cone and falls off with distance."},
        "from": three("Directional: the world direction the light shines from (required for directional)."),
        "at": three("Spot: where the light is, in world space (required for spot)."),
        "to": three("Spot: the point it aims at (default the framed centre). At that distance its strength is intensity."),
        "cone": {"type": "number", "minimum": 1, "maximum": 179, "default": 45, "description": "Spot: the cone's full angle in degrees."},
        "softness": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.25, "description": "Spot: how much of the cone's edge fades (0 hard, 1 from the axis out)."},
        "color": {"type": "array", "items": {"type": "number", "minimum": 0}, "minItems": 3, "maxItems": 3, "default": [1, 1, 1], "description": "Linear colour; multiplied by kelvin's colour when both are given (a gel)."},
        "kelvin": {"type": "number", "minimum": 1667, "maximum": 25000, "description": "Colour temperature: 1900 candle, 2700 to 3200 tungsten, 5500 daylight, 6500 white, 7000 and up cool sky."},
        "intensity": {"type": "number", "minimum": 0, "maximum": 100, "default": 2.5}
    }, "additionalProperties": false})
}

pub(crate) fn view_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "view": {"type": "string", "enum": VIEWS.iter().map(|v| v.0).collect::<Vec<_>>(), "default": "three_quarter", "description": "Camera direction; three_quarter is front-right and a little above, as the window opens. Front looks along -z."},
            "saved": {"type": "string", "minLength": 1, "description": "Look from a saved view's direction (save_view, list_views); overrides view."},
            "yaw": {"type": "number", "minimum": -360, "maximum": 360, "description": "Degrees around the vertical axis from the front; overrides view."},
            "pitch": {"type": "number", "minimum": -90, "maximum": 90, "description": "Degrees above the horizon; overrides view."},
            "width": {"type": "integer", "minimum": 64, "maximum": 2048, "default": 640},
            "height": {"type": "integer", "minimum": 64, "maximum": 2048, "default": 640},
            "shading": {"type": "string", "enum": Shading::NAMES, "default": "clay", "description": "The window's matcaps, or pbr for painted roughness and metalness under three lights."},
            "ids": {"type": "array", "items": {"type": "integer", "minimum": 1}, "description": "Only these objects; every visible object when omitted."},
            "frame": {"type": "string", "enum": ["all", "active"], "default": "all", "description": "Fit the camera to all drawn objects or to the active one (others still drawn)."},
            "background": {"type": "string", "enum": ["dark", "light", "transparent"], "default": "dark"},
            "render": {"type": "string", "enum": ["preview", "path"], "default": "preview", "description": "preview draws as the window does (fast); path is a path-traced production render: sky and sun light with soft shadows, light bouncing between surfaces, glossy metal and smooth paint, glowing and see-through paint (seconds to minutes; samples and bounces trade time for noise)."},
            "samples": {"type": "integer", "minimum": 1, "maximum": 1024, "default": 32, "description": "Path render: paths per pixel; 32 is a quick look, 256 a clean final image."},
            "bounces": {"type": "integer", "minimum": 1, "maximum": 8, "default": 4, "description": "Path render: how many times light may bounce."},
            "tone": {"type": "string", "enum": fsc_paint::Tone::NAMES, "default": "reinhard", "description": "Path render: how light is brought to the screen. reinhard is soft; aces filmic and contrasty; agx keeps hot colours from skewing, fading them toward white as film does; neutral keeps base colours almost exact and only compresses highlights; none clips."},
            "exposure": {"type": "number", "minimum": 0, "maximum": 16, "default": 1, "description": "Path render grade: a multiplier on the light (2 is a stop brighter)."},
            "contrast": {"type": "number", "minimum": 0.25, "maximum": 2.5, "default": 1, "description": "Path render grade: contrast about mid grey (1 as rendered)."},
            "saturation": {"type": "number", "minimum": 0, "maximum": 2, "default": 1, "description": "Path render grade: 0 grey, 1 as rendered, 2 doubled."},
            "temperature": {"type": "number", "minimum": -1, "maximum": 1, "default": 0, "description": "Path render grade: white balance, negative cooler, positive warmer."},
            "pass": {"type": "string", "enum": ["beauty", "depth", "normal", "ao"], "default": "beauty", "description": "Path render: the lit image, or a compositing pass (depth near white to far black, world normal as colour, ambient occlusion)."},
            "lights": {"type": "array", "maxItems": 8, "items": light_schema(), "description": "Path render: directional and spot lights added to the key light (a rim, fill or a pool of light)."},
            "sun": {"type": "boolean", "default": true, "description": "Path render: keep the window's key light."},
            "aperture": {"type": "number", "minimum": 0, "maximum": 1, "default": 0, "description": "Path render: depth of field lens size as a fraction of the framed size; 0 keeps everything sharp, 0.05 is a gentle blur."},
            "focus": {"type": "number", "exclusiveMinimum": 0, "description": "Path render: distance from the camera that stays sharp, in world units; the framed centre when omitted."},
            "smooth": {"type": "integer", "minimum": 0, "maximum": 3, "default": 0, "description": "Dynamic subdivision: draw each object Loop-subdivided this many levels (the window's Smooth preview); the meshes are not changed."}
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
    let started = fsc_core::time::Instant::now();
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
    let started = fsc_core::time::Instant::now();
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
            "camera": {"type": "string", "enum": ["fixed", "steps"], "default": "fixed", "description": "fixed uses view and turn; steps looks at each step from the direction it was made from (recorded by set_history_camera), falling back to view and turn where a step has none."},
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
    let per_step = match args.get("camera").and_then(Value::as_str).unwrap_or("fixed") {
        "fixed" => false,
        "steps" => true,
        other => return Err(bad(format!("unknown camera {other}"))),
    };
    let cameras = s.history.cameras();
    let started = fsc_core::time::Instant::now();
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
        // The state after k steps is seen from where step k was made (the first step's camera for the start).
        let step_camera = if per_step { cameras.get(k.max(1) - 1).copied().flatten() } else { None };
        let (yaw, pitch) = step_camera.map_or((yaw0 + turn * k as f32 / done as f32, pitch), |c| (c.yaw.rem_euclid(360.0), c.pitch.clamp(-89.9, 89.9)));
        let yaw = if yaw > 180.0 { yaw - 360.0 } else { yaw };
        let mut frame_args = json!({"width": size, "height": size, "frame": "all", "yaw": yaw, "pitch": pitch});
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

pub(crate) fn save_view_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {"type": "string", "minLength": 1, "maxLength": 60, "description": "The view's name; saving under a name already used replaces that view."},
            "target": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The orbit target in world space."},
            "distance": {"type": "number", "exclusiveMinimum": 0, "description": "The eye's distance from the target."},
            "yaw": {"type": "number", "description": "Degrees around the vertical axis from the front."},
            "pitch": {"type": "number", "minimum": -90, "maximum": 90, "description": "Degrees above the horizon."},
            "ortho": {"type": "boolean", "default": false}
        },
        "required": ["name", "target", "distance", "yaw", "pitch"],
        "additionalProperties": false
    })
}

/// The views as get_scene-style JSON, in order.
fn views_json(s: &Session) -> Value {
    json!(s.scene.views.iter().map(|v| json!({"name": v.name, "target": v.target, "distance": v.distance, "yaw": v.yaw, "pitch": v.pitch, "ortho": v.ortho})).collect::<Vec<_>>())
}

pub(crate) fn save_view(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let name = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty() && n.chars().count() <= 60).ok_or_else(|| bad("name must be 1 to 60 characters"))?.to_string();
    let num = |k: &str| args.get(k).and_then(Value::as_f64).filter(|v| v.is_finite()).map(|v| v as f32).ok_or_else(|| bad(format!("{k} is required")));
    let t = args.get("target").and_then(Value::as_array).filter(|a| a.len() == 3).ok_or_else(|| bad("target must be [x, y, z]"))?;
    let mut target = [0f32; 3];
    for (slot, v) in target.iter_mut().zip(t) {
        *slot = v.as_f64().filter(|v| v.is_finite()).ok_or_else(|| bad("target must be three numbers"))? as f32;
    }
    let distance = num("distance")?;
    if distance <= 0.0 {
        return Err(bad("distance must be above 0"));
    }
    let view = fsc_core::SavedView { name: name.clone(), target, distance, yaw: num("yaw")?, pitch: num("pitch")?.clamp(-90.0, 90.0), ortho: args.get("ortho").and_then(Value::as_bool).unwrap_or(false) };
    let replaced = match s.scene.views.iter_mut().find(|v| v.name == name) {
        Some(v) => {
            *v = view;
            true
        }
        None => {
            if s.scene.views.len() >= 64 {
                return Err(bad("a scene keeps at most 64 saved views; delete one first"));
            }
            s.scene.views.push(view);
            false
        }
    };
    Ok(json!({"name": name, "replaced": replaced, "views": views_json(s)}))
}

pub(crate) fn list_views(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    Ok(json!({"views": views_json(s)}))
}

pub(crate) fn delete_view_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"name": {"type": "string", "minLength": 1, "description": "The saved view to delete."}},
        "required": ["name"],
        "additionalProperties": false
    })
}

pub(crate) fn delete_view(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let name = args.get("name").and_then(Value::as_str).ok_or_else(|| bad("name is required"))?;
    let before = s.scene.views.len();
    s.scene.views.retain(|v| v.name != name);
    if s.scene.views.len() == before {
        return Err(bad(format!("there is no saved view named {name}")));
    }
    Ok(json!({"deleted": name, "views": views_json(s)}))
}

fn render_pixels(s: &mut Session, args: &Value) -> Result<(Vec<[u8; 4]>, Value), CommandError> {
    let name = args.get("view").and_then(Value::as_str).unwrap_or("three_quarter");
    let &(_, yaw, pitch) = VIEWS.iter().find(|v| v.0 == name).ok_or_else(|| bad(format!("unknown view {name}")))?;
    // A saved view by name gives the direction (the framing still fits the drawn objects); yaw and pitch still override.
    let (yaw, pitch) = match args.get("saved").and_then(Value::as_str) {
        Some(saved) => {
            let v = s.scene.views.iter().find(|v| v.name == saved).ok_or_else(|| bad(format!("there is no saved view named {saved}; list_views lists them")))?;
            let yaw = v.yaw.rem_euclid(360.0);
            (if yaw > 180.0 { yaw - 360.0 } else { yaw }, v.pitch)
        }
        None => (yaw, pitch),
    };
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
    let smooth = int(args, "smooth", 0, 0, 3)? as u32;
    // The smooth preview draws subdivided copies without the mask, which belongs to the sculpted vertices.
    let smoothed: Vec<(fsc_core::Mesh, Vec<f32>)> = if smooth > 0 {
        drawn.iter().map(|o| {
            let m = fsc_topo::smooth_preview(&o.mesh, smooth);
            let mask = vec![0.0; m.vertex_count()];
            (m, mask)
        }).collect()
    } else {
        Vec::new()
    };
    let objects: Vec<RenderObject> = if smooth > 0 {
        drawn.iter().zip(&smoothed).map(|(o, (m, mask))| RenderObject { mesh: m, mask, active: Some(o.id) == active }).collect()
    } else {
        drawn.iter().map(|o| RenderObject { mesh: &o.mesh, mask: &o.mask, active: Some(o.id) == active }).collect()
    };
    let view = View { yaw: yaw.to_radians(), pitch: pitch.to_radians(), width, height, shading, background, frame, supersample: if width.max(height) <= 1024 { 2 } else { 1 } };
    let out = match args.get("render").and_then(Value::as_str).unwrap_or("preview") {
        "preview" => fsc_paint::render(&objects, &view),
        "path" => {
            let mut options = fsc_paint::PathOptions::new(int(args, "samples", 32, 1, 1024)? as u32, int(args, "bounces", 4, 1, 8)? as u32);
            options.pass = match args.get("pass").and_then(Value::as_str).unwrap_or("beauty") {
                "beauty" => fsc_paint::Pass::Beauty,
                "depth" => fsc_paint::Pass::Depth,
                "normal" => fsc_paint::Pass::Normal,
                "ao" => fsc_paint::Pass::Occlusion,
                other => return Err(bad(format!("pass must be beauty, depth, normal or ao, not {other}"))),
            };
            options.sun = args.get("sun").and_then(Value::as_bool).unwrap_or(true);
            let tone = args.get("tone").and_then(Value::as_str).unwrap_or("reinhard");
            options.tone = fsc_paint::Tone::parse(tone).ok_or_else(|| bad(format!("tone must be one of {}, not {tone}", fsc_paint::Tone::NAMES.join(", "))))?;
            let num = |k: &str, d: f64, lo: f64, hi: f64| args.get(k).and_then(Value::as_f64).unwrap_or(d).clamp(lo, hi) as f32;
            options.grade = fsc_paint::Grade { exposure: num("exposure", 1.0, 0.0, 16.0), contrast: num("contrast", 1.0, 0.25, 2.5), saturation: num("saturation", 1.0, 0.0, 2.0), temperature: num("temperature", 0.0, -1.0, 1.0) };
            options.aperture = args.get("aperture").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0) as f32;
            options.focus = args.get("focus").and_then(Value::as_f64).filter(|f| *f > 0.0).map(|f| f as f32);
            let three = |v: Option<&Value>, d: [f32; 3]| -> [f32; 3] {
                match v.and_then(Value::as_array) {
                    Some(a) if a.len() == 3 => [0, 1, 2].map(|i| a[i].as_f64().unwrap_or(d[i] as f64) as f32),
                    _ => d,
                }
            };
            for (i, l) in args.get("lights").and_then(Value::as_array).into_iter().flatten().take(8).enumerate() {
                let intensity = l.get("intensity").and_then(Value::as_f64).unwrap_or(2.5).clamp(0.0, 100.0) as f32;
                let mut color = glam::Vec3::from(three(l.get("color"), [1.0; 3])) * intensity;
                if let Some(k) = l.get("kelvin").and_then(Value::as_f64) {
                    color *= fsc_paint::kelvin(k as f32);
                }
                let kind = l.get("kind").and_then(Value::as_str).unwrap_or("directional");
                let spot = match kind {
                    "directional" => None,
                    "spot" => {
                        let at = l.get("at").filter(|v| v.as_array().is_some_and(|a| a.len() == 3)).ok_or_else(|| bad(format!("lights[{i}]: a spot light needs at, where it is")))?;
                        let at = glam::Vec3::from(three(Some(at), [0.0; 3]));
                        let to = l.get("to").map(|v| glam::Vec3::from(three(Some(v), [0.0; 3])));
                        if to.is_some_and(|t| t.distance(at) < 1e-6) {
                            return Err(bad(format!("lights[{i}]: to must differ from at")));
                        }
                        let cone = l.get("cone").and_then(Value::as_f64).unwrap_or(45.0).clamp(1.0, 179.0) as f32;
                        let softness = l.get("softness").and_then(Value::as_f64).unwrap_or(0.25).clamp(0.0, 1.0) as f32;
                        Some(fsc_paint::Spot { at, to, cone, softness })
                    }
                    other => return Err(bad(format!("lights[{i}]: kind must be directional or spot, not {other}"))),
                };
                if spot.is_none() && l.get("from").is_none() {
                    return Err(bad(format!("lights[{i}]: a directional light needs from, the direction it shines from")));
                }
                let from = glam::Vec3::from(three(l.get("from"), [0.0, 1.0, 0.0]));
                options.lights.push(fsc_paint::Light { from, color, spot });
            }
            fsc_paint::path_trace_with(&objects, &view, &options)
        }
        other => return Err(bad(format!("render must be preview or path, not {other}"))),
    }
    .map_err(|e| CommandError::new("render_failed", e))?;
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
