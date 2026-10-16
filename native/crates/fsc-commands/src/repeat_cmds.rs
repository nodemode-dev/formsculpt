use std::collections::HashMap;

use fsc_core::Mesh;
use glam::{EulerRot, Mat4, Quat, Vec3};
use serde_json::{json, Value};

use crate::history::Edit;
use crate::place_cmds::{add_object, number_in, transformed, vec3, SplitMix};
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

const MAX_COPIES: usize = 2048;
const MAX_TRIANGLES: usize = 20_000_000;
const REPORTED: usize = 64;

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn unit(r: &mut SplitMix) -> f32 {
    (r.next() >> 40) as f32 / (1u64 << 24) as f32
}

fn unit64(r: &mut SplitMix) -> f64 {
    (r.next() >> 11) as f64 / (1u64 << 53) as f64
}

fn round(x: f32) -> f64 {
    (x as f64 * 1.0e5).round() / 1.0e5
}

fn seed_of(args: &Value) -> Result<u64, CommandError> {
    match args.get("seed") {
        None | Some(Value::Null) => Ok(1),
        Some(v) => v.as_i64().map(|x| x as u64).or_else(|| v.as_u64()).ok_or_else(|| bad("seed must be an integer")),
    }
}

fn validate_of(args: &Value) -> Result<String, CommandError> {
    match args.get("validate") {
        None | Some(Value::Null) => Ok("merge".into()),
        Some(v) => match v.as_str() {
            Some(m @ ("merge" | "separate" | "linked" | "preview")) => Ok(m.into()),
            _ => Err(bad("validate must be merge, separate, linked or preview")),
        },
    }
}

fn axis_of(obj: &Value, default: &str) -> Result<usize, CommandError> {
    match obj.get("axis") {
        None | Some(Value::Null) => Ok(["x", "y", "z"].iter().position(|a| *a == default).expect("default axis")),
        Some(v) => match v.as_str() {
            Some("x") => Ok(0),
            Some("y") => Ok(1),
            Some("z") => Ok(2),
            _ => Err(bad("axis must be x, y or z")),
        },
    }
}

fn describe(m: Mat4, centre: Vec3) -> Value {
    let (scale, rot, _) = m.to_scale_rotation_translation();
    let (x, y, z) = rot.to_euler(EulerRot::XYZ);
    let at = m.transform_point3(centre);
    json!({
        "position": [round(at.x), round(at.y), round(at.z)],
        "rotation": [round(x.to_degrees()), round(y.to_degrees()), round(z.to_degrees())],
        "scale": [round(scale.x), round(scale.y), round(scale.z)],
        "matrix": m.to_cols_array().iter().map(|&v| round(v)).collect::<Vec<_>>()
    })
}

fn scene_info(s: &Session) -> Value {
    json!({"objects": s.scene.objects.len(), "active": s.scene.active})
}

fn too_big(copies: usize, triangles: usize) -> Result<(), CommandError> {
    if copies.saturating_mul(triangles) > MAX_TRIANGLES {
        return Err(bad(format!("that would build more than {MAX_TRIANGLES} triangles; use fewer copies or a lighter source")));
    }
    Ok(())
}

