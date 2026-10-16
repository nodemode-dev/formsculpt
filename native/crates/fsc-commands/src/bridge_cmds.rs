use fsc_core::border::{border_loops, faces_by_mask, faces_in_group, resample_border};
use fsc_core::time::Instant;
use fsc_core::{Curve, Interp, Mesh};
use glam::{Vec2, Vec3};
use serde_json::{json, Value};

use crate::build_cmds::add;
use crate::curve_cmds::{number, parse_points};
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

struct Section {
    points: Vec<Vec3>,
    normal: Vec3,
}

struct Frame {
    pos: Vec3,
    n: Vec3,
    b: Vec3,
}

fn face_set_schema(what: &str) -> Value {
    json!({
        "type": "object",
        "properties": {
            "source": {"type": "string", "enum": ["group", "mask"], "description": "Take the faces in one face group, or the faces whose three vertices have a mean mask of 0.5 or more."},
            "group": {"type": "integer", "minimum": 0, "description": "The face group id; required when source is group."}
        },
        "required": ["source"],
        "additionalProperties": false,
        "description": what
    })
}

pub(crate) fn bridge_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The source object; the active object when omitted."},
            "from": face_set_schema("The start loop: the longest border of this face set."),
            "to": face_set_schema("The end loop: the longest border of this face set. Without it the end section is the start loop scaled by end_scale (a tapering extrusion)."),
            "points": {"type": "array", "minItems": 2, "maxItems": 4096, "items": crate::curve_cmds::point_schema(3), "description": "The path between the loops, in the same point formats as sample_curve. Without it the path is the straight line between the two loop centres, or without to a straight line along the start loop's average normal."},
            "length": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000000, "description": "Without to and points: how far to extrude along the normal, in world units. Four times the loop's mean radius when omitted."},
            "end_scale": {"type": "number", "minimum": 0.001, "maximum": 100, "default": 0.3, "description": "Without to: the end section is the start loop scaled about its centre by this factor."},
            "sections": {"type": "integer", "minimum": 4, "maximum": 512, "default": 48, "description": "Rings along the path."},
            "samples": {"type": "integer", "minimum": 8, "maximum": 256, "default": 48, "description": "Points around each loop."},
            "offset": {"type": "number", "default": 0, "description": "Lift each loop along the surface normal by this distance, in world units."},
            "name": {"type": "string", "description": "Name for the new object; a default when omitted."}
        },
        "required": ["from"],
        "additionalProperties": false
    })
}

fn section_of(s: &Session, id: u32, spec: Option<&Value>, key: &str, count: usize, offset: f32) -> Result<Section, CommandError> {
    let spec = spec.filter(|v| v.is_object()).ok_or_else(|| bad(format!("{key} is an object with source group or mask")))?;
    for k in spec.as_object().into_iter().flat_map(|o| o.keys()) {
        if !matches!(k.as_str(), "source" | "group") {
            return Err(bad(format!("{key} has no field {k}; use source and group")));
        }
    }
    let object = s.scene.get(id).expect("checked");
    let mesh = &object.mesh;
    let faces = match spec.get("source").and_then(Value::as_str) {
        Some("group") => {
            let group = spec.get("group").and_then(Value::as_u64).filter(|g| *g <= u32::MAX as u64).ok_or_else(|| bad(format!("{key}.group is required when source is group: a whole number")))?;
            if !mesh.has_groups() {
                return Err(CommandError::new("no_groups", "the object has no face groups; make some with group_from_mask or group_auto"));
            }
            faces_in_group(mesh, group as u32)
        }
        Some("mask") => {
            if object.mask.len() != mesh.vertex_count() {
                return Err(CommandError::new("no_mask", "the object has no mask"));
            }
            faces_by_mask(mesh, &object.mask, 0.5)
        }
        _ => return Err(bad(format!("{key}.source must be group or mask"))),
    };
    let loops = border_loops(mesh, &faces);
    let Some(longest) = loops.first() else {
        return Err(CommandError::new("no_border", format!("the {key} face set is empty or covers the whole closed mesh, so it has no border")));
    };
    if !longest.closed {
        return Err(bad(format!("the longest border of the {key} face set is open; a bridge needs a closed loop, so close the set or pick another")));
    }
    let points = resample_border(mesh, longest, count, offset);
    let sum: Vec3 = longest.vertices.iter().map(|v| mesh.normals.get(*v as usize).map(|n| Vec3::from_array(*n)).unwrap_or(Vec3::ZERO)).sum();
    let normal = sum.try_normalize().or_else(|| newell(&points)).unwrap_or(Vec3::Y);
    Ok(Section { points, normal })
}

fn newell(points: &[Vec3]) -> Option<Vec3> {
    let c = centre(points);
    let n = points.len();
    (0..n).map(|i| (points[i] - c).cross(points[(i + 1) % n] - c)).sum::<Vec3>().try_normalize()
}

fn centre(points: &[Vec3]) -> Vec3 {
    points.iter().sum::<Vec3>() / points.len().max(1) as f32
}

fn mean_radius(points: &[Vec3]) -> f32 {
    let c = centre(points);
    points.iter().map(|p| p.distance(c)).sum::<f32>() / points.len().max(1) as f32
}

fn by_arc_length(dense: &[(Vec3, Vec3)], count: usize) -> Vec<(Vec3, Vec3)> {
    let mut cum = vec![0.0f32];
    for w in dense.windows(2) {
        cum.push(cum[cum.len() - 1] + w[0].0.distance(w[1].0));
    }
    let total = cum[cum.len() - 1];
    let last = dense.len() - 1;
    (0..count)
        .map(|k| {
            let d = total * k as f32 / (count - 1) as f32;
            let i = cum.partition_point(|c| *c <= d).clamp(1, last) - 1;
            let u = ((d - cum[i]) / (cum[i + 1] - cum[i]).max(1e-12)).clamp(0.0, 1.0);
            let t = dense[i].1.lerp(dense[i + 1].1, u).try_normalize().unwrap_or(dense[i].1);
            (dense[i].0.lerp(dense[i + 1].0, u), t)
        })
        .collect()
}

