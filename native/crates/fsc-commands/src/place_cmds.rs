use fsc_core::{Mesh, Primitive};
use glam::{Mat4, Quat, Vec3};
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn vec3(args: &Value, key: &str) -> Result<Option<Vec3>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() == 3 => {
            let v: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
            if v.iter().any(|x| !x.is_finite()) {
                return Err(bad(format!("{key} values must be numbers")));
            }
            Ok(Some(Vec3::new(v[0], v[1], v[2])))
        }
        Some(_) => Err(bad(format!("{key} is [x, y, z]"))),
    }
}

/// A copy of `mesh` under an affine transform; a mirroring transform reverses the triangles so they still face out.
pub(crate) fn transformed(mesh: &Mesh, m: Mat4) -> Mesh {
    let positions = mesh.positions.iter().map(|&p| m.transform_point3(Vec3::from(p)).to_array()).collect();
    let mut indices = mesh.indices.clone();
    if m.determinant() < 0.0 {
        for t in indices.chunks_mut(3) {
            t.swap(1, 2);
        }
    }
    let mut out = Mesh::new(positions, indices);
    out.paint = mesh.paint.clone();
    if mesh.has_groups() {
        out.groups = mesh.groups.clone();
    }
    out
}

/// Linked copies follow their original: after any change to the original they are rebuilt from it under their own
/// transform. A copy changed directly (a stroke on it, a transform) leaves the link, keeping its own shape.
pub fn sync_instances(s: &mut Session) {
    let links: Vec<(u32, fsc_core::Instance)> = s.scene.objects.iter().filter_map(|o| o.instance.clone().map(|i| (o.id, i))).collect();
    for (id, link) in links {
        let source = s.scene.get(link.source).map(|o| (o.revision, o.mesh.clone()));
        let copy = s.scene.get_mut(id).expect("listed");
        let Some((source_revision, mesh)) = source else {
            copy.instance = None;
            continue;
        };
        if copy.revision != link.own_revision {
            copy.instance = None;
            continue;
        }
        if source_revision == link.source_revision {
            continue;
        }
        let mesh = transformed(&mesh, Mat4::from_cols_array(&link.matrix));
        let mask = vec![0.0; mesh.vertex_count()];
        copy.replace_mesh(mesh, mask);
        copy.instance = Some(fsc_core::Instance { source_revision, own_revision: copy.revision, ..link });
    }
}

/// Adds an object; with `record` it is its own undo step (an array records one scene step for all its copies).
pub(crate) fn add_object(s: &mut Session, name: &str, mesh: Mesh, record: bool) -> u32 {
    let name = s.scene.unique_name(name);
    let id = s.scene.add(name, mesh);
    if record {
        let index = s.scene.index_of(id).expect("just added");
        s.history.push(Edit::Add { id, index, parked: None });
    }
    id
}

pub(crate) struct SplitMix(pub(crate) u64);

