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

pub(crate) fn target_texture(args: &Value) -> Result<bool, CommandError> {
    match args.get("target") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::String(t)) if t == "vertex" => Ok(false),
        Some(Value::String(t)) if t == "texture" => Ok(true),
        Some(_) => Err(bad("target is vertex or texture")),
    }
}

pub fn parse_brush(args: &Value) -> Result<BrushSettings, CommandError> {
    let kind_name = args.get("brush").and_then(Value::as_str).unwrap_or("draw");
    let kind = BrushKind::parse(kind_name).ok_or_else(|| bad(format!("unknown brush {kind_name}; use one of {}", BrushKind::NAMES.join(", "))))?;
    let falloff_name = args.get("falloff").and_then(Value::as_str).unwrap_or("smooth");
    let falloff = match args.get("falloff_curve") {
        None | Some(Value::Null) => Falloff::parse(falloff_name).ok_or_else(|| bad(format!("unknown falloff {falloff_name}; use one of {}", Falloff::NAMES.join(", "))))?,
        Some(Value::Array(a)) if (2..=64).contains(&a.len()) => {
            let w: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
            Falloff::curve(&w).ok_or_else(|| bad("falloff_curve holds numbers from 0 to 1"))?
        }
        Some(_) => return Err(bad("falloff_curve is 2 to 64 weights from the centre to the rim")),
    };
    // ZBrush's Focal Shift, over whichever falloff was chosen.
    let falloff = match args.get("focal_shift") {
        None | Some(Value::Null) => falloff,
        Some(_) => falloff.focal_shifted(f(args, "focal_shift", 0.0, -100.0, 100.0)?),
    };
    let stroke_mode = args.get("stroke").and_then(Value::as_str).unwrap_or("dots");
    if !["dots", "roll", "spray"].contains(&stroke_mode) {
        return Err(bad(format!("unknown stroke {stroke_mode}; use dots, roll or spray")));
    }
    let sym = args.get("symmetry").cloned().unwrap_or(Value::Null);
    let flag = |k: &str| sym.get(k).and_then(Value::as_bool).unwrap_or(false);
    let radial = sym.get("radial").and_then(Value::as_u64).unwrap_or(1).clamp(1, 150) as u32;
    let size = match args.get("dyntopo_size") {
        None | Some(Value::Null) => None,
        Some(_) => Some(f(args, "dyntopo_size", 0.01, 1e-5, 1e4)?),
    };
    let color = parse_color(args.get("color"))?;
    let rough = match args.get("roughness") {
        None | Some(Value::Null) => None,
        Some(_) => Some(f(args, "roughness", 0.5, 0.0, 1.0)?),
    };
    let metal = match args.get("metalness") {
        None | Some(Value::Null) => None,
        Some(_) => Some(f(args, "metalness", 0.0, 0.0, 1.0)?),
    };
    let opacity = match args.get("opacity") {
        None | Some(Value::Null) => None,
        Some(_) => Some(f(args, "opacity", 1.0, 0.0, 1.0)?),
    };
    let emissive = match args.get("emissive") {
        None | Some(Value::Null) => None,
        Some(_) => Some(f(args, "emissive", 0.0, 0.0, 1.0)?),
    };
    let base = fsc_core::DEFAULT_PAINT;
    let c = color.unwrap_or([0.8, 0.2, 0.2]);
    let paint = [c[0], c[1], c[2], rough.unwrap_or(base[3]), metal.unwrap_or(base[4]), opacity.unwrap_or(base[5]), emissive.unwrap_or(base[6])];
    let given = [color.is_some(), rough.is_some(), metal.is_some(), opacity.is_some(), emissive.is_some()];
    let paint_channels = if given.iter().any(|&g| g) { given } else { [true, false, false, false, false] };
    let dyntopo = match args.get("dyntopo") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => None,
        Some(Value::Bool(true)) => Some(0.25),
        Some(_) => Some(f(args, "dyntopo", 0.25, 0.05, 1.0)?),
    };
    let jitter = match args.get("jitter") {
        None | Some(Value::Null) => Value::Null,
        Some(v @ Value::Object(o)) => {
            if let Some(k) = o.keys().find(|k| !["position", "radius", "strength", "seed"].contains(&k.as_str())) {
                return Err(bad(format!("unknown jitter field {k}; use position, radius, strength or seed")));
            }
            v.clone()
        }
        Some(_) => return Err(bad("jitter is an object with position, radius, strength and seed")),
    };
    let jitter_seed = match jitter.get("seed") {
        None | Some(Value::Null) => 1,
        Some(v) => v.as_u64().ok_or_else(|| bad("jitter.seed must be a non-negative integer"))?,
    };
    let view_dir = match args.get("view_dir") {
        None | Some(Value::Null) => None,
        Some(Value::Array(a)) if a.len() == 3 => {
            let v: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
            if v.iter().any(|x| !x.is_finite()) || v.iter().all(|x| *x == 0.0) {
                return Err(bad("view_dir must be a non-zero [x, y, z]"));
            }
            Some([v[0], v[1], v[2]])
        }
        Some(_) => return Err(bad("view_dir is [x, y, z]")),
    };
    let mut protect_groups = fsc_sculpt::GroupList::default();
    match args.get("protect_groups") {
        None | Some(Value::Null) => {}
        Some(Value::Array(a)) if a.len() <= protect_groups.ids.len() => {
            for g in a {
                let id = g.as_u64().filter(|&g| g <= u32::MAX as u64).ok_or_else(|| bad("protect_groups holds group ids (non-negative integers)"))?;
                protect_groups.ids[protect_groups.len] = id as u32;
                protect_groups.len += 1;
            }
        }
        Some(_) => return Err(bad("protect_groups is up to 16 group ids")),
    }
    let flag_arg = |k: &str, default: bool| match args.get(k) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(bad(format!("{k} must be true or false"))),
    };
    let texture = target_texture(args)?;
    if texture && kind != BrushKind::Paint {
        return Err(bad("target texture is for the paint brush only"));
    }
    Ok(BrushSettings {
        kind,
        texture,
        front_facing: flag_arg("front_facing", false)?,
        view_dir,
        connected_only: flag_arg("connected_only", false)?,
        protect_group: flag_arg("protect_group", false)?,
        protect_groups,
        keep_sharp: f(args, "keep_sharp", 0.0, 0.0, 180.0)?,
        accumulate: flag_arg("accumulate", true)?,
        area_sample: f(args, "area_sample", 1.0, 0.1, 2.0)?,
        depth: f(args, "depth", 0.0, 0.0, 1e4)?,
        spacing_compensation: args.get("spacing_compensation").and_then(Value::as_bool).unwrap_or(false),
        offset: f(args, "offset", 0.0, -1.0, 1.0)?,
        jitter_position: f(&jitter, "position", 0.0, 0.0, 2.0)?,
        jitter_radius: f(&jitter, "radius", 0.0, 0.0, 1.0)?,
        jitter_strength: f(&jitter, "strength", 0.0, 0.0, 1.0)?,
        jitter_seed,
        snap_radius: f(args, "snap_radius", 0.0, 0.0, 1e4)?,
        radius: f(args, "radius", 0.2, 1e-5, 1e4)?,
        strength: f(args, "strength", 0.5, 0.0, 1.0)?,
        invert: args.get("invert").and_then(Value::as_bool).unwrap_or(false),
        falloff,
        spacing: f(args, "spacing", 0.15, 0.02, 2.0)?,
        lazy: f(args, "lazy", 0.0, 0.0, 0.95)?,
        symmetry: Symmetry { x: flag("x"), y: flag("y"), z: flag("z"), radial, local: flag("local"), center: [0.0; 3] },
        dyntopo: dyntopo.or(size.map(|_| 0.25)),
        dyntopo_size: size,
        dyntopo_mode: match args.get("dyntopo_mode").and_then(Value::as_str) {
            None => fsc_sculpt::DyntopoMode::Balanced,
            Some(m) => fsc_sculpt::DyntopoMode::parse(m).ok_or_else(|| bad(format!("dyntopo_mode is one of {}", fsc_sculpt::DyntopoMode::NAMES.join(", "))))?,
        },
        dyntopo_method: match args.get("dyntopo_method").and_then(Value::as_str) {
            None => fsc_sculpt::DyntopoMethod::Both,
            Some(m) => fsc_sculpt::DyntopoMethod::parse(m).ok_or_else(|| bad(format!("dyntopo_method is one of {}", fsc_sculpt::DyntopoMethod::NAMES.join(", "))))?,
        },
        dyntopo_protect_mask: flag_arg("dyntopo_protect_mask", false)?,
        paint,
        paint_channels,
        noise_scale: f(args, "noise_scale", 0.0, 0.0, 1e4)?,
        alpha_up: match args.get("alpha_up").and_then(Value::as_array) {
            Some(a) if a.len() == 3 => {
                let v: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
                if v.iter().any(|x| !x.is_finite()) || v.iter().all(|x| *x == 0.0) {
                    return Err(bad("alpha_up must be a non-zero [x, y, z]"));
                }
                [v[0], v[1], v[2]]
            }
            Some(_) => return Err(bad("alpha_up is [x, y, z]")),
            None => [0.0, 1.0, 0.0],
        },
        alpha_angle: f(args, "alpha_angle", 0.0, -360.0, 360.0)?.to_radians(),
        alpha_stretch: f(args, "alpha_stretch", 1.0, 0.1, 10.0)?,
        alpha_rotation: f(args, "alpha_rotation", 0.0, -360.0, 360.0)?.to_radians(),
        alpha_tile: match args.get("alpha_tile") {
            None | Some(Value::Null) => 1,
            Some(v) => v.as_u64().filter(|n| (1..=16).contains(n)).ok_or_else(|| bad("alpha_tile must be an integer between 1 and 16"))? as u32,
        },
        alpha_mid: f(args, "alpha_mid", 0.0, 0.0, 1.0)?,
        alpha_invert: flag_arg("alpha_invert", false)?,
        roll: stroke_mode == "roll",
        spray: if stroke_mode == "spray" { f(args, "spray", 0.6, 0.05, 2.0)? } else { 0.0 },
        pressure_gamma: f(args, "pressure_gamma", 1.0, 0.2, 5.0)?,
        pressure_strength: args.get("pressure_strength").and_then(Value::as_bool).unwrap_or(true),
        pressure_size: args.get("pressure_size").and_then(Value::as_bool).unwrap_or(false),
        height: f(args, "height", 0.0, 0.0, 1e4)?,
    })
}

