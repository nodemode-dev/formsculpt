//! Hard-surface cuts and the texturing round trip: clipping by a plane (keep one side or slice in two), beveling a
//! chosen edge run with an exact cutter, dragging single UV points, and reading Substance Painter's exported textures
//! back onto the vertices.

use std::collections::HashMap;

use fsc_core::time::Instant;
use fsc_core::Mesh;
use fsc_topo::BooleanOp;
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::multires_cmds::guard;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::topo_cmds::apply;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn vec3(args: &Value, key: &str) -> Result<Option<Vec3>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() == 3 => {
            let c = |i: usize| a[i].as_f64().map(|x| x as f32).ok_or_else(|| bad(format!("{key} needs three numbers")));
            Ok(Some(Vec3::new(c(0)?, c(1)?, c(2)?)))
        }
        _ => Err(bad(format!("{key} must be [x, y, z]"))),
    }
}

fn uv(args: &Value, key: &str) -> Result<[f32; 2], CommandError> {
    match args.get(key).and_then(Value::as_array) {
        Some(a) if a.len() == 2 => {
            let f = |v: &Value| v.as_f64().map(|x| x as f32).ok_or_else(|| bad(format!("{key} is [u, v]")));
            Ok([f(&a[0])?, f(&a[1])?])
        }
        _ => Err(bad(format!("{key} is required as [u, v]"))),
    }
}

/// A closed prism: a convex polygon (in order around it) swept from a to b, wound outward.
fn prism(section: &[Vec3], along: Vec3) -> Mesh {
    let n = section.len();
    let mut positions: Vec<[f32; 3]> = section.iter().map(|p| p.to_array()).collect();
    positions.extend(section.iter().map(|p| (*p + along).to_array()));
    let mut indices = Vec::new();
    for i in 1..n as u32 - 1 {
        indices.extend([0, i + 1, i]);
        indices.extend([n as u32, n as u32 + i, n as u32 + i + 1]);
    }
    for i in 0..n as u32 {
        let j = (i + 1) % n as u32;
        indices.extend([i, j, n as u32 + j, i, n as u32 + j, n as u32 + i]);
    }
    let mut mesh = Mesh::new(positions, indices);
    // Signed volume decides the winding.
    let vol: f32 = mesh.indices.chunks(3).map(|t| Vec3::from(mesh.positions[t[0] as usize]).dot(Vec3::from(mesh.positions[t[1] as usize]).cross(Vec3::from(mesh.positions[t[2] as usize])))).sum();
    if vol < 0.0 {
        for t in mesh.indices.chunks_mut(3) {
            t.swap(1, 2);
        }
        mesh.recompute_normals();
    }
    mesh
}

/// A box on the positive side of a plane, large enough to swallow everything within `size` of the point.
fn half_space(point: Vec3, normal: Vec3, size: f32) -> Mesh {
    let u = normal.any_orthonormal_vector();
    let v = normal.cross(u);
    let section = [point + (u + v) * size, point + (-u + v) * size, point + (-u - v) * size, point + (u - v) * size];
    prism(&section, normal * size * 2.0)
}

pub(crate) fn clip_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A point on the cutting plane; the object's centre when omitted."},
            "normal": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The plane's normal: the side it points to is cut away."},
            "mode": {"type": "string", "enum": ["keep", "slice"], "default": "keep", "description": "keep removes the far side (a clip cut); slice keeps both halves as two capped objects."}
        },
        "required": ["normal"],
        "additionalProperties": false
    })
}