fn place_copies(s: &mut Session, src: u32, label: &str, transforms: &[Mat4], validate: &str, remove_source: bool, word: &str) -> Vec<u32> {
    let o = s.scene.get(src).expect("checked");
    let name = o.name.clone();
    let revision = o.revision;
    let painted = o.mesh.has_paint();
    let meshes: Vec<Mesh> = transforms.iter().map(|m| transformed(&o.mesh, *m)).collect();
    let previous = s.scene.active;
    s.history.push(Edit::Scene { label: label.into(), objects: s.scene.objects.clone(), active: previous });
    if validate == "merge" {
        let mut mesh = fsc_topo::merge(&meshes.iter().collect::<Vec<_>>());
        if painted {
            mesh.paint = meshes.iter().flat_map(|m| m.paint.clone()).collect();
        }
        if remove_source {
            s.scene.remove(src);
        }
        let id = add_object(s, &format!("{name} {word}"), mesh, false);
        s.scene.active = Some(id);
        return vec![id];
    }
    let ids: Vec<u32> = meshes.into_iter().enumerate().map(|(k, m)| add_object(s, &format!("{name} {word} {}", k + 1), m, false)).collect();
    if validate == "linked" {
        for (&cid, m) in ids.iter().zip(transforms) {
            let copy = s.scene.get_mut(cid).expect("just added");
            copy.instance = Some(fsc_core::Instance { source: src, matrix: m.to_cols_array(), source_revision: revision, own_revision: copy.revision });
        }
    }
    s.scene.active = previous;
    ids
}

fn finish(s: &mut Session, id: u32, word: &str, validate: &str, transforms: &[Mat4], centre: Vec3) -> Value {
    let label = if word == "repeat" { "Repeat" } else { "Scatter" };
    let ids = if validate == "preview" { Vec::new() } else { place_copies(s, id, label, transforms, validate, word == "repeat", word) };
    let shown: Vec<Value> = transforms.iter().take(REPORTED).map(|m| describe(*m, centre)).collect();
    let mut reply = json!({"copies": transforms.len(), "objects": ids, "transforms": shown, "validate": validate, "scene": scene_info(s)});
    if validate == "merge" {
        reply["object"] = summary(s.scene.get(ids[0]).expect("added"));
    }
    reply
}

fn randomize_of(step: &Value) -> Result<Option<(f32, f32, f32)>, CommandError> {
    match step.get("randomize") {
        None | Some(Value::Null) => Ok(None),
        Some(r @ Value::Object(m)) => {
            if let Some(k) = m.keys().find(|k| !["position", "rotation", "scale"].contains(&k.as_str())) {
                return Err(bad(format!("randomize has no field {k}; use position, rotation and scale")));
            }
            Ok(Some((number_in(r, "position", 0.0, 1.0e6, 0.0)?, number_in(r, "rotation", 0.0, 180.0, 0.0)?, number_in(r, "scale", 0.0, 0.95, 0.0)?)))
        }
        Some(_) => Err(bad("randomize is an object {position, rotation, scale}")),
    }
}