/// The alpha a stroke names: a built-in pattern or one loaded with alpha_load. None for "none" or when omitted.
pub fn stroke_alpha(s: &Session, args: &Value) -> Result<Option<std::sync::Arc<fsc_sculpt::Alpha>>, CommandError> {
    match args.get("alpha").and_then(Value::as_str) {
        None | Some("none") | Some("") => Ok(None),
        Some(name) => {
            if let Some(a) = s.alphas.get(name) {
                return Ok(Some(a.clone()));
            }
            fsc_sculpt::Alpha::builtin(name).map(|a| Some(std::sync::Arc::new(a))).ok_or_else(|| {
                let mut known: Vec<&str> = fsc_sculpt::Alpha::BUILTIN.to_vec();
                known.extend(s.alphas.keys().map(String::as_str));
                bad(format!("unknown alpha {name}; use none or one of {}", known.join(", ")))
            })
        }
    }
}

pub(crate) fn parse_color(v: Option<&Value>) -> Result<Option<[f32; 3]>, CommandError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let h = s.trim().trim_start_matches('#');
            if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(bad("color must be #rrggbb or [r, g, b] from 0 to 1"));
            }
            let n = u32::from_str_radix(h, 16).expect("hex");
            Ok(Some([((n >> 16) & 255) as f32 / 255.0, ((n >> 8) & 255) as f32 / 255.0, (n & 255) as f32 / 255.0]))
        }
        Some(Value::Array(a)) if a.len() == 3 => {
            let mut out = [0.0f32; 3];
            for (k, x) in a.iter().enumerate() {
                out[k] = x.as_f64().filter(|x| (0.0..=1.0).contains(x)).ok_or_else(|| bad("color components must be 0 to 1"))? as f32;
            }
            Ok(Some(out))
        }
        _ => Err(bad("color must be #rrggbb or [r, g, b] from 0 to 1")),
    }
}