impl SplitMix {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [-1, 1).
    pub(crate) fn signed(&mut self) -> f32 {
        ((self.next() >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
}

struct Jitter {
    position: f32,
    rotation: f32,
    scale: f32,
    seed: u64,
}

pub(crate) fn number_in(obj: &Value, key: &str, lo: f64, hi: f64, default: f64) -> Result<f32, CommandError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(default as f32),
        Some(v) => match v.as_f64() {
            Some(x) if x.is_finite() && (lo..=hi).contains(&x) => Ok(x as f32),
            _ => Err(bad(format!("{key} must be a number from {lo} to {hi}"))),
        },
    }
}

fn parse_jitter(args: &Value) -> Result<Option<Jitter>, CommandError> {
    let j = match args.get("jitter") {
        None | Some(Value::Null) => return Ok(None),
        Some(j @ Value::Object(m)) => {
            if let Some(k) = m.keys().find(|k| !["position", "rotation", "scale", "seed"].contains(&k.as_str())) {
                return Err(bad(format!("jitter has no field {k}; use position, rotation, scale and seed")));
            }
            j
        }
        Some(_) => return Err(bad("jitter is an object {position, rotation, scale, seed}")),
    };
    let seed = match j.get("seed") {
        None | Some(Value::Null) => 1,
        Some(v) => v.as_i64().map(|x| x as u64).or_else(|| v.as_u64()).ok_or_else(|| bad("jitter seed must be an integer"))?,
    };
    Ok(Some(Jitter {
        position: number_in(j, "position", 0.0, 1.0e6, 0.0)?,
        rotation: number_in(j, "rotation", 0.0, 180.0, 0.0)?,
        scale: number_in(j, "scale", 0.0, 0.95, 0.0)?,
        seed,
    }))
}

pub(crate) fn array_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object to repeat; the active object when omitted."},
            "count": {"type": "integer", "minimum": 2, "maximum": 200, "default": 4, "description": "Copies in all, the original included."},
            "mode": {"type": "string", "enum": ["linear", "radial", "mirror", "curve"], "default": "linear", "description": "linear and radial repeat count copies; mirror adds one copy reflected across the plane square to axis through center; curve places count copies evenly along points."},
            "points": {"type": "array", "minItems": 2, "maxItems": 4096, "items": crate::curve_cmds::point_schema(3), "description": "Curve: the path, as [x, y, z] points or objects {p, sharp, round} (corners and fillets as in sample_curve); each copy's bounds centre sits on it."},
            "interp": {"type": "string", "enum": ["catmull_rom", "bspline", "linear"], "default": "catmull_rom", "description": "Curve: how the path runs between points."},
            "closed": {"type": "boolean", "default": false, "description": "Curve: loop the path back to its first point."},
            "align": {"type": "boolean", "default": true, "description": "Curve: turn each copy so the original's +x follows the path."},
            "linked": {"type": "boolean", "default": false, "description": "Linked copies (instances) follow the original: sculpt, paint or remesh it and every copy updates after the change. Changing a copy directly unlinks it."},
            "offset": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Linear: the step from one copy to the next, in world units; one bounds width along x when omitted."},
            "axis": {"type": "string", "enum": ["x", "y", "z"], "default": "y", "description": "Radial: the axis the copies turn around (through center)."},
            "center": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Radial: the point the axis passes through; the world origin when omitted."},
            "angle": {"type": "number", "minimum": -360, "maximum": 360, "default": 360, "description": "Radial: the total turn in degrees; 360 spaces the copies evenly around the full circle."},
            "merge": {"type": "boolean", "default": false, "description": "Join the copies with the original into one object instead of adding separate objects."},
            "scale_start": {"type": "number", "minimum": 0.01, "maximum": 100, "default": 1, "description": "Uniform scale of the first copy in the ramp (about the original's bounds centre). Not allowed with mirror."},
            "scale_end": {"type": "number", "minimum": 0.01, "maximum": 100, "default": 1, "description": "Uniform scale of the last copy; copies in between ramp linearly. Not allowed with mirror."},
            "jitter": {
                "type": "object",
                "description": "Deterministic random variation per copy (the original is untouched); the same seed gives the same result. Not allowed with mirror.",
                "properties": {
                    "position": {"type": "number", "minimum": 0, "default": 0, "description": "Each copy moves by up to this many world units either way on each axis."},
                    "rotation": {"type": "number", "minimum": 0, "maximum": 180, "default": 0, "description": "Each copy turns about a random axis by up to this many degrees either way."},
                    "scale": {"type": "number", "minimum": 0, "maximum": 0.95, "default": 0, "description": "Each copy's uniform scale is multiplied by 1 plus or minus up to this fraction."},
                    "seed": {"type": "integer", "default": 1, "description": "The random seed."}
                },
                "additionalProperties": false
            }
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn array_object(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let count = args.get("count").and_then(Value::as_u64).unwrap_or(4);
    if !(2..=200).contains(&count) {
        return Err(bad("count must be 2 to 200"));
    }
    let merge = args.get("merge").and_then(Value::as_bool).unwrap_or(false);
    let o = s.scene.get(id).expect("checked");
    if merge && o.multires.is_some() {
        return Err(CommandError::new("has_levels", "merging copies would change the vertex count under the subdivision levels; delete the levels first or leave merge off"));
    }
    let b = o.mesh.bounds();
    let axis_of = |key: &str, default: &str| -> Result<usize, CommandError> {
        match args.get(key).and_then(Value::as_str).unwrap_or(default) {
            "x" => Ok(0),
            "y" => Ok(1),
            "z" => Ok(2),
            other => Err(bad(format!("{key} must be x, y or z, not {other}"))),
        }
    };
    // Each copy is the original under one transform (kept for linked copies, which follow the original).
    let mut transforms: Vec<Mat4> = Vec::new();
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("linear");
    let scale_start = number_in(args, "scale_start", 0.01, 100.0, 1.0)?;
    let scale_end = number_in(args, "scale_end", 0.01, 100.0, 1.0)?;
    let jitter = parse_jitter(args)?;
    let shaped = ["scale_start", "scale_end", "jitter"].iter().any(|k| !matches!(args.get(*k), None | Some(Value::Null)));
    if mode == "mirror" && shaped {
        return Err(bad("scale_start, scale_end and jitter do not apply to mirror mode"));
    }
    let mut first_k = 1usize;
    match mode {
        "linear" => {
            let step = vec3(args, "offset")?.unwrap_or(Vec3::new(b.extent().x * 1.2, 0.0, 0.0));
            for k in 1..count {
                transforms.push(Mat4::from_translation(step * k as f32));
            }
        }
        "radial" => {
            let axis = [Vec3::X, Vec3::Y, Vec3::Z][axis_of("axis", "y")?];
            let center = vec3(args, "center")?.unwrap_or(Vec3::ZERO);
            let total = args.get("angle").and_then(Value::as_f64).unwrap_or(360.0) as f32;
            let full = (total.abs() - 360.0).abs() < 1e-3;
            let step = total.to_radians() / if full { count as f32 } else { (count - 1) as f32 };
            for k in 1..count {
                transforms.push(Mat4::from_translation(center) * Mat4::from_axis_angle(axis, step * k as f32) * Mat4::from_translation(-center));
            }
        }
        "mirror" => {
            // One mirrored copy across the plane square to `axis` through center (the world origin by default).
            let a = axis_of("axis", "x")?;
            let center = vec3(args, "center")?.unwrap_or(Vec3::ZERO);
            let mut flip = Vec3::ONE;
            flip[a] = -1.0;
            transforms.push(Mat4::from_translation(center) * Mat4::from_scale(flip) * Mat4::from_translation(-center));
        }
        "curve" => {
            // count copies spaced evenly along a polyline, the original's bounds centre on the curve, turned so the
            // original's +x follows the curve's direction when align is on.
            let points: Vec<Vec3> = crate::curve_cmds::path_positions(args, "points", "curve mode needs points: [[x, y, z], ...]")?;
            if points.len() < 2 {
                return Err(bad("a curve needs at least two points"));
            }
            let align = args.get("align").and_then(Value::as_bool).unwrap_or(true);
            let lengths: Vec<f32> = points.windows(2).map(|w| w[0].distance(w[1])).collect();
            let total: f32 = lengths.iter().sum();
            if total <= 1e-9 {
                return Err(bad("the curve has no length"));
            }
            let c = b.center();
            first_k = 0;
            for k in 0..count {
                let want = total * k as f32 / (count - 1) as f32;
                let mut walked = 0.0;
                let mut seg = 0;
                while seg + 1 < lengths.len() && walked + lengths[seg] < want {
                    walked += lengths[seg];
                    seg += 1;
                }
                let t = ((want - walked) / lengths[seg].max(1e-9)).clamp(0.0, 1.0);
                let at = points[seg].lerp(points[seg + 1], t);
                let dir = (points[seg + 1] - points[seg]).normalize_or(Vec3::X);
                let turn = if align { Quat::from_rotation_arc(Vec3::X, dir) } else { Quat::IDENTITY };
                transforms.push(Mat4::from_translation(at) * Mat4::from_quat(turn) * Mat4::from_translation(-c));
            }
        }
        other => return Err(bad(format!("unknown mode {other}; use linear, radial, mirror or curve"))),
    }
    if shaped {
        let c = b.center();
        let mut rng = jitter.as_ref().map(|j| SplitMix(j.seed));
        for (i, m) in transforms.iter_mut().enumerate() {
            let t = (first_k + i) as f32 / (count - 1) as f32;
            let mut scale = scale_start + (scale_end - scale_start) * t;
            let mut turn = Quat::IDENTITY;
            let mut shift = Vec3::ZERO;
            if let (Some(j), Some(r)) = (&jitter, rng.as_mut()) {
                shift = Vec3::new(r.signed(), r.signed(), r.signed()) * j.position;
                let axis = Vec3::new(r.signed(), r.signed(), r.signed()).normalize_or(Vec3::Y);
                turn = Quat::from_axis_angle(axis, (r.signed() * j.rotation).to_radians());
                scale *= 1.0 + r.signed() * j.scale;
            }
            let local = Mat4::from_translation(c) * Mat4::from_quat(turn) * Mat4::from_scale(Vec3::splat(scale)) * Mat4::from_translation(-c);
            *m = Mat4::from_translation(shift) * *m * local;
        }
    }
    let copies: Vec<Mesh> = transforms.iter().map(|m| transformed(&o.mesh, *m)).collect();
    let linked = args.get("linked").and_then(Value::as_bool).unwrap_or(false);
    if linked && merge {
        return Err(bad("linked copies stay separate objects; leave merge off"));
    }
    let source_revision = o.revision;
    let name = o.name.clone();
    if merge {
        let mut parts: Vec<&Mesh> = vec![&o.mesh];
        parts.extend(copies.iter());
        let mut mesh = fsc_topo::merge(&parts);
        if o.mesh.has_paint() {
            mesh.paint = parts.iter().flat_map(|m| if m.has_paint() { m.paint.clone() } else { vec![fsc_core::DEFAULT_PAINT; m.vertex_count()] }).collect();
        }
        let reply = crate::topo_cmds::apply(s, id, "Array", mesh, Vec::new())?;
        return Ok(json!({"object": reply["object"], "copies": count}));
    }
    s.history.push(Edit::Scene { label: "Array".into(), objects: s.scene.objects.clone(), active: s.scene.active });
    let ids: Vec<u32> = copies.into_iter().enumerate().map(|(k, m)| add_object(s, &format!("{name} {}", k + 2), m, false)).collect();
    if linked {
        for (&cid, m) in ids.iter().zip(&transforms) {
            let copy = s.scene.get_mut(cid).expect("just added");
            copy.instance = Some(fsc_core::Instance { source: id, matrix: m.to_cols_array(), source_revision, own_revision: copy.revision });
        }
    }
    s.scene.active = Some(id);
    Ok(json!({"original": id, "added": ids, "copies": ids.len() + 1, "linked": linked}))
}

pub(crate) fn insert_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Where to place it, usually a point on a surface (pick gives one)."},
            "normal": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The surface normal there; the new mesh stands along it. Up when omitted."},
            "kind": {"type": "string", "enum": ["sphere", "cube", "cylinder", "cone", "torus"], "default": "sphere", "description": "A primitive to insert, unless source_id is given."},
            "source_id": {"type": "integer", "minimum": 1, "description": "Insert a copy of this object instead (a horn, a bolt, an eye)."},
            "size": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "default": 0.2, "description": "The inserted mesh's largest dimension, in world units."},
            "sink": {"type": "number", "minimum": -1, "maximum": 1, "default": 0.25, "description": "How far it sinks into the surface, as a fraction of its height (0 sits on top, 0.5 half buried)."}
        },
        "required": ["point"],
        "additionalProperties": false
    })
}