fn step_transforms(step: &Value, index: usize, centre: Vec3, rng: &mut SplitMix) -> Result<Vec<Mat4>, CommandError> {
    let obj = step.as_object().ok_or_else(|| bad(format!("steps[{index}] must be an object")))?;
    let kind = step.get("kind").and_then(Value::as_str).ok_or_else(|| bad(format!("steps[{index}] needs a kind: line, radial, mirror or curve")))?;
    let allowed: &[&str] = match kind {
        "line" => &["kind", "count", "offset", "randomize"],
        "radial" => &["kind", "count", "axis", "angle", "center", "randomize"],
        "mirror" => &["kind", "axis", "offset", "randomize"],
        "curve" => &["kind", "points", "count", "align", "interp", "closed", "randomize"],
        other => return Err(bad(format!("steps[{index}] has unknown kind {other}; use line, radial, mirror or curve"))),
    };
    if let Some(k) = obj.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(bad(format!("steps[{index}] ({kind}) has no field {k}")));
    }
    let count = || -> Result<usize, CommandError> {
        step.get("count").and_then(Value::as_u64).filter(|c| (2..=64).contains(c)).map(|c| c as usize).ok_or_else(|| bad(format!("steps[{index}] needs count from 2 to 64")))
    };
    let mut out: Vec<Mat4> = Vec::new();
    let mut from = 1;
    match kind {
        "line" => {
            let offset = vec3(step, "offset")?.ok_or_else(|| bad(format!("steps[{index}] (line) needs offset: [x, y, z]")))?;
            out.extend((0..count()?).map(|k| Mat4::from_translation(offset * k as f32)));
        }
        "radial" => {
            let n = count()?;
            let axis = [Vec3::X, Vec3::Y, Vec3::Z][axis_of(step, "y")?];
            let pivot = vec3(step, "center")?.unwrap_or(centre);
            let total = number_in(step, "angle", -360.0, 360.0, 360.0)?;
            let full = (total.abs() - 360.0).abs() < 1e-3;
            let each = total.to_radians() / if full { n as f32 } else { (n - 1) as f32 };
            out.extend((0..n).map(|k| Mat4::from_translation(pivot) * Mat4::from_axis_angle(axis, each * k as f32) * Mat4::from_translation(-pivot)));
        }
        "mirror" => {
            let a = axis_of(step, "x")?;
            let plane = number_in(step, "offset", -1.0e6, 1.0e6, 0.0)?;
            let mut at = Vec3::ZERO;
            at[a] = plane;
            let mut flip = Vec3::ONE;
            flip[a] = -1.0;
            out.push(Mat4::IDENTITY);
            out.push(Mat4::from_translation(at) * Mat4::from_scale(flip) * Mat4::from_translation(-at));
        }
        _ => {
            let n = count()?;
            let mut curve_args = step.clone();
            if curve_args.get("interp").is_none() {
                curve_args["interp"] = json!("linear");
            }
            let points = crate::curve_cmds::path_positions(&curve_args, "points", "a curve step needs points: [[x, y, z], ...]")?;
            if points.len() < 2 {
                return Err(bad("a curve step needs at least two points"));
            }
            let align = step.get("align").and_then(Value::as_bool).unwrap_or(true);
            let lengths: Vec<f32> = points.windows(2).map(|w| w[0].distance(w[1])).collect();
            let total: f32 = lengths.iter().sum();
            if total <= 1e-9 {
                return Err(bad("the curve has no length"));
            }
            for k in 0..n {
                let want = total * k as f32 / (n - 1) as f32;
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
                out.push(Mat4::from_translation(at) * Mat4::from_quat(turn) * Mat4::from_translation(-centre));
            }
            from = 0;
        }
    }
    if let Some((position, rotation, scale)) = randomize_of(step)? {
        for m in out.iter_mut().skip(from) {
            let shift = Vec3::new(rng.signed(), rng.signed(), rng.signed()) * position;
            let axis = Vec3::new(rng.signed(), rng.signed(), rng.signed()).normalize_or(Vec3::Y);
            let turn = Quat::from_axis_angle(axis, (rng.signed() * rotation).to_radians());
            let size = 1.0 + rng.signed() * scale;
            let local = Mat4::from_translation(centre) * Mat4::from_quat(turn) * Mat4::from_scale(Vec3::splat(size)) * Mat4::from_translation(-centre);
            *m = Mat4::from_translation(shift) * *m * local;
        }
    }
    Ok(out)
}

