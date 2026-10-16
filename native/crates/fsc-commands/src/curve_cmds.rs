use std::f32::consts::TAU;

use fsc_core::{Curve, CurvePoint, Interp};
use glam::{Vec2, Vec3};
use serde_json::{json, Value};

use crate::object_cmds::set_positions;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn number(args: &Value, key: &str, default: f64, lo: f64, hi: f64) -> Result<f64, CommandError> {
    let v = match args.get(key) {
        None | Some(Value::Null) => default,
        Some(v) => v.as_f64().filter(|f| f.is_finite()).ok_or_else(|| bad(format!("{key} must be a number")))?,
    };
    if !(lo..=hi).contains(&v) {
        return Err(bad(format!("{key} must be {lo} to {hi}")));
    }
    Ok(v)
}

pub(crate) fn point_schema(dim: usize) -> Value {
    let array = if dim == 3 {
        json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 4, "description": "[x, y, z] or [x, y, z, radius]."})
    } else {
        json!({"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "[a, b]."})
    };
    let mut props = json!({
        "p": {"type": "array", "items": {"type": "number"}, "minItems": dim, "maxItems": dim, "description": "The position."},
        "sharp": {"type": "boolean", "description": "A corner: the curve breaks its tangent here instead of passing smoothly through."},
        "round": {"type": "number", "minimum": 0, "description": "On a sharp point: replace the corner with a circular fillet of this world radius (kept under half of either neighbouring segment)."}
    });
    if dim == 3 {
        props["radius"] = json!({"type": "number", "exclusiveMinimum": 0, "description": "Radius at this point, in world units."});
        props["twist"] = json!({"type": "number", "description": "Roll of the cross-section about the path at this point, in degrees."});
    }
    json!({"anyOf": [array, {"type": "object", "properties": props, "required": ["p"], "additionalProperties": false}]})
}

fn finite(v: &Value, what: &str) -> Result<f32, CommandError> {
    v.as_f64().filter(|f| f.is_finite()).map(|f| f as f32).ok_or_else(|| bad(format!("{what} values must be finite numbers")))
}

fn position(v: Option<&Value>, dim: usize, what: &str) -> Result<Vec3, CommandError> {
    let a = v.and_then(Value::as_array).ok_or_else(|| bad(format!("each {what} needs p: an array of {dim} numbers")))?;
    if a.len() != dim {
        return Err(bad(format!("each {what} position needs {dim} numbers")));
    }
    let f: Vec<f32> = a.iter().map(|x| finite(x, what)).collect::<Result<_, _>>()?;
    Ok(Vec3::new(f[0], f[1], f.get(2).copied().unwrap_or(0.0)))
}

pub(crate) fn parse_points(list: &[Value], dim: usize, default_radius: f32, default_sharp: bool, what: &str) -> Result<Vec<CurvePoint>, CommandError> {
    let mut out = Vec::with_capacity(list.len());
    for v in list {
        let mut c = CurvePoint::new(Vec3::ZERO);
        c.radius = default_radius;
        c.sharp = default_sharp;
        match v {
            Value::Array(a) => {
                if a.len() != dim && !(dim == 3 && a.len() == 4) {
                    return Err(bad(format!("each {what} needs {} numbers", if dim == 3 { "3 or 4" } else { "2" })));
                }
                c.pos = position(Some(&Value::Array(a[..dim].to_vec())), dim, what)?;
                if a.len() == 4 {
                    c.radius = finite(&a[3], what)?;
                }
            }
            Value::Object(o) => {
                for k in o.keys() {
                    let ok = matches!(k.as_str(), "p" | "sharp" | "round") || (dim == 3 && matches!(k.as_str(), "radius" | "twist"));
                    if !ok {
                        return Err(bad(format!("a {what} has no field {k}")));
                    }
                }
                c.pos = position(o.get("p"), dim, what)?;
                if let Some(x) = o.get("radius").filter(|x| !x.is_null()) {
                    c.radius = finite(x, "radius")?;
                }
                if let Some(x) = o.get("twist").filter(|x| !x.is_null()) {
                    c.twist = finite(x, "twist")?.to_radians();
                }
                if let Some(x) = o.get("sharp").filter(|x| !x.is_null()) {
                    c.sharp = x.as_bool().ok_or_else(|| bad("sharp must be true or false"))?;
                }
                if let Some(x) = o.get("round").filter(|x| !x.is_null()) {
                    c.round = finite(x, "round")?;
                }
            }
            _ => return Err(bad(format!("each {what} is an array or an object with p"))),
        }
        if c.radius <= 0.0 {
            return Err(bad("point radii must be above 0"));
        }
        if c.round < 0.0 {
            return Err(bad("round must be 0 or more"));
        }
        out.push(c);
    }
    Ok(out)
}

pub(crate) fn interp_of(args: &Value) -> Result<Interp, CommandError> {
    match args.get("interp") {
        None | Some(Value::Null) => Ok(Interp::CatmullRom),
        Some(v) => v.as_str().and_then(Interp::parse).ok_or_else(|| bad("interp must be catmull_rom, bspline or linear")),
    }
}

pub(crate) fn curve_of(args: &Value, key: &str, default_radius: f32) -> Result<Curve, CommandError> {
    let list = args.get(key).and_then(Value::as_array).ok_or_else(|| bad(format!("{key} is required")))?;
    let points = parse_points(list, 3, default_radius, false, "point")?;
    Ok(Curve::new(points, interp_of(args)?, args.get("closed").and_then(Value::as_bool).unwrap_or(false)))
}

pub(crate) fn path_positions(args: &Value, key: &str, missing: &str) -> Result<Vec<Vec3>, CommandError> {
    if !args.get(key).is_some_and(Value::is_array) {
        return Err(bad(missing));
    }
    let curve = curve_of(args, key, 1.0)?;
    let mut out: Vec<Vec3> = curve.sample(16).into_iter().map(|s| s.pos).collect();
    if curve.is_closed() && !out.is_empty() {
        out.push(out[0]);
    }
    Ok(out)
}

pub(crate) fn guide_positions(guide: &Value) -> Result<Vec<Vec3>, CommandError> {
    let list = guide.as_array().ok_or_else(|| bad("each guide is a list of points"))?;
    let points = parse_points(list, 3, 1.0, false, "guide point")?;
    Ok(Curve::new(points, Interp::CatmullRom, false).sample(8).into_iter().map(|s| s.pos).collect())
}

pub(crate) fn lathe_profile(args: &Value) -> Result<Vec<[f32; 2]>, CommandError> {
    let list = args.get("profile").and_then(Value::as_array).ok_or_else(|| bad("profile is required"))?;
    let points = parse_points(list, 2, 1.0, true, "profile point")?;
    if points.iter().any(|p| p.pos.x < 0.0) {
        return Err(bad("profile radii must be 0 or more"));
    }
    let per_span = number(args, "smooth", 6.0, 1.0, 32.0)? as usize;
    let samples = Curve::new(points, interp_of(args)?, false).sample(per_span);
    Ok(samples.iter().map(|s| [s.pos.x.max(0.0), s.pos.y]).collect())
}

pub(crate) fn profile_ring(value: Option<&Value>, sides: usize) -> Result<Vec<Vec2>, CommandError> {
    let corners = |pts: &[[f32; 2]]| pts.iter().map(|p| Vec2::new(p[0], p[1])).collect::<Vec<_>>();
    match value {
        None | Some(Value::Null) => Ok(circle(sides)),
        Some(Value::String(name)) => match name.as_str() {
            "circle" => Ok(circle(sides)),
            "square" => Ok(corners(&[[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]])),
            "flat" => Ok(corners(&[[-1.0, -0.2], [1.0, -0.2], [1.0, 0.2], [-1.0, 0.2]])),
            "triangle" => Ok((0..3).map(|k| {
                let a = TAU * (0.25 + k as f32 / 3.0);
                Vec2::new(a.cos(), a.sin())
            }).collect()),
            "star" => Ok((0..10).map(|k| {
                let a = TAU * (0.25 + k as f32 / 10.0);
                let r = if k % 2 == 0 { 1.0 } else { 0.45 };
                Vec2::new(a.cos() * r, a.sin() * r)
            }).collect()),
            other => Err(bad(format!("unknown profile {other}; use circle, square, triangle, star, flat or a list of points"))),
        },
        Some(Value::Array(list)) => {
            let points = parse_points(list, 2, 1.0, true, "profile point")?;
            if points.len() < 3 {
                return Err(bad("a tube profile needs at least three points"));
            }
            let ring = Curve::new(points, Interp::CatmullRom, true).sample(6);
            Ok(ring.iter().map(|s| Vec2::new(s.pos.x, s.pos.y)).collect())
        }
        Some(_) => Err(bad("profile is a preset name or a list of [x, y] points")),
    }
}

fn circle(sides: usize) -> Vec<Vec2> {
    (0..sides.max(3))
        .map(|j| {
            let a = TAU * j as f32 / sides.max(3) as f32;
            Vec2::new(a.cos(), a.sin())
        })
        .collect()
}

pub(crate) fn set_point_fields(point: &mut Value, patch: &Value, dim: usize) -> Result<(), CommandError> {
    if let Value::Array(a) = point {
        let mut o = json!({"p": a[..dim.min(a.len())].to_vec()});
        if a.len() > dim {
            o["radius"] = a[dim].clone();
        }
        *point = o;
    }
    let Some(target) = point.as_object_mut() else { return Err(bad("that point cannot be edited")) };
    let Some(fields) = patch.as_object() else { return Err(bad("each set_points entry is an object")) };
    for (k, v) in fields {
        match k.as_str() {
            "index" => {}
            "sharp" | "round" | "radius" | "twist" => {
                if v.is_null() {
                    target.remove(k);
                } else {
                    target.insert(k.clone(), v.clone());
                }
            }
            other => return Err(bad(format!("set_points has no field {other}; use index, sharp, round, radius or twist"))),
        }
    }
    Ok(())
}

pub(crate) fn moved_point(point: &mut Value, to: &Value) {
    let (Some(o), Some(a)) = (point.as_object_mut(), to.as_array()) else {
        *point = to.clone();
        return;
    };
    let n = o.get("p").and_then(Value::as_array).map(Vec::len).unwrap_or(a.len());
    if a.len() < n {
        *point = to.clone();
        return;
    }
    o.insert("p".into(), Value::Array(a[..n].to_vec()));
    if a.len() > n {
        o.insert("radius".into(), a[n].clone());
    }
}

pub(crate) fn sample_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "points": {"type": "array", "minItems": 2, "maxItems": 4096, "items": point_schema(3), "description": "The curve's control points: [x, y, z], [x, y, z, radius], or objects {p, radius, twist, sharp, round}. A sharp point is a corner; sharp with round above 0 becomes a circular fillet of that world radius."},
            "interp": {"type": "string", "enum": ["catmull_rom", "bspline", "linear"], "default": "catmull_rom", "description": "catmull_rom passes through every smooth point (centripetal, no cusps); bspline approximates with a smoother curve; linear is a polyline. Sharp points split the curve into sections that are interpolated on their own."},
            "closed": {"type": "boolean", "default": false, "description": "Loop the last point back to the first. The result does not repeat the first sample at the end."},
            "per_span": {"type": "integer", "minimum": 1, "maximum": 64, "default": 8, "description": "Samples per span between control points (also sets how finely fillet arcs are cut)."},
            "spacing": {"type": "number", "exclusiveMinimum": 0, "description": "When given, resample by arc length instead: samples this many world units apart (corners always kept)."},
            "radius": {"type": "number", "exclusiveMinimum": 0, "default": 1, "description": "Radius for points that give none."}
        },
        "required": ["points"],
        "additionalProperties": false
    })
}

