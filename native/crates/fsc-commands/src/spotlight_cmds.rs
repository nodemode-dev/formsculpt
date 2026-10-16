use std::sync::Arc;

use base64::Engine as _;
use fsc_core::texture::{rasterize, PAD};
use fsc_sculpt::VertexEdit;
use glam::{Mat4, Vec3, Vec4};
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::{no_texture, push_vertices, target, target_texture};
use crate::{CommandError, Session};

const MAX_IMAGES: usize = 16;

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

/// One image on the Spotlight: its pixels and where it sits over the view (centre as fractions of the view, y down;
/// height as a fraction of the view's height; turn in degrees clockwise on screen).
#[derive(Clone)]
pub struct SpotImage {
    pub id: u32,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub pixels: Arc<Vec<[u8; 4]>>,
    pub center: [f32; 2],
    pub size: f32,
    pub angle: f32,
    pub opacity: f32,
    pub tile: bool,
    pub flip: bool,
}

impl std::fmt::Debug for SpotImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SpotImage({} {} {}x{})", self.id, self.name, self.width, self.height)
    }
}

/// The images held on the Spotlight: view state, like ZBrush's, kept out of the undo history and out of projects.
#[derive(Debug, Default)]
pub struct Spotlight {
    pub images: Vec<SpotImage>,
    pub active: Option<u32>,
    pub revision: u64,
    next: u32,
}

impl SpotImage {
    fn info(&self, active: bool) -> Value {
        json!({
            "image": self.id, "name": self.name, "width": self.width, "height": self.height, "center": self.center,
            "size": self.size, "angle": self.angle, "opacity": self.opacity, "tile": self.tile, "flip": self.flip, "active": active
        })
    }

    /// The image point under a view pixel (sx, sy) of a view w by h pixels, as u, v in 0..1 (v down); None off the image.
    fn local(&self, sx: f32, sy: f32, w: f32, h: f32) -> Option<[f32; 2]> {
        let ih = (self.size * h).max(1e-6);
        let iw = ih * self.width as f32 / self.height as f32;
        let (dx, dy) = (sx - self.center[0] * w, sy - self.center[1] * h);
        let (sin, cos) = (-self.angle.to_radians()).sin_cos();
        let mut u = (dx * cos - dy * sin) / iw + 0.5;
        let v = (dx * sin + dy * cos) / ih + 0.5;
        if self.flip {
            u = 1.0 - u;
        }
        if self.tile {
            return Some([u.rem_euclid(1.0), v.rem_euclid(1.0)]);
        }
        ((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)).then_some([u, v])
    }

    /// Bilinear RGBA in 0..1 at u, v.
    fn sample(&self, [u, v]: [f32; 2]) -> [f32; 4] {
        let (iw, ih) = (self.width, self.height);
        let x = u.clamp(0.0, 1.0) * (iw as f32 - 1.0);
        let y = v.clamp(0.0, 1.0) * (ih as f32 - 1.0);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(iw - 1), (y0 + 1).min(ih - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let px = |x: usize, y: usize| self.pixels[y * iw + x];
        std::array::from_fn(|c| {
            let a = px(x0, y0)[c] as f32 * (1.0 - fx) + px(x1, y0)[c] as f32 * fx;
            let b = px(x0, y1)[c] as f32 * (1.0 - fx) + px(x1, y1)[c] as f32 * fx;
            (a * (1.0 - fy) + b * fy) / 255.0
        })
    }
}

fn num(args: &Value, key: &str, lo: f32, hi: f32) -> Result<Option<f32>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().map(|x| x as f32).filter(|x| x.is_finite() && (lo..=hi).contains(x)).map(Some).ok_or_else(|| bad(format!("{key} must be a number from {lo} to {hi}"))),
    }
}

fn flag(args: &Value, key: &str) -> Result<Option<bool>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(bad(format!("{key} is true or false"))),
    }
}