pub(crate) fn repeat_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The source object to repeat; the active object when omitted."},
            "steps": {
                "type": "array",
                "minItems": 1,
                "maxItems": 4,
                "description": "The repeaters as an ordered stack, the first innermost: step 2 repeats the whole set step 1 made, and so on, so the copy counts multiply (at most 2048 in all, the source included).",
                "items": {
                    "type": "object",
                    "properties": {
                        "kind": {"type": "string", "enum": ["line", "radial", "mirror", "curve"], "description": "line steps by offset; radial turns about an axis; mirror doubles the set with a reflected copy; curve spaces copies evenly by arc length along points."},
                        "count": {"type": "integer", "minimum": 2, "maximum": 64, "description": "Line, radial and curve: copies this step makes of the set, the set itself included."},
                        "offset": {"description": "Line: the step between copies as [x, y, z] in world units (required). Mirror: the plane's position along the axis, a number (default 0).", "oneOf": [{"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}, {"type": "number", "default": 0}]},
                        "axis": {"type": "string", "enum": ["x", "y", "z"], "description": "Radial: the axis the copies turn about (default y). Mirror: the axis the mirror plane is square to (default x)."},
                        "angle": {"type": "number", "minimum": -360, "maximum": 360, "default": 360, "description": "Radial: the total turn in degrees; 360 spaces the copies round the full circle without repeating the first."},
                        "center": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Radial: the point the axis passes through; the source's bounds centre when omitted."},
                        "points": {"type": "array", "minItems": 2, "maxItems": 4096, "items": crate::curve_cmds::point_schema(3), "description": "Curve: the path as [x, y, z] points; the source's bounds centre sits on it."},
                        "align": {"type": "boolean", "default": true, "description": "Curve: turn each copy so the source's +x follows the path."},
                        "interp": {"type": "string", "enum": ["linear", "catmull_rom", "bspline"], "default": "linear", "description": "Curve: how the path runs between points (a plain polyline by default)."},
                        "closed": {"type": "boolean", "default": false, "description": "Curve: loop the path back to its first point."},
                        "randomize": {
                            "type": "object",
                            "description": "Random variation for each copy this step creates (the set it repeats is untouched); the same seed gives the same result.",
                            "properties": {
                                "position": {"type": "number", "minimum": 0, "default": 0, "description": "Each copy moves by up to this many world units either way on each axis."},
                                "rotation": {"type": "number", "minimum": 0, "maximum": 180, "default": 0, "description": "Each copy turns about a random axis by up to this many degrees either way."},
                                "scale": {"type": "number", "minimum": 0, "maximum": 0.95, "default": 0, "description": "Each copy's uniform scale is multiplied by 1 plus or minus up to this fraction."}
                            },
                            "additionalProperties": false
                        }
                    },
                    "required": ["kind"],
                    "additionalProperties": false
                }
            },
            "seed": {"type": "integer", "default": 1, "description": "The random seed for every randomize."},
            "validate": {"type": "string", "enum": ["merge", "separate", "linked", "preview"], "default": "merge", "description": "merge: all copies, the source included, become one new object and the source is removed. separate: each copy is a new object and the source stays. linked: like separate, and the copies follow the source when it changes. preview: nothing changes; the reply only gives the transforms."}
        },
        "required": ["steps"],
        "additionalProperties": false
    })
}

/// Nested repeaters (line, radial, mirror, curve) applied as a stack to a source object, merged, separate, linked or only previewed.
pub(crate) fn repeat_object(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let steps = args.get("steps").and_then(Value::as_array).ok_or_else(|| bad("steps is required: 1 to 4 step objects"))?;
    if !(1..=4).contains(&steps.len()) {
        return Err(bad("steps holds 1 to 4 step objects"));
    }
    let seed = seed_of(args)?;
    let validate = validate_of(args)?;
    let o = s.scene.get(id).expect("checked");
    let centre = o.mesh.bounds().center();
    let triangles = o.mesh.triangle_count();
    let mut rng = SplitMix(seed);
    let mut set = vec![Mat4::IDENTITY];
    for (i, step) in steps.iter().enumerate() {
        let outer = step_transforms(step, i, centre, &mut rng)?;
        if set.len().saturating_mul(outer.len()) > MAX_COPIES {
            return Err(bad(format!("that would make {} copies; the limit is {MAX_COPIES}", set.len() * outer.len())));
        }
        set = outer.iter().flat_map(|a| set.iter().map(move |b| *a * *b)).collect();
    }
    too_big(set.len(), triangles)?;
    Ok(finish(s, id, "repeat", &validate, &set, centre))
}