pub(crate) fn paint_fill_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "color": {"type": ["string", "array"], "description": "#rrggbb or [r, g, b] from 0 to 1."},
            "roughness": {"type": "number", "minimum": 0, "maximum": 1},
            "metalness": {"type": "number", "minimum": 0, "maximum": 1},
            "opacity": {"type": "number", "minimum": 0, "maximum": 1, "description": "1 is solid; lower lets what is behind show through (drawn dithered in the window, as alpha in GLB and baked colour maps)."},
            "emissive": {"type": "number", "minimum": 0, "maximum": 1, "description": "How much the surface glows in its own colour, unlit (lamps, lava, eyes)."},
            "target": {"type": "string", "enum": ["vertex", "texture"], "default": "vertex", "description": "texture fills every texel of the object's texture (texture_create first) instead of the vertex paint; masked and hidden areas are protected as for vertices."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn paint_fill(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let color = parse_color(args.get("color"))?;
    let unit = |k: &str| args.get(k).and_then(Value::as_f64).map(|x| x.clamp(0.0, 1.0) as f32);
    let (rough, metal, opacity, emissive) = (unit("roughness"), unit("metalness"), unit("opacity"), unit("emissive"));
    let channels = [color.is_some(), rough.is_some(), metal.is_some(), opacity.is_some(), emissive.is_some()];
    if !channels.iter().any(|&c| c) {
        return Err(bad("give color, roughness, metalness, opacity or emissive"));
    }
    if target_texture(args)? {
        let c = color.unwrap_or([0.0; 3]);
        let fill = [c[0], c[1], c[2], rough.unwrap_or(0.0), metal.unwrap_or(0.0), opacity.unwrap_or(1.0), emissive.unwrap_or(0.0)];
        return crate::texture_cmds::fill_texture(s, id, fill, channels);
    }
    let object = s.scene.get_mut(id).expect("checked");
    let had = object.mesh.has_paint();
    object.mesh.ensure_paint();
    let mut edit = VertexEdit::default();
    let mut layer = object.layered().then(|| (object.paint_layers.active, Vec::new(), Vec::new()));
    let c = color.unwrap_or([0.0; 3]);
    let fill = [c[0], c[1], c[2], rough.unwrap_or(0.0), metal.unwrap_or(0.0), opacity.unwrap_or(1.0), emissive.unwrap_or(0.0)];
    for v in 0..object.mesh.vertex_count() {
        let w = 1.0 - object.mask[v];
        if w <= 0.0 {
            continue;
        }
        let before = object.mesh.paint[v];
        let was = object.paint_target(v as u32);
        // Fills the active paint layer or the base when the object has paint layers, the paint itself otherwise.
        object.paint_vertex(v as u32, fill, channels, w, false);
        object.composite_paint(Some(&[v as u32]));
        let after = object.mesh.paint[v];
        let now = object.paint_target(v as u32);
        if after == before && was == now {
            continue;
        }
        if let (Some((_, b, a)), Some(was), Some(now)) = (layer.as_mut(), was, now) {
            b.push(was);
            a.push(now);
        }
        let p = object.mesh.positions[v];
        edit.indices.push(v as u32);
        edit.before.push(p);
        edit.after.push(p);
        edit.mask_before.push(object.mask[v]);
        edit.mask_after.push(object.mask[v]);
        edit.paint_before.push(if had { before } else { fsc_core::DEFAULT_PAINT });
        edit.paint_after.push(after);
    }
    if layer.as_ref().is_some_and(|(_, b, _)| b.len() == edit.indices.len()) {
        edit.layer = layer;
    }
    object.revision += 1;
    let n = edit.indices.len();
    let after = summary(object);
    if n > 0 {
        push_vertices(s, id, "Fill Paint".into(), edit);
    }
    Ok(json!({"object": after, "vertices_painted": n}))
}

pub(crate) fn project_schema() -> Value {
    json!({"type": "object", "properties": {
        "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
        "png_base64": {"type": "string", "description": "The image as a base64 PNG."},
        "path": {"type": "string", "description": "A PNG file on the engine's machine, instead of png_base64."},
        "mode": {"type": "string", "enum": ["view", "triplanar"], "default": "view", "description": "view: projected from a camera like a slide projector, onto the surfaces facing it and not hidden behind others; triplanar: tiled over the whole surface from the three axis planes (fabric, stone, skin texture)."},
        "view": {"type": "string", "enum": ["front", "back", "right", "left", "top", "bottom", "three_quarter"], "default": "front", "description": "view mode: the named camera, framed as get_view frames it, so the image lands where it would sit over a get_view picture of the same view and aspect."},
        "yaw": {"type": "number"}, "pitch": {"type": "number"},
        "width": {"type": "number", "minimum": 1, "maximum": 8192, "default": 640}, "height": {"type": "number", "minimum": 1, "maximum": 8192, "default": 640},
        "view_proj": {"type": "array", "items": {"type": "number"}, "minItems": 16, "maxItems": 16, "description": "view mode: a column-major view-projection matrix instead of a named view (the window sends its camera's); the image covers the whole view."},
        "scale": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "description": "triplanar: the image's size in world units; a quarter of the object's size when omitted."},
        "opacity": {"type": "number", "minimum": 0, "maximum": 1, "default": 1, "description": "How strongly the image replaces the paint (its own alpha multiplies this)."}
    }, "additionalProperties": false})
}