fn pair(args: &Value, key: &str) -> Result<Option<[f32; 2]>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() == 2 => {
            let n = |i: usize| a[i].as_f64().map(|x| x as f32).filter(|x| x.is_finite());
            match (n(0), n(1)) {
                (Some(x), Some(y)) => Ok(Some([x, y])),
                _ => Err(bad(format!("{key} is [x, y] of numbers"))),
            }
        }
        Some(_) => Err(bad(format!("{key} is [x, y]"))),
    }
}

fn placement_props() -> serde_json::Map<String, Value> {
    let v = json!({
        "center": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "The image's centre over the view as [x, y] fractions of the view's width and height, y down; [0.5, 0.5] is the middle."},
        "size": {"type": "number", "minimum": 0.01, "maximum": 20, "description": "The image's height as a fraction of the view's height (its width follows its aspect)."},
        "angle": {"type": "number", "minimum": -360, "maximum": 360, "description": "Turn in degrees, clockwise on screen."},
        "opacity": {"type": "number", "minimum": 0, "maximum": 1, "description": "How strongly the image paints or sculpts (its own alpha multiplies this); the window also draws it at this opacity."},
        "tile": {"type": "boolean", "description": "Repeat the image across the whole view instead of one copy."},
        "flip": {"type": "boolean", "description": "Mirror the image left to right."}
    });
    v.as_object().cloned().expect("object")
}

fn set_placement(img: &mut SpotImage, args: &Value) -> Result<(), CommandError> {
    if let Some(c) = pair(args, "center")? {
        img.center = c;
    }
    if let Some(x) = num(args, "size", 0.01, 20.0)? {
        img.size = x;
    }
    if let Some(x) = num(args, "angle", -360.0, 360.0)? {
        img.angle = x;
    }
    if let Some(x) = num(args, "opacity", 0.0, 1.0)? {
        img.opacity = x;
    }
    if let Some(b) = flag(args, "tile")? {
        img.tile = b;
    }
    if let Some(b) = flag(args, "flip")? {
        img.flip = b;
    }
    Ok(())
}

fn image_id(s: &Session, args: &Value) -> Result<u32, CommandError> {
    let id = match args.get("image").and_then(Value::as_u64) {
        Some(i) => i as u32,
        None => s.spotlight.active.ok_or_else(|| CommandError::new("no_image", "the Spotlight holds no image; spotlight_add first"))?,
    };
    if !s.spotlight.images.iter().any(|i| i.id == id) {
        return Err(CommandError::new("not_found", format!("no Spotlight image {id}; spotlight_list lists them")));
    }
    Ok(id)
}

fn listing(s: &Session) -> Value {
    let sp = &s.spotlight;
    json!({"images": sp.images.iter().map(|i| i.info(sp.active == Some(i.id))).collect::<Vec<_>>(), "active": sp.active, "revision": sp.revision})
}

pub(crate) fn add_schema() -> Value {
    let mut p = placement_props();
    p.insert("png_base64".into(), json!({"type": "string", "description": "The image as a base64 PNG."}));
    p.insert("path".into(), json!({"type": "string", "description": "A PNG file on the engine's machine, instead of png_base64."}));
    p.insert("name".into(), json!({"type": "string", "maxLength": 120, "description": "A name to list it by; Image N when omitted."}));
    json!({"type": "object", "properties": p, "required": [], "additionalProperties": false})
}