/// The mesh an insert places: a copy of source_id, or a primitive of kind.
fn insert_source(s: &Session, args: &Value) -> Result<(Mesh, String), CommandError> {
    Ok(match args.get("source_id").and_then(Value::as_u64) {
        Some(sid) => {
            let o = s.scene.get(sid as u32).ok_or_else(|| CommandError::new("not_found", format!("no object {sid}")))?;
            (o.mesh.clone(), format!("{} insert", o.name))
        }
        None => {
            let kind = args.get("kind").and_then(Value::as_str).unwrap_or("sphere");
            let p = match kind {
                "sphere" => Primitive::Sphere { radius: 0.5, subdivisions: 4 },
                "cube" => Primitive::Cube { size: 1.0, divisions: 8 },
                "cylinder" => Primitive::Cylinder { radius: 0.5, height: 1.0, segments: 48, rings: 12 },
                "cone" => Primitive::Cone { radius: 0.5, height: 1.0, segments: 48 },
                "torus" => Primitive::Torus { major: 0.35, minor: 0.15, segments: 48, sides: 24 },
                other => return Err(bad(format!("unknown kind {other}"))),
            };
            let mut c = kind.to_string();
            if let Some(f) = c.get_mut(0..1) {
                f.make_ascii_uppercase();
            }
            (p.build(), c)
        }
    })
}

