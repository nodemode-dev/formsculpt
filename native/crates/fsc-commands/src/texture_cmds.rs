use base64::Engine as _;
use fsc_core::texture::{color_pixels, emissive_pixels, mr_pixels, rasterize, PAD, TEXTURE_MAX, TEXTURE_MIN};
use fsc_core::{Maps, Object, ObjectId, Paint, Texture, TextureEdit, DEFAULT_PAINT, PAINT_CHANNELS, PAINT_LEN};
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::{id_opt_schema, no_texture, target};
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn texture_create_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "size": {"type": "integer", "minimum": TEXTURE_MIN, "maximum": TEXTURE_MAX, "default": 1024, "description": "Texels per side (square)."},
            "from": {"type": "string", "enum": ["vertex", "blank"], "default": "vertex", "description": "vertex bakes the current vertex paint into the texels (each UV triangle rasterised with its corner paint interpolated, with a few texels of padding past the seams); blank starts from the default grey."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Makes the object's texture, replacing any it has; one undo step. Refuses with no_uvs when the object is not unwrapped.
pub(crate) fn texture_create(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let size = match args.get("size") {
        None | Some(Value::Null) => 1024,
        Some(v) => v.as_u64().filter(|n| (TEXTURE_MIN as u64..=TEXTURE_MAX as u64).contains(n)).ok_or_else(|| bad(format!("size must be an integer from {TEXTURE_MIN} to {TEXTURE_MAX}")))? as usize,
    };
    let from = args.get("from").and_then(Value::as_str).unwrap_or("vertex");
    if !["vertex", "blank"].contains(&from) {
        return Err(bad("from is vertex or blank"));
    }
    let object = s.scene.get_mut(id).expect("checked");
    if !object.has_uvs() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)"));
    }
    let mut tex = if from == "vertex" {
        Texture::from_vertex(&object.mesh, size)
    } else {
        let mut t = Texture::blank(size);
        t.mark_interior(&object.mesh);
        t
    };
    object.revision += 1;
    tex.revision = object.revision;
    let covered = tex.interior.iter().filter(|&&i| i).count() as f64 / (size * size) as f64;
    let before = object.texture.replace(tex);
    let after = summary(object);
    s.history.push(Edit::TextureState { label: "Create Texture".into(), object: id, texture: before });
    Ok(json!({"object": after, "size": size, "from": from, "covered": covered}))
}

/// Removes the object's texture; one undo step.
pub(crate) fn texture_clear(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get_mut(id).expect("checked");
    let Some(old) = object.texture.take() else { return Err(no_texture()) };
    object.revision += 1;
    let after = summary(object);
    s.history.push(Edit::TextureState { label: "Clear Texture".into(), object: id, texture: Some(old) });
    Ok(json!({"object": after}))
}

pub(crate) fn texture_clear_schema() -> Value {
    id_opt_schema()
}

pub(crate) fn get_texture_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "channel": {"type": "string", "enum": ["color", "roughness_metal", "emissive"], "default": "color", "description": "color is RGB with opacity as alpha; roughness_metal has roughness in green and metalness in blue; emissive is the glow colour (paint colour times emissive)."},
            "size": {"type": "integer", "minimum": 1, "maximum": TEXTURE_MAX, "description": "Reduce the picture to this many texels a side by averaging (never above the texture's own size); for previews."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// The texture as a PNG (png_base64) with its size, the texture's own size and its revision, which rises on every change.
pub(crate) fn get_texture(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let channel = args.get("channel").and_then(Value::as_str).unwrap_or("color");
    if !["color", "roughness_metal", "emissive"].contains(&channel) {
        return Err(bad("channel is color, roughness_metal or emissive"));
    }
    let object = s.scene.get(id).expect("checked");
    let tex = object.live_texture().ok_or_else(no_texture)?;
    let size = match args.get("size") {
        None | Some(Value::Null) => tex.size,
        Some(v) => v.as_u64().filter(|n| (1..=TEXTURE_MAX as u64).contains(n)).ok_or_else(|| bad(format!("size must be an integer from 1 to {TEXTURE_MAX}")))?.min(tex.size as u64) as usize,
    };
    let texels = tex.downsampled(size);
    let (pixels, glows) = match channel {
        "color" => (color_pixels(&texels), false),
        "roughness_metal" => (mr_pixels(&texels), false),
        _ => emissive_pixels(&texels),
    };
    let png = fsc_paint::png_rgba(size, &pixels).map_err(|e| CommandError::new("encode_failed", e))?;
    Ok(json!({
        "id": id, "channel": channel, "mime": "image/png", "size": size, "texture_size": tex.size, "revision": tex.revision, "glows": glows,
        "png_base64": base64::engine::general_purpose::STANDARD.encode(&png)
    }))
}

pub(crate) fn pick_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A point [x, y, z] in object space on or near the surface; the nearest surface point is used."},
            "ray": {"type": "object", "properties": {"origin": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}, "dir": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}}, "required": ["origin", "dir"], "additionalProperties": false, "description": "A ray in object space instead of point; the first surface it hits is used (hit false when it misses)."}
        },
        "required": [],
        "additionalProperties": false
    })
}

fn vec3_of(v: &Value, what: &str) -> Result<Vec3, CommandError> {
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| bad(format!("{what} is [x, y, z]")))?;
    let n = |i: usize| a[i].as_f64().filter(|x| x.is_finite()).map(|x| x as f32).ok_or_else(|| bad(format!("{what} is [x, y, z] of numbers")));
    Ok(Vec3::new(n(0)?, n(1)?, n(2)?))
}

fn hex(p: &Paint) -> String {
    let b = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(p[0]), b(p[1]), b(p[2]))
}

