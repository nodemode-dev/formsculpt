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
    let falloff = match args.get("falloff_curve") {
        None | Some(Value::Null) => Falloff::parse(falloff_name).ok_or_else(|| bad(format!("unknown falloff {falloff_name}; use one of {}", Falloff::NAMES.join(", "))))?,
        Some(Value::Array(a)) if (2..=64).contains(&a.len()) => {
            let w: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
            Falloff::curve(&w).ok_or_else(|| bad("falloff_curve holds numbers from 0 to 1"))?
        }
        Some(_) => return Err(bad("falloff_curve is 2 to 64 weights from the centre to the rim")),
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
    let base = fsc_core::DEFAULT_PAINT;
    let c = color.unwrap_or([0.8, 0.2, 0.2]);
    let paint = [c[0], c[1], c[2], rough.unwrap_or(base[3]), metal.unwrap_or(base[4])];
    let paint_channels = if color.is_none() && rough.is_none() && metal.is_none() { [true, false, false] } else { [color.is_some(), rough.is_some(), metal.is_some()] };
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
        roll: stroke_mode == "roll",
        spray: if stroke_mode == "spray" { f(args, "spray", 0.6, 0.05, 2.0)? } else { 0.0 },
        pressure_gamma: f(args, "pressure_gamma", 1.0, 0.2, 5.0)?,
        pressure_strength: args.get("pressure_strength").and_then(Value::as_bool).unwrap_or(true),
        pressure_size: args.get("pressure_size").and_then(Value::as_bool).unwrap_or(false),
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
            "metalness": {"type": "number", "minimum": 0, "maximum": 1}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn paint_fill(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let color = parse_color(args.get("color"))?;
    let rough = args.get("roughness").and_then(Value::as_f64).map(|x| x.clamp(0.0, 1.0) as f32);
    let metal = args.get("metalness").and_then(Value::as_f64).map(|x| x.clamp(0.0, 1.0) as f32);
    if color.is_none() && rough.is_none() && metal.is_none() {
        return Err(bad("give color, roughness or metalness"));
    }
    let object = s.scene.get_mut(id).expect("checked");
    let had = object.mesh.has_paint();
    object.mesh.ensure_paint();
    let mut edit = VertexEdit::default();
    for v in 0..object.mesh.vertex_count() {
        let w = 1.0 - object.mask[v];
        if w <= 0.0 {
            continue;
        }
        let before = object.mesh.paint[v];
        let mut after = before;
        if let Some(c) = color {
            for k in 0..3 {
                after[k] += (c[k] - after[k]) * w;
            }
        }
        if let Some(r) = rough {
            after[3] += (r - after[3]) * w;
        }
        if let Some(m) = metal {
            after[4] += (m - after[4]) * w;
        }
        if after == before {
            continue;
        }
        object.mesh.paint[v] = after;
        let p = object.mesh.positions[v];
        edit.indices.push(v as u32);
        edit.before.push(p);
        edit.after.push(p);
        edit.mask_before.push(object.mask[v]);
        edit.mask_after.push(object.mask[v]);
        edit.paint_before.push(if had { before } else { fsc_core::DEFAULT_PAINT });
        edit.paint_after.push(after);
    }
    object.revision += 1;
    let n = edit.indices.len();
    let after = summary(object);
    if n > 0 {
        s.history.push(Edit::Vertices { label: "Fill Paint".into(), object: id, edit });
    }
    Ok(json!({"object": after, "vertices_painted": n}))
}

pub(crate) fn paint_clear(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get_mut(id).expect("checked");
    if !object.mesh.has_paint() {
        return Ok(json!({"id": id, "cleared": false}));
    }
    let mut edit = VertexEdit::default();
    for v in 0..object.mesh.vertex_count() {
        let before = object.mesh.paint[v];
        if before == fsc_core::DEFAULT_PAINT {
            continue;
        }
        let p = object.mesh.positions[v];
        object.mesh.paint[v] = fsc_core::DEFAULT_PAINT;
        edit.indices.push(v as u32);
        edit.before.push(p);
        edit.after.push(p);
        edit.mask_before.push(object.mask[v]);
        edit.mask_after.push(object.mask[v]);
        edit.paint_before.push(before);
        edit.paint_after.push(fsc_core::DEFAULT_PAINT);
    }
    object.revision += 1;
    let after = summary(object);
    if !edit.is_empty() {
        s.history.push(Edit::Vertices { label: "Clear Paint".into(), object: id, edit });
    }
    Ok(json!({"object": after, "cleared": true}))
}

pub(crate) fn brush_props() -> Value {
    json!({
        "brush": {"type": "string", "enum": BrushKind::NAMES, "default": "draw", "description": "paint colours the surface instead of moving it, using color, roughness and metalness. relax evens the triangles without shrinking the form (smooth also wears it down); planar flattens to the plane under the first dab of the stroke; trim cuts down only what stands above each dab's plane (invert fills hollows instead); snake_hook pulls the area under the first point along the path, so give points that leave the surface (add dyntopo to keep detail along the pull); noise roughens along the normal with value noise of noise_scale."},
        "noise_scale": {"type": "number", "minimum": 0, "maximum": 10000, "default": 0, "description": "Noise brush: feature size in world units; a fifth of the radius when 0."},
        "alpha": {"type": "string", "description": "A grayscale alpha shaping each dab (white pushes, black does not): none, square, noise, cracks, rings, star, stripes, dots, or a name loaded with alpha_load. A one-point stroke with an alpha is a stamp."},
        "alpha_angle": {"type": "number", "minimum": -360, "maximum": 360, "default": 0, "description": "Turns the alpha, in degrees."},
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
        "radius": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "default": 0.2, "description": "World units; compare with the object's bounds from get_scene."},
        "strength": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.5},
        "invert": {"type": "boolean", "default": false, "description": "Carve instead of build, deflate, unpinch, erase mask."},
        "falloff": {"type": "string", "enum": Falloff::NAMES, "default": "smooth"},
        "spacing": {"type": "number", "minimum": 0.02, "maximum": 2, "default": 0.15, "description": "Distance between dabs as a fraction of the radius."},
        "lazy": {"type": "number", "minimum": 0, "maximum": 0.95, "default": 0, "description": "Lazy rope: the brush trails the path on a rope of lazy times three radii and moves only when it pulls taut, which steadies a line but stops short of the path's end by the rope length."},
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
    if settings.dyntopo.is_some() && object.multires.is_some() {
        return Err(CommandError::new("has_levels", "dynamic topology cannot run on an object with subdivision levels; sculpt without dyntopo, or multires_delete_lower and multires_delete_higher first"));
    }
    if settings.dyntopo.is_some() && !object.layers.list.is_empty() {
        return Err(CommandError::new("has_layers", "dynamic topology cannot run on an object with sculpt layers; sculpt without dyntopo, or layer_apply or layer_delete them first"));
    }
    let mut stroke = Stroke::new(settings);
    stroke.set_alpha(alpha);
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
            let layer = s.scene.get_mut(id).and_then(|o| {
                let at = o.layers.active?;
                o.record_into_layer(at, &edit.indices, &edit.before, &edit.after, 1.0);
                Some(at)
            });
            match layer {
                Some(layer) => s.history.push(Edit::LayerStroke { label, object: id, layer, edit }),
                None => s.history.push(Edit::Vertices { label, object: id, edit }),
            }
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