/// Projection painting: an image painted onto the surface from a view or tiled triplanar, into the active paint target.
pub(crate) fn paint_project(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    use base64::Engine as _;
    use glam::{Mat4, Vec4};
    let id = target(s, args)?;
    let bytes = match (args.get("png_base64").and_then(Value::as_str), args.get("path").and_then(Value::as_str)) {
        (Some(b), None) => base64::engine::general_purpose::STANDARD.decode(b.trim()).map_err(|e| bad(format!("png_base64 is not base64: {e}")))?,
        (None, Some(p)) => std::fs::read(p).map_err(|e| CommandError::new("read_failed", format!("could not read {p}: {e}")))?,
        _ => return Err(bad("give exactly one of png_base64 or path")),
    };
    let (iw, ih, img) = fsc_paint::png_rgba_decode(&bytes).map_err(bad)?;
    let opacity = f(args, "opacity", 1.0, 0.0, 1.0)?;
    // Bilinear sample at u, v in 0..1 (v down), clamped or tiled.
    let sample = |u: f32, v: f32, tile: bool| -> [f32; 4] {
        let (u, v) = if tile { (u.rem_euclid(1.0), v.rem_euclid(1.0)) } else { (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)) };
        let x = u * (iw as f32 - 1.0);
        let y = v * (ih as f32 - 1.0);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(iw - 1), (y0 + 1).min(ih - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let px = |x: usize, y: usize| img[y * iw + x];
        let mut out = [0.0; 4];
        for (c, o) in out.iter_mut().enumerate() {
            let a = px(x0, y0)[c] as f32 * (1.0 - fx) + px(x1, y0)[c] as f32 * fx;
            let b = px(x0, y1)[c] as f32 * (1.0 - fx) + px(x1, y1)[c] as f32 * fx;
            *o = (a * (1.0 - fy) + b * fy) / 255.0;
        }
        out
    };
    let srgb = |c: f32| c.powf(2.2);
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("view");
    let object = s.scene.get(id).expect("checked");
    let vc = object.mesh.vertex_count();
    // Per vertex: the colour to paint and its weight (0 leaves the vertex alone).
    let mut targets: Vec<Option<([f32; 3], f32)>> = vec![None; vc];
    match mode {
        "triplanar" => {
            let size = object.mesh.bounds().extent().length();
            let scale = args.get("scale").and_then(Value::as_f64).map(|x| x as f32).unwrap_or(size * 0.25).max(1e-6);
            for (v, slot) in targets.iter_mut().enumerate() {
                let p = Vec3::from(object.mesh.positions[v]) / scale;
                let n = Vec3::from(object.mesh.normals[v]);
                let w = Vec3::new(n.x.abs(), n.y.abs(), n.z.abs()).powf(4.0);
                let total = (w.x + w.y + w.z).max(1e-6);
                let mut c = [0.0f32; 4];
                for (s4, wt) in [(sample(p.z, -p.y, true), w.x), (sample(p.x, -p.z, true), w.y), (sample(p.x, -p.y, true), w.z)] {
                    for k in 0..4 {
                        c[k] += s4[k] * wt / total;
                    }
                }
                *slot = Some(([srgb(c[0]), srgb(c[1]), srgb(c[2])], c[3] * opacity));
            }
        }
        "view" => {
            // Screen coordinates (0..1, y down) and the direction toward the camera for each vertex.
            let matrix = match args.get("view_proj") {
                Some(Value::Array(a)) if a.len() == 16 => {
                    let m: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
                    if m.iter().any(|x| !x.is_finite()) {
                        return Err(bad("view_proj values must be numbers"));
                    }
                    Some(Mat4::from_cols_slice(&m))
                }
                Some(Value::Null) | None => None,
                Some(_) => return Err(bad("view_proj is 16 numbers")),
            };
            type Toward = Box<dyn Fn(Vec3) -> Vec3>;
            let (screen, toward): (Vec<Option<[f32; 2]>>, Toward) = match matrix {
                Some(m) => {
                    let inv = m.inverse();
                    let screen = object.mesh.positions.iter().map(|&p| {
                        let c = m * Vec4::new(p[0], p[1], p[2], 1.0);
                        (c.w > 1e-9).then(|| [(c.x / c.w) * 0.5 + 0.5, 0.5 - (c.y / c.w) * 0.5])
                    }).collect();
                    let toward = move |p: Vec3| {
                        let c = m * p.extend(1.0);
                        let near = inv * Vec4::new(c.x / c.w, c.y / c.w, -1.0, 1.0);
                        (near.truncate() / near.w - p).normalize_or_zero()
                    };
                    (screen, Box::new(toward))
                }
                None => {
                    let views = [("three_quarter", 30.0, 20.0), ("front", 0.0, 0.0), ("back", 180.0, 0.0), ("right", 90.0, 0.0), ("left", -90.0, 0.0), ("top", 0.0, 89.9), ("bottom", 0.0, -89.9)];
                    let name = args.get("view").and_then(Value::as_str).unwrap_or("front");
                    let &(_, yaw0, pitch0) = views.iter().find(|v| v.0 == name).ok_or_else(|| bad(format!("unknown view {name}")))?;
                    let yaw = args.get("yaw").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(yaw0).to_radians();
                    let pitch = args.get("pitch").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(pitch0).clamp(-89.9, 89.9).to_radians();
                    let aspect = f(args, "width", 640.0, 1.0, 8192.0)? / f(args, "height", 640.0, 1.0, 8192.0)?;
                    let mut frame = fsc_core::Aabb::EMPTY;
                    for o in s.scene.objects.iter().filter(|o| o.visible) {
                        for &p in &o.mesh.positions {
                            frame.grow(Vec3::from(p));
                        }
                    }
                    let screen = fsc_paint::view_coordinates(frame, yaw, pitch, aspect, &object.mesh.positions).into_iter().map(Some).collect();
                    let dir = Vec3::new(pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos());
                    (screen, Box::new(move |_| dir))
                }
            };
            let size = object.mesh.bounds().extent().length().max(1e-6);
            for (v, slot) in targets.iter_mut().enumerate() {
                let Some([u, w]) = screen[v] else { continue };
                if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&w) {
                    continue;
                }
                let p = Vec3::from(object.mesh.positions[v]);
                let n = Vec3::from(object.mesh.normals[v]);
                let to_cam = toward(p);
                let facing = n.dot(to_cam);
                if facing <= 0.05 {
                    continue;
                }
                // Hidden behind another part of the object: a ray toward the camera hits it first.
                if object.bvh.raycast(&object.mesh, p + n * size * 1e-4, to_cam).is_some() {
                    continue;
                }
                let c = sample(u, w, false);
                // Grazing surfaces take less, so the image does not smear around the silhouette.
                let grazing = ((facing - 0.05) / 0.25).clamp(0.0, 1.0);
                *slot = Some(([srgb(c[0]), srgb(c[1]), srgb(c[2])], c[3] * opacity * grazing));
            }
        }
        other => return Err(bad(format!("unknown mode {other}; use view or triplanar"))),
    }
    let object = s.scene.get_mut(id).expect("checked");
    let had = object.mesh.has_paint();
    object.mesh.ensure_paint();
    let mut edit = VertexEdit::default();
    let mut layer = object.layered().then(|| (object.paint_layers.active, Vec::new(), Vec::new()));
    for (v, t) in targets.iter().enumerate() {
        let Some((rgb, w)) = *t else { continue };
        let w = w * (1.0 - object.mask[v]);
        if w <= 1e-4 {
            continue;
        }
        let before = object.mesh.paint[v];
        let was = object.paint_target(v as u32);
        object.paint_vertex(v as u32, fsc_core::paint_from5(&[rgb[0], rgb[1], rgb[2], 0.5, 0.0]), [true, false, false, false, false], w.min(1.0), false);
        object.composite_paint(Some(&[v as u32]));
        let after = object.mesh.paint[v];
        let now = object.paint_target(v as u32);
        if after == before && was == now {
            continue;
        }
        if let (Some((_, b, a)), Some(was), Some(now)) = (layer.as_mut(), was, now) {
            b.push(was);
            a.push(now);
        }
        let p = object.mesh.positions[v];
        edit.indices.push(v as u32);
        edit.before.push(p);
        edit.after.push(p);
        edit.mask_before.push(object.mask[v]);
        edit.mask_after.push(object.mask[v]);
        edit.paint_before.push(if had { before } else { fsc_core::DEFAULT_PAINT });
        edit.paint_after.push(after);
    }
    if layer.as_ref().is_some_and(|(_, b, _)| b.len() == edit.indices.len()) {
        edit.layer = layer;
    }
    object.revision += 1;
    let n = edit.indices.len();
    let after = summary(object);
    if n > 0 {
        s.history.push(Edit::Vertices { label: "Project Image".into(), object: id, edit });
    }
    Ok(json!({"object": after, "vertices_painted": n, "image": [iw, ih]}))
}