fn frames(path: &[(Vec3, Vec3)]) -> Vec<Frame> {
    let mut n = path[0].1.any_orthonormal_vector();
    path.iter()
        .map(|(pos, t)| {
            n = (n - *t * n.dot(*t)).try_normalize().unwrap_or_else(|| t.any_orthonormal_vector());
            Frame { pos: *pos, n, b: t.cross(n) }
        })
        .collect()
}

fn local(frame: &Frame, p: Vec3) -> Vec3 {
    let d = p - frame.pos;
    Vec3::new(d.dot(frame.n), d.dot(frame.b), d.dot(frame.n.cross(frame.b)))
}

fn area(points: &[Vec3]) -> f32 {
    let n = points.len();
    (0..n).map(|i| Vec2::new(points[i].x, points[i].y).perp_dot(Vec2::new(points[(i + 1) % n].x, points[(i + 1) % n].y))).sum::<f32>() * 0.5
}

fn align(start: &[Vec3], end: &mut [Vec3]) {
    if area(start) * area(end) < 0.0 {
        end.reverse();
    }
    let n = start.len();
    let (cs, ce) = (centre(start), centre(end));
    let cost = |shift: usize| (0..n).map(|i| (start[i] - cs).distance_squared(end[(i + shift) % n] - ce)).sum::<f32>();
    let best = (0..n).min_by(|a, b| cost(*a).total_cmp(&cost(*b))).unwrap_or(0);
    end.rotate_left(best);
}

pub(crate) fn curve_bridge(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let id = target(s, args)?;
    let sections = number(args, "sections", 48.0, 4.0, 512.0)?.round() as usize;
    let samples = number(args, "samples", 48.0, 8.0, 256.0)?.round() as usize;
    let offset = number(args, "offset", 0.0, -1e6, 1e6)? as f32;
    let end_scale = number(args, "end_scale", 0.3, 0.001, 100.0)? as f32;
    let length = match args.get("length") {
        None | Some(Value::Null) => None,
        Some(_) => Some(number(args, "length", 1.0, 1e-9, 1e6)? as f32),
    };
    let from = section_of(s, id, args.get("from"), "from", samples, offset)?;
    let to = match args.get("to").filter(|v| !v.is_null()) {
        Some(spec) => Some(section_of(s, id, Some(spec), "to", samples, offset)?),
        None => None,
    };
    let (ca, cb) = (centre(&from.points), to.as_ref().map(|t| centre(&t.points)));

    let dense: Vec<(Vec3, Vec3)> = match args.get("points").filter(|v| !v.is_null()) {
        Some(list) => {
            let list = list.as_array().ok_or_else(|| bad("points is a list of curve points"))?;
            let points = parse_points(list, 3, 1.0, false, "point")?;
            let curve = Curve::new(points, Interp::CatmullRom, false);
            let samples = curve.sample(32);
            if samples.len() < 2 {
                return Err(bad("the path needs at least two distinct points"));
            }
            samples.iter().map(|c| (c.pos, c.tangent)).collect()
        }
        None => {
            let (end, dir) = match cb {
                Some(cb) => (cb, (cb - ca).try_normalize().ok_or_else(|| bad("the two loops have the same centre; give points for the path"))?),
                None => {
                    let reach = length.unwrap_or_else(|| mean_radius(&from.points) * 4.0);
                    (ca + from.normal * reach, from.normal)
                }
            };
            vec![(ca, dir), (end, dir)]
        }
    };
    if dense.windows(2).map(|w| w[0].0.distance(w[1].0)).sum::<f32>() <= 1e-9 {
        return Err(bad("the path has no length"));
    }
    let path = by_arc_length(&dense, sections + 1);
    let frames = frames(&path);
    let (first, last) = (&frames[0], &frames[sections]);

    let start: Vec<Vec3> = from.points.iter().map(|p| local(first, *p)).collect();
    let end: Vec<Vec3> = match &to {
        Some(t) => {
            let mut e: Vec<Vec3> = t.points.iter().map(|p| local(last, *p)).collect();
            align(&start, &mut e);
            e
        }
        None => {
            let c = local(first, ca);
            start.iter().map(|p| c + (*p - c) * end_scale).collect()
        }
    };

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((sections + 1) * samples);
    for (k, f) in frames.iter().enumerate() {
        let w = k as f32 / sections as f32;
        let axis = f.n.cross(f.b);
        for i in 0..samples {
            let l = start[i].lerp(end[i], w);
            positions.push((f.pos + f.n * l.x + f.b * l.y + axis * l.z).to_array());
        }
    }
    let flip = area(&start) < 0.0;
    let mut indices: Vec<u32> = Vec::with_capacity(sections * samples * 6);
    for k in 0..sections {
        let (a, b) = ((k * samples) as u32, ((k + 1) * samples) as u32);
        for j in 0..samples {
            let j2 = ((j + 1) % samples) as u32;
            let j = j as u32;
            let (p, q, r, t) = (a + j, a + j2, b + j2, b + j);
            if flip {
                indices.extend_from_slice(&[p, r, q, p, t, r]);
            } else {
                indices.extend_from_slice(&[p, q, r, p, r, t]);
            }
        }
    }
    let mesh = Mesh::new(positions, indices);
    let mut reply = add(s, args, "Curve Bridge", mesh, started);
    reply["sections"] = json!(sections);
    reply["samples"] = json!(samples);
    Ok(reply)
}