/// Clip by a plane with an exact boolean, so the cut face is flat and watertight.
pub(crate) fn clip_plane(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Clip")?;
    let started = Instant::now();
    let object = s.scene.get(id).expect("checked");
    let bounds = object.mesh.bounds();
    let normal = vec3(args, "normal")?.and_then(Vec3::try_normalize).ok_or_else(|| bad("normal is required and must have length"))?;
    let point = vec3(args, "point")?.unwrap_or_else(|| bounds.center());
    let slice = match args.get("mode").and_then(Value::as_str).unwrap_or("keep") {
        "keep" => false,
        "slice" => true,
        other => return Err(bad(format!("mode must be keep or slice, not {other}"))),
    };
    let size = bounds.extent().length() * 2.0 + (point - bounds.center()).length() * 2.0 + 1e-3;
    let cutter = half_space(point, normal, size);
    let kept = fsc_topo::exact_boolean(&object.mesh, &cutter, BooleanOp::Difference).map_err(|e| CommandError::new("boolean_failed", e))?;
    let other = if slice { Some(fsc_topo::exact_boolean(&object.mesh, &cutter, BooleanOp::Intersection).map_err(|e| CommandError::new("boolean_failed", e))?) } else { None };
    let name = object.name.clone();
    let mask = vec![0.0; kept.vertex_count()];
    let mut reply = apply(s, id, "Clip", kept, mask)?;
    if let Some(piece) = other.filter(|m| m.triangle_count() > 0) {
        let new_name = s.scene.unique_name(&format!("{name} slice"));
        let new_id = s.scene.add(new_name, piece);
        let index = s.scene.index_of(new_id).expect("added");
        s.history.push(Edit::Add { id: new_id, index, parked: None });
        reply["slice"] = summary(s.scene.get(new_id).expect("added"));
    }
    reply["ms"] = json!(started.elapsed().as_millis());
    Ok(reply)
}

/// Sharp edges keyed by welded positions, so meshes with split vertices along creases still connect.
struct Edges {
    /// Each edge: its two welded end points and the normals of the two faces that meet there.
    list: Vec<(Vec3, Vec3, Vec3, Vec3, Vec3, Vec3)>,
}

fn sharp_edges(mesh: &Mesh, min_angle: f32) -> Edges {
    let size = mesh.bounds().extent().length().max(1e-6);
    let q = |p: Vec3| ((p / size * 1e5).round()).as_ivec3();
    let normals = mesh.face_normals();
    let mut by_edge: HashMap<(glam::IVec3, glam::IVec3), Vec<usize>> = HashMap::new();
    for (f, t) in mesh.indices.chunks(3).enumerate() {
        for k in 0..3 {
            let (a, b) = (q(mesh.position(t[k])), q(mesh.position(t[(k + 1) % 3])));
            let key = if (a.x, a.y, a.z) < (b.x, b.y, b.z) { (a, b) } else { (b, a) };
            by_edge.entry(key).or_default().push(f);
        }
    }
    let cos_limit = min_angle.to_radians().cos();
    let mut list = Vec::new();
    for faces in by_edge.values() {
        if faces.len() != 2 {
            continue;
        }
        let (f1, f2) = (faces[0], faces[1]);
        let (n1, n2) = (normals[f1].normalize_or_zero(), normals[f2].normalize_or_zero());
        if n1.dot(n2) > cos_limit {
            continue;
        }
        // The shared corners and each face's third corner.
        let t1: Vec<Vec3> = mesh.indices[f1 * 3..f1 * 3 + 3].iter().map(|&v| mesh.position(v)).collect();
        let t2: Vec<Vec3> = mesh.indices[f2 * 3..f2 * 3 + 3].iter().map(|&v| mesh.position(v)).collect();
        let shared: Vec<Vec3> = t1.iter().copied().filter(|a| t2.iter().any(|b| q(*a) == q(*b))).collect();
        if shared.len() != 2 {
            continue;
        }
        let o1 = t1.iter().copied().find(|a| q(*a) != q(shared[0]) && q(*a) != q(shared[1])).unwrap_or(t1[0]);
        let o2 = t2.iter().copied().find(|a| q(*a) != q(shared[0]) && q(*a) != q(shared[1])).unwrap_or(t2[0]);
        list.push((shared[0], shared[1], n1, n2, o1, o2));
    }
    Edges { list }
}

fn point_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-20)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