pub(crate) fn insert_along_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "array", "minItems": 1, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 6, "maxItems": 6}, "description": "The stroke over the surface as [x, y, z, nx, ny, nz] points with their normals (pick gives both), in order."},
            "kind": {"type": "string", "enum": ["sphere", "cube", "cylinder", "cone", "torus"], "default": "sphere", "description": "A primitive to insert, unless source_id is given."},
            "source_id": {"type": "integer", "minimum": 1, "description": "Insert copies of this object instead (scales, studs, spikes, chain links, teeth)."},
            "size": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "default": 0.2, "description": "Each copy's largest dimension, in world units."},
            "spacing": {"type": "number", "minimum": 0.1, "maximum": 20, "default": 1.2, "description": "Distance between copies along the stroke, as a multiple of size."},
            "sink": {"type": "number", "minimum": -1, "maximum": 1, "default": 0.25, "description": "How far each copy sinks into the surface, as a fraction of its height."},
            "align": {"type": "boolean", "default": true, "description": "Turn each copy so its x follows the stroke (a chain, a row of scales)."},
            "merge": {"type": "boolean", "default": true, "description": "Make all copies one object; false gives one object per copy."}
        },
        "required": ["path"],
        "additionalProperties": false
    })
}

/// Insert mesh along a stroke: copies strung over the surface at even spacing, each standing on its normal.
pub(crate) fn insert_along(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let path: Vec<(Vec3, Vec3)> = args
        .get("path")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("path is a list of [x, y, z, nx, ny, nz] points"))?
        .iter()
        .map(|p| {
            let f: Vec<f32> = p.as_array().map(|a| a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect()).unwrap_or_default();
            if f.len() != 6 || f.iter().any(|x| !x.is_finite()) {
                return Err(bad("each path point is [x, y, z, nx, ny, nz]"));
            }
            Ok((Vec3::new(f[0], f[1], f[2]), Vec3::new(f[3], f[4], f[5]).normalize_or(Vec3::Y)))
        })
        .collect::<Result<_, _>>()?;
    if path.is_empty() {
        return Err(bad("path needs at least one point"));
    }
    let size = args.get("size").and_then(Value::as_f64).unwrap_or(0.2) as f32;
    let spacing = args.get("spacing").and_then(Value::as_f64).unwrap_or(1.2).clamp(0.1, 20.0) as f32;
    let sink = args.get("sink").and_then(Value::as_f64).unwrap_or(0.25) as f32;
    if !(size > 0.0 && size.is_finite()) || !(-1.0..=1.0).contains(&sink) {
        return Err(bad("size must be above 0 and sink -1 to 1"));
    }
    let align = args.get("align").and_then(Value::as_bool).unwrap_or(true);
    let merge = args.get("merge").and_then(Value::as_bool).unwrap_or(true);
    let (source, name) = insert_source(s, args)?;
    // Places along the stroke every `step` of arc length, normals blended between samples.
    let step = size * spacing;
    let mut places: Vec<(Vec3, Vec3, Vec3)> = Vec::new();
    let tangent_at = |i: usize| -> Vec3 {
        let (a, b) = (path[i.saturating_sub(1)].0, path[(i + 1).min(path.len() - 1)].0);
        (b - a).normalize_or(Vec3::X)
    };
    places.push((path[0].0, path[0].1, tangent_at(0)));
    let mut carried = 0.0;
    for i in 1..path.len() {
        let (a, b) = (path[i - 1], path[i]);
        let len = (b.0 - a.0).length();
        let mut at = step - carried;
        while at <= len {
            let t = at / len.max(1e-9);
            places.push((a.0.lerp(b.0, t), a.1.lerp(b.1, t).normalize_or(Vec3::Y), (b.0 - a.0).normalize_or(Vec3::X)));
            at += step;
        }
        carried = (carried + len) % step;
        if places.len() > 2000 {
            return Err(bad("that would place more than 2000 copies; raise spacing or size"));
        }
    }
    let b = source.bounds();
    let scale = size / b.extent().max_element().max(1e-9);
    let base = Vec3::new(b.center().x, b.min.y, b.center().z);
    let height = b.extent().y * scale;
    let copy = |(point, normal, dir): (Vec3, Vec3, Vec3)| -> Mesh {
        let stand = Quat::from_rotation_arc(Vec3::Y, normal);
        // Turn about the normal so the copy's x follows the stroke across the surface.
        let rot = if align {
            let along = (dir - normal * dir.dot(normal)).normalize_or(stand * Vec3::X);
            let x_now = stand * Vec3::X;
            let angle = x_now.cross(along).dot(normal).atan2(x_now.dot(along));
            Quat::from_axis_angle(normal, angle) * stand
        } else {
            stand
        };
        let place = point - normal * (height * sink);
        let positions = source.positions.iter().map(|&p| (rot * ((Vec3::from(p) - base) * scale) + place).to_array()).collect();
        let mut m = Mesh::new(positions, source.indices.clone());
        m.paint = source.paint.clone();
        m
    };
    let count = places.len();
    if merge {
        let parts: Vec<Mesh> = places.into_iter().map(copy).collect();
        let mesh = fsc_topo::merge(&parts.iter().collect::<Vec<_>>());
        let id = add_object(s, &format!("{name} row"), mesh, true);
        return Ok(json!({"object": summary(s.scene.get(id).expect("added")), "active": id, "copies": count}));
    }
    let mut ids = Vec::with_capacity(count);
    for p in places {
        let m = copy(p);
        ids.push(add_object(s, &name, m, true));
    }
    Ok(json!({"ids": ids, "active": ids.last(), "copies": count}))
}