/// Loads an image onto the Spotlight and makes it the active one; centred at 60 percent of the view's height unless placed.
pub(crate) fn spotlight_add(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let bytes = match (args.get("png_base64").and_then(Value::as_str), args.get("path").and_then(Value::as_str)) {
        (Some(b), None) => base64::engine::general_purpose::STANDARD.decode(b.trim()).map_err(|e| bad(format!("png_base64 is not base64: {e}")))?,
        (None, Some(p)) => std::fs::read(p).map_err(|e| CommandError::new("read_failed", format!("could not read {p}: {e}")))?,
        _ => return Err(bad("give exactly one of png_base64 or path")),
    };
    let (width, height, pixels) = fsc_paint::png_rgba_decode(&bytes).map_err(bad)?;
    if width == 0 || height == 0 {
        return Err(bad("the image is empty"));
    }
    if s.spotlight.images.len() >= MAX_IMAGES {
        return Err(CommandError::new("too_many", format!("the Spotlight holds at most {MAX_IMAGES} images; spotlight_remove one first")));
    }
    s.spotlight.next += 1;
    let id = s.spotlight.next;
    let name = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).map(String::from).unwrap_or_else(|| format!("Image {id}"));
    let mut img = SpotImage { id, name, width, height, pixels: Arc::new(pixels), center: [0.5, 0.5], size: 0.6, angle: 0.0, opacity: 1.0, tile: false, flip: false };
    set_placement(&mut img, args)?;
    let info = img.info(true);
    s.spotlight.images.push(img);
    s.spotlight.active = Some(id);
    s.spotlight.revision += 1;
    Ok(json!({"image": info, "revision": s.spotlight.revision}))
}

pub(crate) fn set_schema() -> Value {
    let mut p = placement_props();
    p.insert("image".into(), json!({"type": "integer", "minimum": 1, "description": "Spotlight image id; the active image when omitted."}));
    p.insert("activate".into(), json!({"type": "boolean", "description": "Make this image the active one (the one spotlight_apply uses by default)."}));
    json!({"type": "object", "properties": p, "required": [], "additionalProperties": false})
}

/// Turns the dial: moves, sizes, turns, fades, tiles or flips one image, or makes it the active one.
pub(crate) fn spotlight_set(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = image_id(s, args)?;
    let activate = flag(args, "activate")?.unwrap_or(false);
    let img = s.spotlight.images.iter_mut().find(|i| i.id == id).expect("checked");
    let mut next = img.clone();
    set_placement(&mut next, args)?;
    *img = next;
    if activate {
        s.spotlight.active = Some(id);
    }
    s.spotlight.revision += 1;
    let img = s.spotlight.images.iter().find(|i| i.id == id).expect("checked");
    Ok(json!({"image": img.info(s.spotlight.active == Some(id)), "revision": s.spotlight.revision}))
}

pub(crate) fn spotlight_list(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    Ok(listing(s))
}

pub(crate) fn image_schema() -> Value {
    json!({"type": "object", "properties": {
        "image": {"type": "integer", "minimum": 1, "description": "Spotlight image id; the active image when omitted."}
    }, "required": [], "additionalProperties": false})
}

/// The image's own pixels as a PNG, so a window can draw an image an agent loaded.
pub(crate) fn spotlight_image(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = image_id(s, args)?;
    let img = s.spotlight.images.iter().find(|i| i.id == id).expect("checked");
    let png = fsc_paint::png_rgba_rect(img.width, img.height, &img.pixels).map_err(|e| CommandError::new("encode_failed", e))?;
    Ok(json!({"image": img.info(s.spotlight.active == Some(id)), "mime": "image/png", "png_base64": base64::engine::general_purpose::STANDARD.encode(&png)}))
}

pub(crate) fn remove_schema() -> Value {
    json!({"type": "object", "properties": {
        "image": {"type": "integer", "minimum": 1, "description": "Spotlight image id; the active image when omitted."},
        "all": {"type": "boolean", "default": false, "description": "Clear every image from the Spotlight."}
    }, "required": [], "additionalProperties": false})
}

/// Clears one image, or all of them, from the Spotlight; the next image becomes active.
pub(crate) fn spotlight_remove(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let removed = if flag(args, "all")?.unwrap_or(false) {
        let n = s.spotlight.images.len();
        s.spotlight.images.clear();
        s.spotlight.active = None;
        n
    } else {
        let id = image_id(s, args)?;
        s.spotlight.images.retain(|i| i.id != id);
        if s.spotlight.active == Some(id) {
            s.spotlight.active = s.spotlight.images.last().map(|i| i.id);
        }
        1
    };
    s.spotlight.revision += 1;
    let mut out = listing(s);
    out["removed"] = json!(removed);
    Ok(out)
}