pub(crate) fn bevel_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A point on or near the edge to bevel (a click on the model); the straight run of sharp edges through it is beveled."},
            "width": {"type": "number", "exclusiveMinimum": 0, "description": "How far the bevel reaches into each face, in world units; 4 percent of the object's size when omitted."},
            "mode": {"type": "string", "enum": ["chamfer", "round"], "default": "chamfer", "description": "chamfer cuts one flat face; round cuts a fillet."},
            "segments": {"type": "integer", "minimum": 2, "maximum": 32, "default": 6, "description": "Round: steps across the fillet."},
            "angle": {"type": "number", "minimum": 5, "maximum": 170, "default": 30, "description": "Faces meeting at more than this many degrees count as a sharp edge."}
        },
        "required": ["point"],
        "additionalProperties": false
    })
}

/// Bevel one straight run of sharp edges (the one nearest point) by subtracting an exact chamfer or fillet prism.
pub(crate) fn bevel_edge(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Bevel edge")?;
    let started = Instant::now();
    let object = s.scene.get(id).expect("checked");
    let point = vec3(args, "point")?.ok_or_else(|| bad("point is required"))?;
    let size = object.mesh.bounds().extent().length().max(1e-6);
    let width = args.get("width").and_then(Value::as_f64).map(|w| w as f32).unwrap_or(size * 0.04);
    if !(width > 0.0 && width < size) {
        return Err(bad("width must be above 0 and below the object's size"));
    }
    let round = match args.get("mode").and_then(Value::as_str).unwrap_or("chamfer") {
        "chamfer" => false,
        "round" => true,
        other => return Err(bad(format!("mode must be chamfer or round, not {other}"))),
    };
    let segments = args.get("segments").and_then(Value::as_u64).unwrap_or(6).clamp(2, 32) as usize;
    let angle = args.get("angle").and_then(Value::as_f64).unwrap_or(30.0).clamp(5.0, 170.0) as f32;
    let edges = sharp_edges(&object.mesh, angle);
    let seed = edges
        .list
        .iter()
        .min_by(|a, b| point_segment(point, a.0, a.1).total_cmp(&point_segment(point, b.0, b.1)))
        .ok_or_else(|| CommandError::new("no_edge", format!("the object has no edges sharper than {angle} degrees")))?;
    if point_segment(point, seed.0, seed.1) > size * 0.1 {
        return Err(CommandError::new("no_edge", "no sharp edge near that point"));
    }
    let dir = (seed.1 - seed.0).normalize();
    let (n1, n2) = (seed.2, seed.3);
    // The run: every sharp edge on the same line with the same pair of face normals.
    let tol = size * 1e-4;
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for e in &edges.list {
        let on_line = |p: Vec3| (p - seed.0 - dir * (p - seed.0).dot(dir)).length() < tol;
        let same = (e.2.dot(n1) > 0.999 && e.3.dot(n2) > 0.999) || (e.2.dot(n2) > 0.999 && e.3.dot(n1) > 0.999);
        if same && on_line(e.0) && on_line(e.1) {
            for p in [e.0, e.1] {
                let t = (p - seed.0).dot(dir);
                lo = lo.min(t);
                hi = hi.max(t);
            }
        }
    }
    // In-face directions across the edge, pointing into each face.
    let mut t1 = n1.cross(dir).normalize();
    if t1.dot(seed.4 - seed.0) < 0.0 {
        t1 = -t1;
    }
    let mut t2 = n2.cross(dir).normalize();
    if t2.dot(seed.5 - seed.0) < 0.0 {
        t2 = -t2;
    }
    // Only outward (convex) edges: the other face lies behind each face's plane.
    if t2.dot(n1) > 1e-4 || t1.dot(n2) > 1e-4 {
        return Err(CommandError::new("concave_edge", "that edge folds inward; bevel works on outward edges"));
    }
    let reach = width * 3.0;
    let e = seed.0 + dir * (lo - reach);
    let (a, b) = (e + t1 * width, e + t2 * width);
    let mut section = vec![a + n1 * reach, a];
    if round {
        let alpha = t1.dot(t2).clamp(-1.0, 1.0).acos();
        let r = width * (alpha * 0.5).tan();
        let centre = a - n1 * r;
        for k in 1..segments {
            let f = k as f32 / segments as f32;
            let d = n1.lerp(n2, f).normalize();
            // Slerp-like spacing on the arc by angle.
            let full = n1.dot(n2).clamp(-1.0, 1.0).acos();
            let d = if full > 1e-4 { (n1 * ((1.0 - f) * full).sin() + n2 * (f * full).sin()) / full.sin() } else { d };
            section.push(centre + d * r);
        }
    }
    section.push(b);
    section.push(b + n2 * reach);
    section.push(e + (n1 + n2).normalize() * reach * 2.0);
    let length = hi - lo + reach * 2.0;
    let cutter = prism(&section, dir * length);
    let mesh = fsc_topo::exact_boolean(&object.mesh, &cutter, BooleanOp::Difference).map_err(|e| CommandError::new("boolean_failed", e))?;
    let mask = vec![0.0; mesh.vertex_count()];
    let mut reply = apply(s, id, if round { "Round Edge" } else { "Chamfer Edge" }, mesh, mask)?;
    reply["edge"] = json!({"from": (seed.0 + dir * lo).to_array(), "length": hi - lo, "direction": dir.to_array()});
    reply["width"] = json!(width);
    reply["ms"] = json!(started.elapsed().as_millis());
    Ok(reply)
}