/// The eyedropper: the paint at a surface point or ray hit, from the texture when there is one (at the hit's UV), else from vertex paint by barycentric weights.
pub(crate) fn pick_paint(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get(id).expect("checked");
    let m = &object.mesh;
    let (triangle, bary, point, distance) = match (args.get("point").filter(|v| !v.is_null()), args.get("ray").filter(|v| !v.is_null())) {
        (Some(p), None) => {
            let p = vec3_of(p, "point")?;
            let reach = m.bounds().extent().length() + p.distance(m.bounds().center());
            let (q, t, bary, d) = fsc_topo::nearest_feature(&object.bvh, m, p, reach).ok_or_else(|| CommandError::new("no_hit", "the object has no surface"))?;
            (t, bary, q, Some(d))
        }
        (None, Some(r)) => {
            let origin = vec3_of(&r["origin"], "ray.origin")?;
            let dir = vec3_of(&r["dir"], "ray.dir")?.try_normalize().ok_or_else(|| bad("ray.dir must not be zero"))?;
            match object.bvh.raycast(m, origin, dir) {
                Some(h) => (h.triangle, [1.0 - h.u - h.v, h.u, h.v], h.point, None),
                None => return Ok(json!({"id": id, "hit": false})),
            }
        }
        _ => return Err(bad("give point or ray")),
    };
    let ids = m.triangle(triangle as usize);
    let (paint, source, uv) = match object.live_texture() {
        Some(tex) => {
            let uv: [f32; 2] = std::array::from_fn(|c| (0..3).map(|k| m.uvs[3 * triangle as usize + k][c] * bary[k]).sum());
            (tex.at(uv), "texture", Some(uv))
        }
        None if m.has_paint() => (std::array::from_fn(|c| (0..3).map(|k| m.paint[ids[k] as usize][c] * bary[k]).sum()), "vertex", None),
        None => (DEFAULT_PAINT, "vertex", None),
    };
    Ok(json!({
        "id": id, "hit": true, "source": source, "color": hex(&paint), "rgb": [paint[0], paint[1], paint[2]],
        "roughness": paint[3], "metalness": paint[4], "opacity": paint[5], "emissive": paint[6],
        "triangle": triangle, "barycentric": bary, "point": point.to_array(), "uv": uv, "distance": distance
    }))
}

/// Fills every texel of an object's texture with `fill` in the channels chosen, protecting masked and hidden areas in proportion to the mask.
pub(crate) fn fill_texture(s: &mut Session, id: ObjectId, fill: Paint, channels: [bool; PAINT_CHANNELS]) -> Result<Value, CommandError> {
    let object = s.scene.get_mut(id).expect("checked");
    if object.live_texture().is_none() {
        return Err(no_texture());
    }
    let mut tex = object.texture.take().expect("checked");
    let m = &object.mesh;
    let n = tex.size * tex.size;
    let hid = m.has_hidden();
    let masked = hid || object.mask.iter().any(|&x| x > 0.0);
    let mut weight: Vec<f32> = if masked { vec![0.0; n] } else { Vec::new() };
    if masked {
        let free: Vec<f32> = (0..m.vertex_count() as u32)
            .map(|v| if hid && object.topology.faces_of(v).iter().all(|&t| m.hidden[t as usize]) { 0.0 } else { 1.0 - object.mask[v as usize] })
            .collect();
        let mut hits = Vec::new();
        for t in 0..m.triangle_count() {
            if m.face_hidden(t) {
                continue;
            }
            hits.clear();
            rasterize(m, tex.size, t, PAD, None, &mut hits);
            let ids = m.triangle(t);
            for h in &hits {
                let at = h.at as usize;
                if !h.inside && tex.interior[at] {
                    continue;
                }
                let w: f32 = (0..3).map(|k| free[ids[k] as usize] * h.bary[k]).sum();
                weight[at] = weight[at].max(w);
            }
        }
    }
    let mut edit = TextureEdit::default();
    for (at, texel) in tex.texels.iter_mut().enumerate() {
        let w = if masked { weight[at] } else { 1.0 };
        if w <= 0.0 {
            continue;
        }
        let before = *texel;
        let mut now = before;
        for c in 0..PAINT_LEN {
            if channels[fsc_core::paint_channel(c)] {
                now[c] += (fill[c] - now[c]) * w;
            }
        }
        if now != before {
            edit.texels.push(at as u32);
            edit.before.push(before);
            edit.after.push(now);
            *texel = now;
        }
    }
    let painted = edit.texels.len();
    if painted > 0 {
        tex.revision += 1;
        object.revision += 1;
    }
    object.texture = Some(tex);
    let after = summary(object);
    if painted > 0 {
        s.history.push(Edit::Texels { label: "Fill Paint".into(), object: id, edit });
    }
    Ok(json!({"object": after, "vertices_painted": 0, "texels_painted": painted}))
}

/// The maps a glTF export uses for an object with a texture: colour, roughness-metal and glow from the texture, the normal map and displacement from any bake.
pub(crate) fn export_maps(o: &Object) -> Option<Maps> {
    let t = o.live_texture()?;
    let png = |px: &[[u8; 4]]| fsc_paint::png_rgba(t.size, px).ok();
    let (glow, glows) = emissive_pixels(&t.texels);
    let baked = o.maps.as_ref();
    Some(Maps {
        size: t.size,
        color_png: png(&color_pixels(&t.texels))?,
        mr_png: png(&mr_pixels(&t.texels))?,
        normal_png: baked.map(|b| b.normal_png.clone()).unwrap_or_default(),
        displacement_exr: baked.map(|b| b.displacement_exr.clone()).unwrap_or_default(),
        emissive_png: if glows { png(&glow)? } else { Vec::new() },
        source: "texture".into(),
        translucent: t.texels.iter().any(|p| p[5] < 0.999),
    })
}