pub(crate) fn apply_schema() -> Value {
    json!({"type": "object", "properties": {
        "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
        "image": {"type": "integer", "minimum": 1, "description": "Spotlight image id; the active image when omitted."},
        "view_proj": {"type": "array", "items": {"type": "number"}, "minItems": 16, "maxItems": 16, "description": "The camera the image is seen through, as a column-major view-projection matrix (the window sends its own)."},
        "width": {"type": "number", "minimum": 1, "maximum": 16384, "description": "The view's width in pixels."},
        "height": {"type": "number", "minimum": 1, "maximum": 16384, "description": "The view's height in pixels."},
        "mode": {"type": "string", "enum": ["paint", "sculpt"], "default": "paint", "description": "paint: the image's colour onto the surface; sculpt: the surface raised along its normals by the image's brightness."},
        "target": {"type": "string", "enum": ["vertex", "texture"], "default": "vertex", "description": "paint: texture paints the object's texture per texel (texture_create first) instead of vertex paint."},
        "strength": {"type": "number", "minimum": 0, "maximum": 1, "default": 1, "description": "Multiplies the image's opacity and alpha."},
        "depth": {"type": "number", "minimum": -1, "maximum": 1, "default": 0.02, "description": "sculpt: the raise at full white as a fraction of the object's size; negative carves."},
        "mid": {"type": "number", "minimum": 0, "maximum": 1, "default": 0, "description": "sculpt: the brightness that stays put; darker carves and brighter raises (0: black stays, everything else raises)."},
        "brush": {"type": "object", "properties": {
            "center": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "[x, y] in view pixels, y down."},
            "radius": {"type": "number", "exclusiveMinimum": 0, "maximum": 16384, "description": "In view pixels; the weight falls smoothly to 0 at the rim."}
        }, "required": ["center", "radius"], "additionalProperties": false, "description": "Only under this circle of the view, as a brush dab through the image; the whole image when omitted."}
    }, "required": ["view_proj", "width", "height"], "additionalProperties": false})
}