pub(crate) fn paint_clear(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get_mut(id).expect("checked");
    if !object.mesh.has_paint() {
        return Ok(json!({"id": id, "cleared": false}));
    }
    let mut edit = VertexEdit::default();
    let mut layer = object.layered().then(|| (object.paint_layers.active, Vec::new(), Vec::new()));
    for v in 0..object.mesh.vertex_count() {
        let before = object.mesh.paint[v];
        let was = object.paint_target(v as u32);
        // With paint layers: clears the active layer (or resets the base) and recomposites.
        match (&layer, object.paint_layers.active) {
            (Some(_), Some(_)) => object.paint_vertex(v as u32, fsc_core::DEFAULT_PAINT, [true; fsc_core::PAINT_CHANNELS], 1.0, true),
            (Some(_), None) => object.set_paint_target(None, v as u32, [0.8, 0.8, 0.8, 0.5, 0.0, 1.0, 0.0, 1.0]),
            _ => object.mesh.paint[v] = fsc_core::DEFAULT_PAINT,
        }
        object.composite_paint(Some(&[v as u32]));
        let now = object.paint_target(v as u32);
        let after = object.mesh.paint[v];
        if before == after && was == now {
            continue;
        }
        if let (Some((_, b, a)), Some(was), Some(now)) = (layer.as_mut(), was, now) {
            b.push(was);
            a.push(now);
        }
        let p = object.mesh.positions[v];
        edit.indices.push(v as u32);
        edit.before.push(p);
        edit.after.push(p);
        edit.mask_before.push(object.mask[v]);
        edit.mask_after.push(object.mask[v]);
        edit.paint_before.push(before);
        edit.paint_after.push(after);
    }
    if layer.as_ref().is_some_and(|(_, b, _)| b.len() == edit.indices.len()) {
        edit.layer = layer;
    }
    object.revision += 1;
    let after = summary(object);
    if !edit.is_empty() {
        s.history.push(Edit::Vertices { label: "Clear Paint".into(), object: id, edit });
    }
    Ok(json!({"object": after, "cleared": true}))
}