pub(crate) fn sample(_: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let radius = number(args, "radius", 1.0, 1e-6, 1e6)? as f32;
    let curve = curve_of(args, "points", radius)?;
    let per_span = number(args, "per_span", 8.0, 1.0, 64.0)? as usize;
    let spacing = number(args, "spacing", 0.0, 0.0, 1e9)? as f32;
    let samples = if spacing > 0.0 { curve.sample_spacing(spacing) } else { curve.sample(per_span) };
    if samples.len() < 2 {
        return Err(bad("the curve needs at least two distinct points"));
    }
    if samples.len() > 400_000 {
        return Err(bad("that would give more than 400000 samples; raise spacing or lower per_span"));
    }
    let v3 = |v: Vec3| json!([v.x, v.y, v.z]);
    Ok(json!({
        "count": samples.len(),
        "closed": curve.is_closed(),
        "length": curve.length(),
        "positions": samples.iter().map(|s| v3(s.pos)).collect::<Vec<_>>(),
        "tangents": samples.iter().map(|s| v3(s.tangent)).collect::<Vec<_>>(),
        "radii": samples.iter().map(|s| s.radius).collect::<Vec<_>>(),
        "twists": samples.iter().map(|s| s.twist.to_degrees()).collect::<Vec<_>>(),
        "t": samples.iter().map(|s| s.t).collect::<Vec<_>>(),
        "corners": samples.iter().enumerate().filter(|(_, s)| s.corner).map(|(i, _)| i).collect::<Vec<_>>()
    }))
}