/// Projection painting and sculpting through a Spotlight image from the given camera: front-facing, unoccluded surfaces
/// only, fading at grazing angles, masked and hidden areas protected; one undo step.
pub(crate) fn spotlight_apply(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let image = image_id(s, args)?;
    let img = s.spotlight.images.iter().find(|i| i.id == image).expect("checked").clone();
    let m = match args.get("view_proj") {
        Some(Value::Array(a)) if a.len() == 16 => {
            let v: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
            if v.iter().any(|x| !x.is_finite()) {
                return Err(bad("view_proj values must be numbers"));
            }
            Mat4::from_cols_slice(&v)
        }
        _ => return Err(bad("view_proj is 16 numbers")),
    };
    let w = num(args, "width", 1.0, 16384.0)?.ok_or_else(|| bad("width is required"))?;
    let h = num(args, "height", 1.0, 16384.0)?.ok_or_else(|| bad("height is required"))?;
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("paint");
    if !["paint", "sculpt"].contains(&mode) {
        return Err(bad("mode is paint or sculpt"));
    }
    let texture = target_texture(args)?;
    if texture && mode == "sculpt" {
        return Err(bad("target texture is for mode paint"));
    }
    let strength = num(args, "strength", 0.0, 1.0)?.unwrap_or(1.0) * img.opacity;
    let depth = num(args, "depth", -1.0, 1.0)?.unwrap_or(0.02);
    let mid = num(args, "mid", 0.0, 1.0)?.unwrap_or(0.0);
    let brush = match args.get("brush") {
        None | Some(Value::Null) => None,
        Some(b) => {
            let c = pair(b, "center")?.ok_or_else(|| bad("brush.center is [x, y]"))?;
            let r = num(b, "radius", 1e-6, 16384.0)?.ok_or_else(|| bad("brush.radius is required"))?;
            Some((c, r))
        }
    };
    let object = s.scene.get(id).expect("checked");
    if texture && object.live_texture().is_none() {
        return Err(no_texture());
    }
    let mesh = &object.mesh;
    let vc = mesh.vertex_count();
    let inv = m.inverse();
    let screen = |p: Vec3| -> Option<[f32; 2]> {
        let c = m * p.extend(1.0);
        (c.w > 1e-9).then(|| [((c.x / c.w) * 0.5 + 0.5) * w, (0.5 - (c.y / c.w) * 0.5) * h])
    };
    let dab = |[x, y]: [f32; 2]| -> f32 {
        match brush {
            None => 1.0,
            Some((c, r)) => {
                let t = (1.0 - ((x - c[0]).powi(2) + (y - c[1]).powi(2)).sqrt() / r).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            }
        }
    };
    let size = mesh.bounds().extent().length().max(1e-6);
    let hid = mesh.has_hidden();
    // Per vertex: how much the camera sees it (facing, unoccluded, unmasked, shown), 0 to 1.
    let mut seen = vec![0.0f32; vc];
    for (v, out) in seen.iter_mut().enumerate() {
        let free = 1.0 - object.mask[v];
        if free <= 0.0 || (hid && object.topology.faces_of(v as u32).iter().all(|&t| mesh.hidden[t as usize])) {
            continue;
        }
        let p = Vec3::from(mesh.positions[v]);
        let Some(sp) = screen(p) else { continue };
        if !texture && (img.local(sp[0], sp[1], w, h).is_none() || dab(sp) <= 0.0) {
            continue;
        }
        let c = m * p.extend(1.0);
        let near = inv * Vec4::new(c.x / c.w, c.y / c.w, -1.0, 1.0);
        let to_cam = (near.truncate() / near.w - p).normalize_or_zero();
        let n = Vec3::from(mesh.normals[v]);
        let facing = n.dot(to_cam);
        if facing <= 0.05 || object.bvh.raycast(mesh, p + n * size * 1e-4, to_cam).is_some() {
            continue;
        }
        *out = ((facing - 0.05) / 0.25).clamp(0.0, 1.0) * free;
    }
    let srgb = |c: f32| c.powf(2.2);
    if texture {
        let object = s.scene.get_mut(id).expect("checked");
        let mut tex = object.texture.take().expect("checked");
        let mesh = &object.mesh;
        let mut done = vec![false; tex.size * tex.size];
        let mut edit = fsc_core::TextureEdit::default();
        let mut hits = Vec::new();
        for t in 0..mesh.triangle_count() {
            let ids = mesh.triangle(t);
            if mesh.face_hidden(t) || ids.iter().all(|&k| seen[k as usize] <= 0.0) {
                continue;
            }
            hits.clear();
            rasterize(mesh, tex.size, t, PAD, None, &mut hits);
            for hit in &hits {
                let at = hit.at as usize;
                if done[at] || (!hit.inside && tex.interior[at]) {
                    continue;
                }
                let p: Vec3 = (0..3).map(|k| Vec3::from(mesh.positions[ids[k] as usize]) * hit.bary[k]).sum();
                let vis: f32 = (0..3).map(|k| seen[ids[k] as usize] * hit.bary[k]).sum();
                let Some(sp) = screen(p) else { continue };
                let Some(uv) = img.local(sp[0], sp[1], w, h) else { continue };
                let c = img.sample(uv);
                let wgt = (c[3] * strength * vis * dab(sp)).min(1.0);
                if wgt <= 1e-4 {
                    continue;
                }
                done[at] = true;
                let before = tex.texels[at];
                let mut now = before;
                for k in 0..3 {
                    now[k] += (srgb(c[k]) - now[k]) * wgt;
                }
                if now != before {
                    edit.texels.push(at as u32);
                    edit.before.push(before);
                    edit.after.push(now);
                    tex.texels[at] = now;
                }
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
            s.history.push(Edit::Texels { label: "Spotlight Paint".into(), object: id, edit });
        }
        return Ok(json!({"object": after, "image": image, "mode": mode, "target": "texture", "texels_painted": painted, "vertices_changed": 0}));
    }
    // Per vertex: the image's colour and alpha where the vertex lands.
    let mut hits: Vec<(u32, [f32; 4], f32)> = Vec::new();
    for (v, &vis) in seen.iter().enumerate() {
        if vis <= 0.0 {
            continue;
        }
        let sp = screen(Vec3::from(mesh.positions[v])).expect("seen vertices project");
        let Some(uv) = img.local(sp[0], sp[1], w, h) else { continue };
        let c = img.sample(uv);
        let wgt = c[3] * strength * vis * dab(sp);
        if wgt > 1e-4 {
            hits.push((v as u32, c, wgt));
        }
    }
    let object = s.scene.get_mut(id).expect("checked");
    let mut edit = VertexEdit::default();
    if mode == "sculpt" {
        for &(v, c, wgt) in &hits {
            let lum = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let rise = (lum - mid) * depth * size * wgt;
            if rise.abs() < 1e-9 {
                continue;
            }
            let p = object.mesh.positions[v as usize];
            let n = Vec3::from(object.mesh.normals[v as usize]);
            edit.indices.push(v);
            edit.before.push(p);
            edit.after.push((Vec3::from(p) + n * rise).to_array());
            edit.mask_before.push(object.mask[v as usize]);
            edit.mask_after.push(object.mask[v as usize]);
        }
        let n = edit.indices.len();
        if n > 0 {
            edit.apply(object, true);
        }
        let after = summary(object);
        if n > 0 {
            push_vertices(s, id, "Spotlight Sculpt".into(), edit);
        }
        return Ok(json!({"object": after, "image": image, "mode": mode, "target": "vertex", "vertices_changed": n, "texels_painted": 0}));
    }
    let had = object.mesh.has_paint();
    object.mesh.ensure_paint();
    let mut layer = object.layered().then(|| (object.paint_layers.active, Vec::new(), Vec::new()));
    for &(v, c, wgt) in &hits {
        let before = object.mesh.paint[v as usize];
        let was = object.paint_target(v);
        object.paint_vertex(v, fsc_core::paint_from5(&[srgb(c[0]), srgb(c[1]), srgb(c[2]), 0.5, 0.0]), [true, false, false, false, false], wgt.min(1.0), false);
        object.composite_paint(Some(&[v]));
        let after = object.mesh.paint[v as usize];
        let now = object.paint_target(v);
        if after == before && was == now {
            continue;
        }
        if let (Some((_, b, a)), Some(was), Some(now)) = (layer.as_mut(), was, now) {
            b.push(was);
            a.push(now);
        }
        let p = object.mesh.positions[v as usize];
        edit.indices.push(v);
        edit.before.push(p);
        edit.after.push(p);
        edit.mask_before.push(object.mask[v as usize]);
        edit.mask_after.push(object.mask[v as usize]);
        edit.paint_before.push(if had { before } else { fsc_core::DEFAULT_PAINT });
        edit.paint_after.push(after);
    }
    if layer.as_ref().is_some_and(|(_, b, _)| b.len() == edit.indices.len()) {
        edit.layer = layer;
    }
    let n = edit.indices.len();
    if n > 0 {
        object.revision += 1;
    }
    let after = summary(object);
    if n > 0 {
        s.history.push(Edit::Vertices { label: "Spotlight Paint".into(), object: id, edit });
    }
    Ok(json!({"object": after, "image": image, "mode": mode, "target": "vertex", "vertices_changed": n, "texels_painted": 0}))
}