pub(crate) fn insert_mesh(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let point = vec3(args, "point")?.ok_or_else(|| bad("point is required"))?;
    let normal = vec3(args, "normal")?.unwrap_or(Vec3::Y).normalize_or(Vec3::Y);
    let size = args.get("size").and_then(Value::as_f64).unwrap_or(0.2) as f32;
    let sink = args.get("sink").and_then(Value::as_f64).unwrap_or(0.25) as f32;
    if !(size > 0.0 && size.is_finite()) || !(-1.0..=1.0).contains(&sink) {
        return Err(bad("size must be above 0 and sink -1 to 1"));
    }
    let (source, name) = insert_source(s, args)?;
    // Scale to `size`, stand it on its local y along the normal, and sink it by a fraction of its height.
    let b = source.bounds();
    let scale = size / b.extent().max_element().max(1e-9);
    let rot = Quat::from_rotation_arc(Vec3::Y, normal);
    let base = Vec3::new(b.center().x, b.min.y, b.center().z);
    let height = b.extent().y * scale;
    let place = point - normal * (height * sink);
    let positions = source.positions.iter().map(|&p| (rot * ((Vec3::from(p) - base) * scale) + place).to_array()).collect();
    let mut mesh = Mesh::new(positions, source.indices.clone());
    mesh.paint = source.paint.clone();
    if source.has_groups() {
        mesh.groups = source.groups.clone();
    }
    let id = add_object(s, &name, mesh, true);
    Ok(json!({"object": summary(s.scene.get(id).expect("added")), "active": id}))
}