pub(crate) fn brush_props() -> Value {
    let mut all = json!({
        "brush": {"type": "string", "enum": BrushKind::NAMES, "default": "draw", "description": "paint colours the surface instead of moving it, using color, roughness and metalness. relax evens the triangles without shrinking the form (smooth also wears it down); planar flattens to the plane under the first dab of the stroke; trim cuts down only what stands above each dab's plane (invert fills hollows instead); snake_hook pulls the area under the first point along the path, so give points that leave the surface (add dyntopo to keep detail along the pull); noise roughens along the normal with value noise of noise_scale; elastic drags like grab but with a smooth Kelvinlet falloff that reaches about 3 radii (ignores falloff); polish cuts what sticks out of the plane under each dab and smooths without rounding creases sharper than 40 degrees (invert fills hollows only); layer builds a raised layer capped at height per vertex over the stroke (invert recesses it); nudge slides the surface along the stroke without changing its height; stamp places one draw dab at the first point and ignores the rest; layer_reset scales down the active sculpt layer under the brush (strength 1 erases it) and reports no_active_layer when there is none; clay_strips builds like clay but over a square footprint turned along the stroke, so it lays flat-edged strips (Blender's Clay Strips)."},
        "height": {"type": "number", "minimum": 0, "maximum": 10000, "default": 0, "description": "Layer brush: the most each vertex may rise (or sink, inverted) over one stroke, in world units; a tenth of the radius when 0."},
        "spacing_compensation": {"type": "boolean", "default": false, "description": "Multiplies each dab's strength by spacing over 0.15 (at most 4), so the total effect along a path stays about the same whatever the spacing. Not used by grab, elastic, snake_hook or nudge."},
        "offset": {"type": "number", "minimum": -1, "maximum": 1, "default": 0, "description": "Lifts each dab's centre along its area normal by offset times the radius before vertices are gathered: positive touches only what stands out (peaks first), negative reaches into hollows. Not used by the drag brushes."},
        "jitter": {"type": "object", "properties": {"position": {"type": "number", "minimum": 0, "maximum": 2, "default": 0, "description": "Each dab moves along the surface by up to this many radii in a random direction."}, "radius": {"type": "number", "minimum": 0, "maximum": 1, "default": 0, "description": "Each dab's radius is scaled by 1 plus or minus up to this share."}, "strength": {"type": "number", "minimum": 0, "maximum": 1, "default": 0, "description": "Each dab's strength is scaled by 1 plus or minus up to this share."}, "seed": {"type": "integer", "minimum": 0, "default": 1, "description": "Seed of the generator; the same seed repeats the same stroke."}}, "additionalProperties": false, "description": "Random variation per dab, deterministic for a seed and usable with any stroke mode including spray. Position is not used by snake_hook and nudge; strength is not used by snake_hook."},
        "snap_radius": {"type": "number", "minimum": 0, "maximum": 10000, "default": 0, "description": "When above 0, each dab's radius (after pressure and jitter) is rounded to the nearest multiple of this in world units, at least one step."},
    });
    let filters = json!({
        "front_facing": {"type": "boolean", "default": false, "description": "Only vertices whose normal faces the viewer are affected (dot of normal and minus view_dir above 0, easing in over 0 to 0.2), so a brush does not reach through to the back of a thin part. Uses view_dir; when that is absent in sculpt_stroke, the reverse of the surface normal at the stroke's first point (the nearest vertex's normal) stands in for the view (live strokes use each sample's ray direction)."},
        "view_dir": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The direction the viewer looks along, [x, y, z], for front_facing; mirrored with each symmetry copy. Absent in sculpt_stroke, the first point's normal approximates it."},
        "connected_only": {"type": "boolean", "default": false, "description": "Only vertices joined by mesh edges to the vertex nearest the dab centre, without leaving the brush sphere, are affected, so a brush on one finger does not touch the next."},
        "protect_group": {"type": "boolean", "default": false, "description": "Only faces in the face group under the stroke's first hit are affected (each symmetry copy uses the group under its own first dab): a vertex is affected when any of its faces is in the group, and vertices on its border get half weight. Does nothing on a mesh without face groups."},
        "protect_groups": {"type": "array", "items": {"type": "integer", "minimum": 0}, "maxItems": 16, "description": "Face group ids that are protected: vertices whose faces all lie in these groups are untouched."},
        "keep_sharp": {"type": "number", "minimum": 0, "maximum": 180, "default": 0, "description": "Degrees; 0 is off. Vertices on an edge whose dihedral angle exceeds this are held (weight 0), and their neighbours take half weight. The edges are found from the mesh as it was when the stroke began."},
        "accumulate": {"type": "boolean", "default": true, "description": "When false, a vertex's displacement over one stroke is capped at the largest move any single dab gave it, so a long back-and-forth stroke does not pile up. Not used by grab, elastic, snake_hook, nudge, smooth, relax, layer or layer_reset."},
        "area_sample": {"type": "number", "minimum": 0.1, "maximum": 2, "default": 1, "description": "The radius, as a share of the brush radius, over which each dab takes its area normal and plane (draw, clay, flatten, planar, trim, polish and the other brushes that read them). A small share follows fine detail, a large one a broad average."},
        "depth": {"type": "number", "minimum": 0, "maximum": 10000, "default": 0, "description": "World units; 0 is off. Vertices more than this far below the dab's area plane (along minus the area normal) are untouched, easing over the last 20%, so a brush does not reach through a thin part. The plane is taken from the vertices under the dab whose normals agree with the nearest one's (so the far side of a thin part does not tilt it)."},
    });
    let rest = json!({
        "noise_scale": {"type": "number", "minimum": 0, "maximum": 10000, "default": 0, "description": "Noise brush: feature size in world units; a fifth of the radius when 0."},
        "alpha": {"type": "string", "description": "A grayscale alpha shaping each dab (white pushes, black does not): none, square, noise, cracks, rings, star, stripes, dots, or a name loaded with alpha_load. A one-point stroke with an alpha is a stamp."},
        "alpha_angle": {"type": "number", "minimum": -360, "maximum": 360, "default": 0, "description": "Turns the alpha, in degrees."},
        "alpha_stretch": {"type": "number", "minimum": 0.1, "maximum": 10, "default": 1, "description": "Y stretch: the alpha's v is divided by this, so above 1 the stamp grows longer along its up direction (below 1 shorter)."},
        "alpha_rotation": {"type": "number", "minimum": -360, "maximum": 360, "default": 0, "description": "Degrees added on top of alpha_angle and, with the roll stroke, on top of the along-stroke direction."},
        "alpha_tile": {"type": "integer", "minimum": 1, "maximum": 16, "default": 1, "description": "The alpha repeats this many times across the dab in each direction."},
        "alpha_mid": {"type": "number", "minimum": 0, "maximum": 1, "default": 0, "description": "Mid value of the alpha: values above it raise and values below it carve (rescaled to -1..1), so a grey-is-zero alpha does both. Carving applies to draw, inflate, noise and stamp; the other brushes treat below-mid as 0. 0 is off."},
        "alpha_invert": {"type": "boolean", "default": false, "description": "Inverts the alpha (1 minus its value) before alpha_mid."},
        "alpha_up": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The world direction the alpha's top faces (project it onto the surface); +y when omitted. The window sends the view's up."},
        "stroke": {"type": "string", "enum": ["dots", "roll", "spray"], "default": "dots", "description": "dots keeps the alpha's orientation; roll turns it along the stroke's direction (chains, stitches, scales along a line); spray scatters each dab up to spray radii off the path, turned at random and at a random share of the strength (pores, gravel, speckle). Spray is seeded, so the same stroke repeats exactly."},
        "spray": {"type": "number", "minimum": 0.05, "maximum": 2, "default": 0.6, "description": "Spray stroke: how far dabs scatter from the path, in radii."},
        "falloff_curve": {"type": "array", "items": {"type": "number", "minimum": 0, "maximum": 1}, "minItems": 2, "maxItems": 64, "description": "A drawn falloff instead of a named one: weights evenly spaced from the centre (first) to the rim (last), read linearly between. [1, 1, 1, 0] is a flat-topped brush with a hard edge; [0, 1, 0] a ring."},
        "pressure_gamma": {"type": "number", "minimum": 0.2, "maximum": 5, "default": 1, "description": "Pen pressure curve: pressure is raised to this power (above 1 needs a firmer press, below 1 a lighter one). Per-point pressure is the fourth number of a point."},
        "pressure_strength": {"type": "boolean", "default": true, "description": "Pressure scales strength."},
        "pressure_size": {"type": "boolean", "default": false, "description": "Pressure scales the radius (down to a tenth)."},
        "color": {"type": ["string", "array"], "description": "Paint brush: #rrggbb or [r, g, b] from 0 to 1."},
        "roughness": {"type": "number", "minimum": 0, "maximum": 1, "description": "Paint brush: paint this roughness (only the channels given are painted)."},
        "metalness": {"type": "number", "minimum": 0, "maximum": 1, "description": "Paint brush: paint this metalness."},
        "opacity": {"type": "number", "minimum": 0, "maximum": 1, "description": "Paint brush: paint this opacity (1 solid, 0 clear)."},
        "emissive": {"type": "number", "minimum": 0, "maximum": 1, "description": "Paint brush: paint this glow in the paint colour."},
        "target": {"type": "string", "enum": ["vertex", "texture"], "default": "vertex", "description": "Paint brush only: vertex paints the mesh's vertex paint; texture paints the object's texture per texel in UV space (texture_create first), so a stroke can be finer than the mesh. Refused with bad_arguments for any other brush. Invert erases toward the default paint in the chosen channels."},
        "radius": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "default": 0.2, "description": "World units; compare with the object's bounds from get_scene."},
        "strength": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.5},
        "invert": {"type": "boolean", "default": false, "description": "Carve instead of build, deflate, unpinch, erase mask."},
        "falloff": {"type": "string", "enum": Falloff::NAMES, "default": "smooth"},
        "focal_shift": {"type": "number", "minimum": -100, "maximum": 100, "default": 0, "description": "Focal Shift over the falloff (or falloff_curve): below 0 full strength reaches further out (-100 a crisp tip with no falloff), above 0 the falloff starts sooner (100 narrows the tip toward a point)."},
        "spacing": {"type": "number", "minimum": 0.02, "maximum": 2, "default": 0.15, "description": "Distance between dabs as a fraction of the radius."},
        "lazy": {"type": "number", "minimum": 0, "maximum": 0.95, "default": 0, "description": "Lazy rope: the brush trails the path on a rope of lazy times three radii and moves only when it pulls taut, which steadies a line but stops short of the path's end by the rope length."},
        "symmetry": {"type": "object", "properties": {"x": {"type": "boolean"}, "y": {"type": "boolean"}, "z": {"type": "boolean"}, "radial": {"type": "integer", "minimum": 1, "maximum": 150}, "local": {"type": "boolean", "default": false, "description": "Mirror and turn about the object's own centre (the centre of its bounds when the stroke starts) instead of the world origin, so an object moved off the origin still sculpts symmetrically (Local symmetry). The planes keep the world's directions."}}, "additionalProperties": false},
        "dyntopo": {"type": ["number", "boolean"], "description": "Dynamic topology: split edges longer than this fraction of the radius (0.05 to 1; true means 0.25) under the brush as it moves, adding detail where you sculpt, and collapse edges under 0.4 of that length. Off when omitted. Not used by grab or mask, nor on objects with subdivision levels."},
        "dyntopo_size": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "description": "Constant detail: the target edge length in world units, whatever the radius. Turns dynamic topology on and overrides the dyntopo fraction."},
        "dyntopo_mode": {"type": "string", "enum": fsc_sculpt::DyntopoMode::NAMES, "default": "balanced", "description": "With dyntopo: speed refines once per dab and does not even out the new triangles (fastest, rougher); balanced refines twice and evens twice; quality four times each (smoothest, slowest)."},
        "dyntopo_method": {"type": "string", "enum": fsc_sculpt::DyntopoMethod::NAMES, "default": "both", "description": "With dyntopo: subdivide only splits long edges (detail is only ever added), collapse only merges short ones (the brush thins out over-dense areas), both does each."},
        "dyntopo_protect_mask": {"type": "boolean", "default": false, "description": "With dyntopo: never split an edge with a masked end, so masked areas keep their triangles as well as their shape."}
    });
    let out = all.as_object_mut().expect("object");
    for part in [filters, rest] {
        out.extend(part.as_object().expect("object").clone());
    }
    all
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