pub(crate) fn uv_point_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "at": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "The UV point [u, v] to grab."},
            "to": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "Where it goes."},
            "radius": {"type": "number", "minimum": 0, "maximum": 1, "default": 0, "description": "UV corners within this distance follow with a smooth falloff; 0 moves only the corners at the nearest point (one UV vertex)."},
            "island": {"type": "boolean", "default": true, "description": "Keep the grab to the island under at."}
        },
        "required": ["at", "to"],
        "additionalProperties": false
    })
}

/// Drag UV vertices: the nearest UV point (or every corner within radius, with falloff). Clears baked maps.
pub(crate) fn uv_move_point(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let at = uv(args, "at")?;
    let to = uv(args, "to")?;
    let radius = args.get("radius").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0) as f32;
    let one_island = args.get("island").and_then(Value::as_bool).unwrap_or(true);
    let object = s.scene.get_mut(id).expect("checked");
    if object.mesh.uvs.len() != object.mesh.indices.len() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first"));
    }
    let d2 = |p: [f32; 2]| (p[0] - at[0]).powi(2) + (p[1] - at[1]).powi(2);
    let (nearest, best) = object.mesh.uvs.iter().enumerate().map(|(c, p)| (c, d2(*p))).min_by(|a, b| a.1.total_cmp(&b.1)).ok_or_else(|| bad("no UVs"))?;
    let islands = if one_island { Some(fsc_paint::islands(&object.mesh)) } else { None };
    let home = islands.as_ref().map(|i| i[nearest / 3]);
    let anchor = object.mesh.uvs[nearest];
    let delta = [to[0] - at[0], to[1] - at[1]];
    let mut mesh = object.mesh.clone();
    let mut moved = 0;
    for (c, p) in object.mesh.uvs.iter().enumerate() {
        if let (Some(i), Some(h)) = (&islands, home) {
            if i[c / 3] != h {
                continue;
            }
        }
        let w = if radius > 0.0 {
            let d = d2(*p).sqrt();
            if d >= radius {
                continue;
            }
            let x = 1.0 - d / radius;
            x * x * (3.0 - 2.0 * x)
        } else {
            let same = (p[0] - anchor[0]).abs() < 1e-6 && (p[1] - anchor[1]).abs() < 1e-6;
            if !same {
                continue;
            }
            1.0
        };
        mesh.uvs[c] = [p[0] + delta[0] * w, p[1] + delta[1] * w];
        moved += 1;
    }
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    object.clear_maps();
    let after = summary(object);
    s.history.push(Edit::Topology { label: "Move UV point".into(), object: id, mesh: old_mesh, mask: old_mask });
    Ok(json!({"object": after, "corners_moved": moved, "grabbed": anchor, "distance": best.sqrt()}))
}