pub(crate) fn warp_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "points": {"type": "array", "minItems": 2, "maxItems": 4096, "items": point_schema(3), "description": "The curve to follow, in the same point formats as sample_curve (sharp and round respected)."},
            "interp": {"type": "string", "enum": ["catmull_rom", "bspline", "linear"], "default": "catmull_rom"},
            "closed": {"type": "boolean", "default": false},
            "axis": {"type": "string", "enum": ["x", "y", "z"], "description": "The object's axis that runs along the curve; its longest side when omitted."},
            "stretch": {"type": "boolean", "default": true, "description": "Fit the object's length to the curve's full length. Off keeps the object's length (in world units) from the curve's start, running on straight past the end."},
            "twist": {"type": "number", "minimum": -36000, "maximum": 36000, "default": 0, "description": "Extra roll about the curve in degrees, spread over its length."}
        },
        "required": ["points"],
        "additionalProperties": false
    })
}

pub(crate) fn warp_curve(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let mut curve = curve_of(args, "points", 1.0)?;
    if curve.is_closed() {
        let first = curve.points[0];
        curve.points.push(first);
        curve.closed = false;
    }
    let samples = curve.sample(32);
    if samples.len() < 2 {
        return Err(bad("the curve needs at least two distinct points"));
    }
    let mut cum = vec![0.0f32];
    for w in samples.windows(2) {
        cum.push(cum[cum.len() - 1] + w[0].pos.distance(w[1].pos));
    }
    let total = cum[cum.len() - 1];
    if total <= 1e-9 {
        return Err(bad("the curve has no length"));
    }
    let object = s.scene.get(id).expect("checked");
    let bounds = object.mesh.bounds();
    let extent = bounds.extent();
    let axis = match args.get("axis").and_then(Value::as_str) {
        None => [0, 2].into_iter().fold(1, |best, a| if extent[a] > extent[best] + 1e-6 { a } else { best }),
        Some("x") => 0,
        Some("y") => 1,
        Some("z") => 2,
        Some(other) => return Err(bad(format!("axis must be x, y or z, not {other}"))),
    };
    let stretch = args.get("stretch").and_then(Value::as_bool).unwrap_or(true);
    let roll = (number(args, "twist", 0.0, -36000.0, 36000.0)? as f32).to_radians();
    let (ua, ub) = ((axis + 1) % 3, (axis + 2) % 3);
    let lo = bounds.min[axis];
    let len = extent[axis].max(1e-9);
    let center = bounds.center();

    let mut seed = Vec3::ZERO;
    seed[ua] = 1.0;
    let mut normals: Vec<Vec3> = Vec::with_capacity(samples.len());
    let mut nrm = (seed - samples[0].tangent * seed.dot(samples[0].tangent)).try_normalize().unwrap_or_else(|| samples[0].tangent.any_orthonormal_vector());
    for sm in &samples {
        nrm = (nrm - sm.tangent * nrm.dot(sm.tangent)).try_normalize().unwrap_or_else(|| sm.tangent.any_orthonormal_vector());
        normals.push(nrm);
    }
    let last = samples.len() - 1;
    let frame = |d: f32| -> (Vec3, Vec3, Vec3, Vec3) {
        if d >= total {
            let t = samples[last].tangent;
            return (samples[last].pos + t * (d - total), t, normals[last], t.cross(normals[last]));
        }
        let i = cum.partition_point(|&c| c <= d).clamp(1, last) - 1;
        let u = ((d - cum[i]) / (cum[i + 1] - cum[i]).max(1e-12)).clamp(0.0, 1.0);
        let t = samples[i].tangent.lerp(samples[i + 1].tangent, u).try_normalize().unwrap_or(samples[i].tangent);
        let n = normals[i].lerp(normals[i + 1], u);
        let n = (n - t * n.dot(t)).try_normalize().unwrap_or_else(|| t.any_orthonormal_vector());
        (samples[i].pos.lerp(samples[i + 1].pos, u), t, n, t.cross(n))
    };
    let positions: Vec<[f32; 3]> = object
        .mesh
        .positions
        .iter()
        .enumerate()
        .map(|(v, p)| {
            let p = Vec3::from_array(*p);
            let along = (p[axis] - lo).max(0.0);
            let d = if stretch { along / len * total } else { along };
            let (at, _, n, b) = frame(d);
            let (sin, cos) = (roll * d / total).sin_cos();
            let (n, b) = (n * cos + b * sin, b * cos - n * sin);
            let moved = at + n * (p[ua] - center[ua]) + b * (p[ub] - center[ub]);
            let w = 1.0 - object.mask.get(v).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            p.lerp(moved, w).to_array()
        })
        .collect();
    let mut reply = set_positions(s, id, "Warp Along Curve", positions);
    reply["curve_length"] = json!(total);
    reply["axis"] = json!(["x", "y", "z"][axis]);
    Ok(reply)
}
