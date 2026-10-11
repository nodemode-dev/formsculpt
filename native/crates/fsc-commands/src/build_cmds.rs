use std::time::Instant;

use fsc_core::Mesh;
use fsc_topo::Ball;
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn number(args: &Value, key: &str, default: f64, lo: f64, hi: f64) -> Result<f64, CommandError> {
    let v = args.get(key).and_then(Value::as_f64).unwrap_or(default);
    if !(lo..=hi).contains(&v) {
        return Err(bad(format!("{key} must be {lo} to {hi}")));
    }
    Ok(v)
}

fn floats(v: &Value, what: &str, lens: &[usize]) -> Result<Vec<f32>, CommandError> {
    let a = v.as_array().ok_or_else(|| bad(format!("each {what} must be an array of numbers")))?;
    if !lens.contains(&a.len()) {
        return Err(bad(format!("each {what} needs {} numbers", lens.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(" or "))));
    }
    a.iter()
        .map(|x| x.as_f64().filter(|f| f.is_finite()).map(|f| f as f32).ok_or_else(|| bad(format!("{what} values must be finite numbers"))))
        .collect()
}

fn list<'a>(args: &'a Value, key: &str) -> Result<&'a Vec<Value>, CommandError> {
    args.get(key).and_then(Value::as_array).ok_or_else(|| bad(format!("{key} is required")))
}

fn add(s: &mut Session, args: &Value, base: &str, mesh: Mesh, started: Instant) -> Value {
    let base = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).unwrap_or(base);
    let name = s.scene.unique_name(base);
    let id = s.scene.add(name, mesh);
    let index = s.scene.index_of(id).expect("just added");
    s.history.push(Edit::Add { id, index, parked: None });
    json!({"object": summary(s.scene.get(id).expect("just added")), "active": id, "ms": started.elapsed().as_millis()})
}

const NAME: &str = "Name for the new object; a default when omitted.";

pub(crate) fn skeleton_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "spheres": {"type": "array", "minItems": 1, "maxItems": 512, "items": {"type": "array", "items": {"type": "number"}, "minItems": 4, "maxItems": 4}, "description": "Joints as [x, y, z, radius]."},
            "links": {"type": "array", "maxItems": 1024, "items": {"type": "array", "items": {"type": "integer", "minimum": 0}, "minItems": 2, "maxItems": 2}, "description": "Pairs of sphere indices joined by a tapered limb."},
            "blend": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.3, "description": "How softly limbs merge where they meet, as a fraction of the mean radius."},
            "resolution": {"type": "integer", "minimum": 32, "maximum": 256, "default": 96, "description": "Voxel cells along the longest side."},
            "name": {"type": "string", "description": NAME}
        },
        "required": ["spheres"],
        "additionalProperties": false
    })
}

pub(crate) fn skeleton(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let mut balls = Vec::new();
    for v in list(args, "spheres")? {
        let f = floats(v, "sphere", &[4])?;
        if f[3] <= 0.0 {
            return Err(bad("sphere radii must be above 0"));
        }
        balls.push(Ball { center: Vec3::new(f[0], f[1], f[2]), radius: f[3] });
    }
    let mut links = Vec::new();
    if let Some(ls) = args.get("links").and_then(Value::as_array) {
        for l in ls {
            let pair = l.as_array().filter(|p| p.len() == 2).ok_or_else(|| bad("each link is [i, j]"))?;
            let (a, b) = (pair[0].as_u64().unwrap_or(u64::MAX) as usize, pair[1].as_u64().unwrap_or(u64::MAX) as usize);
            if a >= balls.len() || b >= balls.len() || a == b {
                return Err(bad(format!("link [{a}, {b}] must join two different spheres of the {}", balls.len())));
            }
            links.push((a, b));
        }
    }
    let blend = number(args, "blend", 0.3, 0.0, 1.0)? as f32;
    let resolution = number(args, "resolution", 96.0, 32.0, 256.0)? as u32;
    let mesh = fsc_topo::skeleton_mesh(&balls, &links, blend, resolution).map_err(|e| CommandError::new("build_failed", e))?;
    Ok(add(s, args, "Skeleton", mesh, started))
}

pub(crate) fn tube_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "points": {"type": "array", "minItems": 2, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 4}, "description": "The curve as [x, y, z] or [x, y, z, radius] points; a point's radius overrides radius."},
            "radius": {"type": "number", "exclusiveMinimum": 0, "default": 0.1, "description": "Tube radius where a point gives none."},
            "segments": {"type": "integer", "minimum": 3, "maximum": 128, "default": 16, "description": "Sides around the tube."},
            "smooth": {"type": "integer", "minimum": 1, "maximum": 32, "default": 4, "description": "Rings per curve span (a smooth spline through the points)."},
            "closed": {"type": "boolean", "default": false, "description": "Join the last point back to the first (a ring) instead of capping the ends."},
            "name": {"type": "string", "description": NAME}
        },
        "required": ["points"],
        "additionalProperties": false
    })
}

pub(crate) fn tube(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let radius = number(args, "radius", 0.1, 1e-6, 1e6)? as f32;
    let mut points = Vec::new();
    for v in list(args, "points")? {
        let f = floats(v, "point", &[3, 4])?;
        let r = f.get(3).copied().unwrap_or(radius);
        if r <= 0.0 {
            return Err(bad("point radii must be above 0"));
        }
        points.push((Vec3::new(f[0], f[1], f[2]), r));
    }
    let segments = number(args, "segments", 16.0, 3.0, 128.0)? as usize;
    let smooth = number(args, "smooth", 4.0, 1.0, 32.0)? as usize;
    let closed = args.get("closed").and_then(Value::as_bool).unwrap_or(false);
    let mesh = fsc_topo::tube(&points, segments, smooth, closed).map_err(|e| CommandError::new("build_failed", e))?;
    Ok(add(s, args, "Tube", mesh, started))
}

pub(crate) fn lathe_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "profile": {"type": "array", "minItems": 2, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}, "description": "The outline as [radius, y] points from bottom to top, spun around the y axis. A radius of 0 closes to a point; open ends are capped flat."},
            "segments": {"type": "integer", "minimum": 3, "maximum": 256, "default": 48, "description": "Steps around the axis."},
            "name": {"type": "string", "description": NAME}
        },
        "required": ["profile"],
        "additionalProperties": false
    })
}

pub(crate) fn lathe(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let mut profile = Vec::new();
    for v in list(args, "profile")? {
        let f = floats(v, "profile point", &[2])?;
        if f[0] < 0.0 {
            return Err(bad("profile radii must be 0 or more"));
        }
        profile.push([f[0], f[1]]);
    }
    let segments = number(args, "segments", 48.0, 3.0, 256.0)? as usize;
    let mesh = fsc_topo::lathe(&profile, segments).map_err(|e| CommandError::new("build_failed", e))?;
    Ok(add(s, args, "Lathe", mesh, started))
}