/// With local symmetry, the mirror planes and the radial axis pass through the object's centre (its bounds' centre now)
/// rather than the world origin.
pub(crate) fn local_symmetry(settings: &mut BrushSettings, object: &fsc_core::Object) {
    if settings.symmetry.local {
        settings.symmetry.center = object.mesh.bounds().center().to_array();
    }
}

pub(crate) fn sculpt_stroke(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let mut settings = parse_brush(args)?;
    local_symmetry(&mut settings, s.scene.get(id).expect("checked"));
    let alpha = stroke_alpha(s, args)?;
    let points = args.get("points").and_then(Value::as_array).ok_or_else(|| bad("points is required"))?;
    let mut parsed = Vec::with_capacity(points.len());
    for p in points {
        let a = p.as_array().ok_or_else(|| bad("each point is [x, y, z] or [x, y, z, pressure]"))?;
        let n = |i: usize| a.get(i).and_then(Value::as_f64).map(|x| x as f32);
        let (Some(x), Some(y), Some(z)) = (n(0), n(1), n(2)) else { return Err(bad("each point needs three numbers")) };
        parsed.push((Vec3::new(x, y, z), n(3).unwrap_or(1.0).clamp(0.0, 1.0)));
    }
    let object = s.scene.get_mut(id).expect("checked");
    if settings.texture && object.live_texture().is_none() {
        return Err(no_texture());
    }
    if settings.dyntopo.is_some() && object.multires.is_some() {
        return Err(CommandError::new("has_levels", "dynamic topology cannot run on an object with subdivision levels; sculpt without dyntopo, or multires_delete_lower and multires_delete_higher first"));
    }
    if settings.dyntopo.is_some() && !object.layers.list.is_empty() {
        return Err(CommandError::new("has_layers", "dynamic topology cannot run on an object with sculpt layers; sculpt without dyntopo, or layer_apply or layer_delete them first"));
    }
    let mut stroke = Stroke::new(settings);
    stroke.set_alpha(alpha);
    if settings.kind.drags() {
        let (first, _) = parsed[0];
        let (last, pressure) = *parsed.last().expect("non-empty");
        stroke.grab_between(object, first, last, pressure);
    } else {
        for (p, pressure) in parsed {
            stroke.sample_point(object, p, pressure);
        }
    }
    let dabs = stroke.dabs;
    let no_layer = stroke.no_active_layer;
    let result = stroke.finish(object);
    let after = summary(object);
    let label = format!("{} Stroke", capital(BrushKind::NAMES[settings.kind as usize]));
    let moved = push_stroke(s, id, label, result);
    let unchanged = moved.as_u64() == Some(0);
    let mut reply = json!({"object": after, "dabs": dabs, "vertices_changed": moved});
    if settings.texture {
        reply["vertices_changed"] = json!(0);
        reply["texels_changed"] = moved;
    }
    if no_layer {
        reply["no_active_layer"] = json!(true);
    }
    // A stroke that changes nothing is almost always aimed wrong (points off the surface, or another object than
    // meant when id is omitted); it is not an error, but the reply says so, so an agent does not take it as done.
    if !settings.texture && !no_layer && unchanged {
        let name = after.get("name").and_then(Value::as_str).unwrap_or("the object").to_string();
        reply["note"] = json!(format!(
            "the stroke changed nothing on {name} (id {id}): no vertex lies within the radius of its points, or every one there is masked or hidden. Check the id (the active object is used when it is omitted) and take the points from pick on its surface."
        ));
    }
    Ok(reply)
}