pub(crate) fn scatter_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The source object to scatter; the active object when omitted."},
            "target": {"type": "integer", "minimum": 1, "description": "The object whose surface the copies land on; it must differ from the source."},
            "count": {"type": "integer", "minimum": 1, "maximum": 2048, "default": 100, "description": "Copies to place; fewer are placed when min_distance leaves no room, and the reply says how many."},
            "seed": {"type": "integer", "default": 1, "description": "The random seed; the same seed gives the same scatter."},
            "align": {"type": "string", "enum": ["normal", "up"], "default": "normal", "description": "normal turns each copy so its +Y follows the surface normal; up keeps it upright."},
            "scale_min": {"type": "number", "minimum": 0.01, "maximum": 100, "default": 1, "description": "Smallest uniform scale of a copy."},
            "scale_max": {"type": "number", "minimum": 0.01, "maximum": 100, "default": 1, "description": "Largest uniform scale of a copy; each copy takes a random scale between the two."},
            "spin": {"type": "number", "minimum": 0, "maximum": 360, "default": 360, "description": "Random turn about the aligned axis, up to this many degrees."},
            "min_distance": {"type": "number", "minimum": 0, "default": 0, "description": "The least distance between two copies' surface points, in world units; found by rejection sampling with a bounded number of tries."},
            "offset": {"type": "number", "default": 0, "description": "How far each copy's bounds centre sits off the surface along the normal, in world units."},
            "mask_only": {"type": "boolean", "default": false, "description": "Place only where the target's mask is set (the mask interpolated at the sample is at least 0.5); refused when the target has no masked vertex."},
            "validate": {"type": "string", "enum": ["merge", "separate", "linked", "preview"], "default": "merge", "description": "merge: all copies become one new object, the source and target untouched. separate: one new object per copy. linked: like separate, and the copies follow the source when it changes. preview: nothing changes; the reply only gives the transforms."}
        },
        "required": ["target"],
        "additionalProperties": false
    })
}