pub(crate) fn substance_sync_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "folder": {"type": "string", "description": "The folder send_to_substance wrote and Painter exported back into."},
            "color": {"type": "string", "description": "The base colour PNG; [name]_BaseColor.png in folder when omitted."}
        },
        "required": ["folder"],
        "additionalProperties": false
    })
}

/// Bring Substance Painter's exported textures back: BaseColor (and Roughness and Metallic when beside it) sampled at
/// each vertex's UV into vertex paint, and the colour map kept as the object's baked colour map for GLB export.
pub(crate) fn substance_sync(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let folder = args.get("folder").and_then(Value::as_str).filter(|f| !f.trim().is_empty()).ok_or_else(|| bad("folder is required"))?;
    let object = s.scene.get(id).expect("checked");
    if object.mesh.uvs.len() != object.mesh.indices.len() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; the package was sent with UVs, so check it is the same object at level 0"));
    }
    let stem: String = object.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let dir = std::path::Path::new(folder);
    let color_path = args.get("color").and_then(Value::as_str).map(std::path::PathBuf::from).unwrap_or_else(|| dir.join(format!("{stem}_BaseColor.png")));
    let color_bytes = std::fs::read(&color_path).map_err(|e| CommandError::new("io_failed", format!("could not read {}: {e}", color_path.display())))?;
    let (w, h, pixels) = fsc_paint::png_rgba_decode(&color_bytes).map_err(|e| CommandError::new("io_failed", e))?;
    let gray = |name: &str| -> Option<(usize, usize, Vec<u8>)> { std::fs::read(dir.join(format!("{stem}_{name}.png"))).ok().and_then(|b| fsc_paint::png_gray(&b).ok()) };
    let (rough, metal) = (gray("Roughness"), gray("Metallic"));
    let texel = |u: f32, v: f32, w: usize, h: usize| -> usize {
        let x = ((u.rem_euclid(1.0)) * w as f32).floor().clamp(0.0, w as f32 - 1.0) as usize;
        let y = ((1.0 - v.rem_euclid(1.0)) * h as f32).floor().clamp(0.0, h as f32 - 1.0) as usize;
        y * w + x
    };
    let mut mesh = object.mesh.clone();
    mesh.ensure_paint();
    let mut seen = vec![false; mesh.vertex_count()];
    let srgb = |c: u8| (c as f32 / 255.0).powf(2.2);
    for (c, &v) in object.mesh.indices.iter().enumerate() {
        let v = v as usize;
        if seen[v] {
            continue;
        }
        seen[v] = true;
        let [u, vv] = object.mesh.uvs[c];
        let p = pixels[texel(u, vv, w, h)];
        let paint = &mut mesh.paint[v];
        paint[0] = srgb(p[0]);
        paint[1] = srgb(p[1]);
        paint[2] = srgb(p[2]);
        if let Some((rw, rh, r)) = &rough {
            paint[3] = r[texel(u, vv, *rw, *rh)] as f32 / 255.0;
        }
        if let Some((mw, mh, m)) = &metal {
            paint[4] = m[texel(u, vv, *mw, *mh)] as f32 / 255.0;
        }
    }
    let painted = seen.iter().filter(|s| **s).count();
    let object = s.scene.get_mut(id).expect("checked");
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    if let Some(maps) = object.maps.as_mut() {
        maps.color_png = color_bytes;
        maps.source = format!("Substance Painter ({})", color_path.display());
    }
    let after = summary(object);
    s.history.push(Edit::Topology { label: "Substance sync".into(), object: id, mesh: old_mesh, mask: old_mask });
    Ok(json!({"object": after, "vertices_painted": painted, "texture": [w, h], "roughness": rough.is_some(), "metallic": metal.is_some()}))
}