pub(crate) fn no_texture() -> CommandError {
    CommandError::new("no_texture", "the object has no texture; run texture_create first (it needs UVs from uv_unwrap)")
}

pub fn push_stroke(s: &mut Session, id: ObjectId, label: String, result: StrokeEdit) -> Value {
    match result {
        StrokeEdit::Texels(edit) => {
            let n = edit.texels.len();
            s.history.push(Edit::Texels { label, object: id, edit });
            json!(n)
        }
        StrokeEdit::Vertices(edit) => {
            let n = edit.indices.len();
            push_vertices(s, id, label, edit);
            json!(n)
        }
        StrokeEdit::Topology { mesh, mask } => {
            s.history.push(Edit::Topology { label, object: id, mesh, mask });
            json!("topology changed")
        }
        StrokeEdit::Nothing => json!(0),
    }
}

/// Pushes a vertex edit, recording its shape, paint and mask changes into the active sculpt layer when there is one.
pub(crate) fn push_vertices(s: &mut Session, id: ObjectId, label: String, edit: VertexEdit) {
    let layer = s.scene.get_mut(id).and_then(|o| {
        let at = o.layers.active?;
        edit.record_into_layer(o, at, 1.0);
        Some(at)
    });
    match layer {
        Some(layer) => s.history.push(Edit::LayerStroke { label, object: id, layer, edit }),
        None => s.history.push(Edit::Vertices { label, object: id, edit }),
    }
}

fn capital(s: &str) -> String {
    let mut c = s.replace('_', " ");
    if let Some(f) = c.get_mut(0..1) {
        f.make_ascii_uppercase();
    }
    c
}

pub(crate) fn id_opt_schema() -> Value {
    json!({"type": "object", "properties": {"id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."}}, "required": [], "additionalProperties": false})
}

pub(crate) fn mask_edit(s: &mut Session, args: &Value, label: &str, f: impl Fn(&fsc_core::Object, &[f32]) -> Vec<f32>) -> Result<Value, CommandError> {
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

pub(crate) fn alpha_load_schema() -> Value {
    json!({"type": "object", "properties": {
        "name": {"type": "string", "minLength": 1, "maxLength": 64, "description": "The name brushes use for it in their alpha setting; loading the same name again replaces it."},
        "png_base64": {"type": "string", "description": "The alpha as a base64 PNG; white stamps fully, black not at all (colour is averaged to gray)."},
        "path": {"type": "string", "description": "A PNG file on the engine's machine, instead of png_base64."},
        "size": {"type": "integer", "minimum": 16, "maximum": 1024, "description": "The square resolution the alpha is resampled to (256 by default)."}
    }, "required": ["name"], "additionalProperties": false})
}

/// Loads a gray PNG as a named brush alpha for sculpt_stroke and the window's strokes.
pub(crate) fn alpha_load(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    use base64::Engine as _;
    let name = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad("name is required"))?;
    if name == "none" || fsc_sculpt::Alpha::BUILTIN.contains(&name) {
        return Err(bad(format!("{name} is a built-in alpha; choose another name")));
    }
    let bytes = match (args.get("png_base64").and_then(Value::as_str), args.get("path").and_then(Value::as_str)) {
        (Some(b), None) => base64::engine::general_purpose::STANDARD.decode(b.trim()).map_err(|e| bad(format!("png_base64 is not base64: {e}")))?,
        (None, Some(p)) => std::fs::read(p).map_err(|e| CommandError::new("read_failed", format!("could not read {p}: {e}")))?,
        _ => return Err(bad("give exactly one of png_base64 or path")),
    };
    let size = args.get("size").and_then(Value::as_u64).unwrap_or(256).clamp(16, 1024) as usize;
    let (w, h, gray) = fsc_paint::png_gray(&bytes).map_err(bad)?;
    let alpha = fsc_sculpt::Alpha::from_pixels(w, h, &gray, size).map_err(bad)?;
    s.alphas.insert(name.to_string(), std::sync::Arc::new(alpha));
    let mut loaded: Vec<&str> = s.alphas.keys().map(String::as_str).collect();
    loaded.sort_unstable();
    Ok(json!({"name": name, "source_width": w, "source_height": h, "size": size, "builtin": fsc_sculpt::Alpha::BUILTIN, "loaded": loaded}))
}