/// Scatters copies of the source over the target's surface, sampled by triangle area, with optional spacing, scale, spin and mask.
pub(crate) fn scatter_on_surface(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let tid = args.get("target").and_then(Value::as_u64).ok_or_else(|| bad("target is required: the object to scatter over"))? as u32;
    if tid == id {
        return Err(bad("target must differ from the source"));
    }
    let t = s.scene.get(tid).ok_or_else(|| CommandError::new("not_found", format!("no object {tid}; get_scene lists object ids")))?;
    let count = match args.get("count") {
        None | Some(Value::Null) => 100,
        Some(v) => v.as_u64().filter(|c| (1..=MAX_COPIES as u64).contains(c)).ok_or_else(|| bad("count must be 1 to 2048"))? as usize,
    };
    let seed = seed_of(args)?;
    let upright = match args.get("align").and_then(Value::as_str) {
        None | Some("normal") => false,
        Some("up") => true,
        Some(_) => return Err(bad("align must be normal or up")),
    };
    let scale_min = number_in(args, "scale_min", 0.01, 100.0, 1.0)?;
    let scale_max = number_in(args, "scale_max", 0.01, 100.0, 1.0)?;
    if scale_max < scale_min {
        return Err(bad("scale_max must be at least scale_min"));
    }
    let spin = number_in(args, "spin", 0.0, 360.0, 360.0)?;
    let min_distance = number_in(args, "min_distance", 0.0, 1.0e6, 0.0)?;
    let offset = number_in(args, "offset", -1.0e6, 1.0e6, 0.0)?;
    let mask_only = args.get("mask_only").and_then(Value::as_bool).unwrap_or(false);
    let validate = validate_of(args)?;
    let mesh = &t.mesh;
    let masked = mask_only && t.mask.len() == mesh.vertex_count();
    if mask_only && !(masked && t.mask.iter().any(|&m| m > 0.5)) {
        return Err(CommandError::new("no_mask", "mask_only needs a masked area on the target; mask some of it first or leave mask_only off"));
    }
    let tri = |k: usize| [mesh.indices[3 * k] as usize, mesh.indices[3 * k + 1] as usize, mesh.indices[3 * k + 2] as usize];
    let corner = |i: usize| Vec3::from(mesh.positions[i]);
    let mut cumulative: Vec<f64> = Vec::with_capacity(mesh.triangle_count());
    let mut weights: Vec<f64> = Vec::with_capacity(mesh.triangle_count());
    let mut total = 0.0f64;
    for k in 0..mesh.triangle_count() {
        let [a, b, c] = tri(k);
        let hidden = mesh.has_hidden() && mesh.hidden.get(k).copied().unwrap_or(false);
        let off_mask = masked && ![a, b, c].iter().any(|&i| t.mask[i] > 0.5);
        let w = if hidden || off_mask { 0.0 } else { 0.5 * (corner(b) - corner(a)).cross(corner(c) - corner(a)).length() as f64 };
        total += w;
        weights.push(w);
        cumulative.push(total);
    }
    if total <= 0.0 {
        return Err(CommandError::new("no_surface", "the target has no surface area to scatter over"));
    }
    let source = s.scene.get(id).expect("checked");
    let centre = source.mesh.bounds().center();
    too_big(count, source.mesh.triangle_count())?;
    let mut rng = SplitMix(seed);
    let mut transforms: Vec<Mat4> = Vec::new();
    let mut grid: HashMap<(i32, i32, i32), Vec<Vec3>> = HashMap::new();
    let cell = |p: Vec3| ((p.x / min_distance).floor() as i32, (p.y / min_distance).floor() as i32, (p.z / min_distance).floor() as i32);
    let limit = count * 30 + 100;
    let mut tries = 0;
    while transforms.len() < count && tries < limit {
        tries += 1;
        let pick = unit64(&mut rng) * total;
        let k = cumulative.partition_point(|&c| c <= pick).min(weights.len() - 1);
        let (r1, r2) = (unit(&mut rng), unit(&mut rng));
        if weights[k] <= 0.0 {
            continue;
        }
        let [a, b, c] = tri(k);
        let su = r1.sqrt();
        let bary = [1.0 - su, su * (1.0 - r2), su * r2];
        let ids = [a, b, c];
        let point = corner(a) * bary[0] + corner(b) * bary[1] + corner(c) * bary[2];
        if masked && (0..3).map(|j| t.mask[ids[j]] * bary[j]).sum::<f32>() < 0.5 {
            continue;
        }
        let face = (corner(b) - corner(a)).cross(corner(c) - corner(a)).normalize_or(Vec3::Y);
        let normal = (0..3).map(|j| Vec3::from(mesh.normals[ids[j]]) * bary[j]).sum::<Vec3>().normalize_or(face);
        if min_distance > 0.0 {
            let (cx, cy, cz) = cell(point);
            let crowded = (-1..=1).any(|dx| {
                (-1..=1).any(|dy| (-1..=1).any(|dz| grid.get(&(cx + dx, cy + dy, cz + dz)).is_some_and(|v| v.iter().any(|q| q.distance(point) < min_distance))))
            });
            if crowded {
                continue;
            }
            grid.entry((cx, cy, cz)).or_default().push(point);
        }
        let size = scale_min + unit(&mut rng) * (scale_max - scale_min);
        let turn = (unit(&mut rng) * spin).to_radians();
        let rot = if upright { Quat::from_axis_angle(Vec3::Y, turn) } else { Quat::from_axis_angle(normal, turn) * Quat::from_rotation_arc(Vec3::Y, normal) };
        transforms.push(Mat4::from_translation(point + normal * offset) * Mat4::from_quat(rot) * Mat4::from_scale(Vec3::splat(size)) * Mat4::from_translation(-centre));
    }
    if transforms.is_empty() {
        return Err(CommandError::new("none_placed", "no copy could be placed; check the mask and min_distance"));
    }
    let mut reply = finish(s, id, "scatter", &validate, &transforms, centre);
    reply["placed"] = json!(transforms.len());
    reply["requested"] = json!(count);
    Ok(reply)
}
