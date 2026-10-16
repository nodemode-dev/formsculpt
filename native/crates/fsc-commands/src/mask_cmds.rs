use std::cmp::Ordering;
use std::collections::BinaryHeap;

use fsc_core::{Aabb, Object};
use glam::{Mat4, Vec3, Vec4};
use serde_json::{json, Value};

use crate::sculpt_cmds::{mask_edit, parse_color, target};
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn number(args: &Value, key: &str, default: f64, lo: f64, hi: f64) -> Result<f32, CommandError> {
    let v = args.get(key).and_then(Value::as_f64).unwrap_or(default);
    if !(lo..=hi).contains(&v) {
        return Err(bad(format!("{key} must be {lo} to {hi}")));
    }
    Ok(v as f32)
}

#[derive(Clone, Copy)]
enum Op {
    Replace,
    Add,
    Subtract,
    Intersect,
}

fn op(args: &Value, default: &str) -> Result<Op, CommandError> {
    Ok(match args.get("op").and_then(Value::as_str).unwrap_or(default) {
        "replace" => Op::Replace,
        "add" => Op::Add,
        "subtract" => Op::Subtract,
        "intersect" => Op::Intersect,
        other => return Err(bad(format!("unknown op {other}; use replace, add, subtract or intersect"))),
    })
}

fn combine(op: Op, old: &[f32], new: &[f32]) -> Vec<f32> {
    old.iter()
        .zip(new)
        .map(|(&a, &b)| match op {
            Op::Replace => b,
            Op::Add => a.max(b),
            Op::Subtract => (a - b).max(0.0),
            Op::Intersect => a.min(b),
        })
        .collect()
}

const OP: &str = "How the new mask meets the current one: replace it, add (keep the larger), subtract, or intersect (keep the smaller).";

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0).max(1e-12)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn percentile(mut v: Vec<f32>, q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let k = ((v.len() - 1) as f32 * q) as usize;
    v.select_nth_unstable_by(k, |a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    v[k]
}

/// Signed mean curvature per vertex, the average of each edge's normal curvature 2 n.(q - p) / |q - p|^2: positive
/// where the vertex sits below its neighbours (a crease or cavity), negative on ridges and peaks. Unlike the
/// neighbour-average estimate it is not biased by valence, so an icosphere's irregular vertices read as flat.
fn curvature(o: &Object) -> Vec<f32> {
    (0..o.mesh.vertex_count() as u32)
        .map(|v| {
            let nb = o.topology.neighbors_of(v);
            if nb.is_empty() {
                return 0.0;
            }
            let p = o.mesh.position(v);
            let n = Vec3::from(o.mesh.normals[v as usize]);
            let sum: f32 = nb
                .iter()
                .map(|&w| {
                    let d = o.mesh.position(w) - p;
                    2.0 * n.dot(d) / d.length_squared().max(1e-18)
                })
                .sum();
            sum / nb.len() as f32
        })
        .collect()
}

pub(crate) fn cavity_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "mode": {"type": "string", "enum": ["cavity", "peaks"], "default": "cavity", "description": "cavity masks creases and hollows; peaks masks ridges and raised edges."},
            "contrast": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.5, "description": "0 masks broad curvature softly, 1 only the sharpest creases or ridges."},
            "op": {"type": "string", "enum": ["replace", "add", "subtract", "intersect"], "default": "replace", "description": OP}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn mask_cavity(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let peaks = match args.get("mode").and_then(Value::as_str).unwrap_or("cavity") {
        "cavity" => false,
        "peaks" => true,
        other => return Err(bad(format!("unknown mode {other}; use cavity or peaks"))),
    };
    let contrast = number(args, "contrast", 0.5, 0.0, 1.0)?;
    let op = op(args, "replace")?;
    mask_edit(s, args, if peaks { "Mask Peaks" } else { "Mask Cavity" }, |o, old| {
        let mut c = curvature(o);
        // Two passes of neighbour averaging: the umbrella estimate reads irregular vertices (valence 5 or 7) as
        // small dents or bumps, which would otherwise show as faint stars in the mask.
        for _ in 0..2 {
            c = (0..c.len())
                .map(|v| {
                    let nb = o.topology.neighbors_of(v as u32);
                    if nb.is_empty() {
                        return c[v];
                    }
                    0.5 * c[v] + 0.5 * nb.iter().map(|&w| c[w as usize]).sum::<f32>() / nb.len() as f32
                })
                .collect();
        }
        // Measured against the form's typical curvature, so a sphere (convex everywhere) has neither cavities nor
        // peaks until something is carved into it or raised on it.
        let median = percentile(c.clone(), 0.5);
        let scale = percentile(c.iter().map(|x| x.abs()).collect(), 0.9);
        c.iter_mut().for_each(|x| *x = if peaks { median - *x } else { *x - median });
        // Thresholds from the object's own spread, so the same settings work on a soft blob and a detailed sculpt.
        let positive: Vec<f32> = c.iter().copied().filter(|&x| x > 0.0).collect();
        // The floor keeps a featureless surface (float noise only) from being masked.
        let hi = percentile(positive, 0.9).max(scale * 0.15).max(1e-6);
        let lo = hi * (0.05 + contrast * 0.6);
        let raw: Vec<f32> = c.iter().map(|&x| smoothstep(lo, hi, x)).collect();
        let blurred: Vec<f32> = (0..raw.len())
            .map(|v| {
                let nb = o.topology.neighbors_of(v as u32);
                if nb.is_empty() {
                    return raw[v];
                }
                0.5 * raw[v] + 0.5 * nb.iter().map(|&w| raw[w as usize]).sum::<f32>() / nb.len() as f32
            })
            .collect();
        combine(op, old, &blurred)
    })
}

pub(crate) fn color_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "color": {"type": ["string", "array"], "description": "The painted colour to mask: #rrggbb or [r, g, b] from 0 to 1."},
            "tolerance": {"type": "number", "minimum": 0.001, "maximum": 1, "default": 0.15, "description": "How far a colour may differ (0 to 1 across the colour cube) and still be masked; the edge is soft over the last half."},
            "op": {"type": "string", "enum": ["replace", "add", "subtract", "intersect"], "default": "replace", "description": OP}
        },
        "required": ["color"],
        "additionalProperties": false
    })
}

pub(crate) fn mask_by_color(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let color = parse_color(args.get("color"))?.ok_or_else(|| bad("color is required"))?;
    let tolerance = number(args, "tolerance", 0.15, 0.001, 1.0)?;
    let op = op(args, "replace")?;
    let id = target(s, args)?;
    if !s.scene.get(id).expect("checked").mesh.has_paint() {
        return Err(CommandError::new("no_paint", "the object has no vertex paint to mask by; paint it first"));
    }
    let c = Vec3::from(color);
    mask_edit(s, args, "Mask by Color", |o, old| {
        let new: Vec<f32> = o
            .mesh
            .paint
            .iter()
            .map(|p| {
                let d = Vec3::new(p[0], p[1], p[2]).distance(c) / 3f32.sqrt();
                1.0 - smoothstep(tolerance * 0.5, tolerance, d)
            })
            .collect();
        combine(op, old, &new)
    })
}

pub(crate) fn grow_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "steps": {"type": "integer", "minimum": -50, "maximum": 50, "default": 1, "description": "Rings of edges to grow the mask by; negative shrinks it."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn mask_grow(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let steps = args.get("steps").and_then(Value::as_i64).unwrap_or(1);
    if !(-50..=50).contains(&steps) || steps == 0 {
        return Err(bad("steps must be -50 to 50 and not 0"));
    }
    let label = if steps > 0 { "Grow Mask" } else { "Shrink Mask" };
    mask_edit(s, args, label, |o, old| {
        let mut m = old.to_vec();
        for _ in 0..steps.unsigned_abs() {
            m = (0..m.len())
                .map(|v| {
                    let nb = o.topology.neighbors_of(v as u32);
                    let pick = nb.iter().map(|&w| m[w as usize]);
                    if steps > 0 {
                        pick.fold(m[v], f32::max)
                    } else {
                        pick.fold(m[v], f32::min)
                    }
                })
                .collect();
        }
        m
    })
}

pub(crate) fn topology_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Where to start, in object space; the nearest vertex is used."},
            "distance": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "description": "How far to reach, measured along the surface's edges (not straight through space), in world units."},
            "falloff": {"type": "boolean", "default": true, "description": "Fade the mask out toward the reach; false masks the whole reach fully."},
            "op": {"type": "string", "enum": ["replace", "add", "subtract", "intersect"], "default": "replace", "description": OP}
        },
        "required": ["point", "distance"],
        "additionalProperties": false
    })
}

#[derive(PartialEq)]
struct Near(f32, u32);
impl Eq for Near {}
impl PartialOrd for Near {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Near {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal)
    }
}

/// Distance along the edges from `start` to every vertex within `reach` (infinity beyond).
fn surface_distance(o: &Object, start: u32, reach: f32) -> Vec<f32> {
    let mut dist = vec![f32::INFINITY; o.mesh.vertex_count()];
    let mut heap = BinaryHeap::new();
    dist[start as usize] = 0.0;
    heap.push(Near(0.0, start));
    while let Some(Near(d, v)) = heap.pop() {
        if d > dist[v as usize] || d > reach {
            continue;
        }
        let p = o.mesh.position(v);
        for &w in o.topology.neighbors_of(v) {
            let nd = d + p.distance(o.mesh.position(w));
            if nd < dist[w as usize] && nd <= reach {
                dist[w as usize] = nd;
                heap.push(Near(nd, w));
            }
        }
    }
    dist
}

pub(crate) fn mask_topology(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let point = args.get("point").and_then(Value::as_array).filter(|a| a.len() == 3).ok_or_else(|| bad("point is [x, y, z]"))?;
    let p: Vec<f32> = point.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
    if p.iter().any(|x| !x.is_finite()) {
        return Err(bad("point values must be numbers"));
    }
    let p = Vec3::new(p[0], p[1], p[2]);
    let reach = number(args, "distance", 1.0, 1e-9, 1e4)?;
    let fade = args.get("falloff").and_then(Value::as_bool).unwrap_or(true);
    let op = op(args, "replace")?;
    mask_edit(s, args, "Mask by Topology", |o, old| {
        let start = (0..o.mesh.vertex_count() as u32).min_by(|&a, &b| o.mesh.position(a).distance_squared(p).partial_cmp(&o.mesh.position(b).distance_squared(p)).unwrap_or(Ordering::Equal)).unwrap_or(0);
        let dist = surface_distance(o, start, reach);
        let new: Vec<f32> = dist.iter().map(|&d| if d > reach { 0.0 } else if fade { 1.0 - smoothstep(0.0, reach, d) } else { 1.0 }).collect();
        combine(op, old, &new)
    })
}

pub(crate) fn shape_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "shape": {"type": "string", "enum": ["box", "ellipse", "lasso"], "default": "box", "description": "box and ellipse take two corner points; lasso takes the outline, three points or more."},
            "points": {"type": "array", "minItems": 2, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}, "description": "Picture coordinates as [x, y], x from 0 (left) to 1 (right), y from 0 (top) to 1 (bottom), in the picture get_view returns for the same view, width and height."},
            "view": {"type": "string", "enum": ["three_quarter", "front", "back", "right", "left", "top", "bottom"], "default": "front", "description": "The get_view camera the points refer to (framing every visible object, as get_view does by default)."},
            "yaw": {"type": "number", "minimum": -360, "maximum": 360, "description": "Degrees; overrides view, as in get_view."},
            "pitch": {"type": "number", "minimum": -90, "maximum": 90, "description": "Degrees; overrides view, as in get_view."},
            "width": {"type": "integer", "minimum": 1, "maximum": 8192, "default": 640, "description": "The picture's width, for its aspect; as in get_view."},
            "height": {"type": "integer", "minimum": 1, "maximum": 8192, "default": 640},
            "view_proj": {"type": "array", "items": {"type": "number"}, "minItems": 16, "maxItems": 16, "description": "Instead of a view: a column-major view-projection matrix (the window's camera); the points are then that view's picture coordinates."},
            "op": {"type": "string", "enum": ["replace", "add", "subtract", "intersect"], "default": "add", "description": OP}
        },
        "required": ["points"],
        "additionalProperties": false
    })
}

pub(crate) fn inside_polygon(x: f32, y: f32, poly: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > y) != (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub(crate) fn picture_points(args: &Value) -> Result<Vec<[f32; 2]>, CommandError> {
    args.get("points")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("points is required"))?
        .iter()
        .map(|p| {
            let a = p.as_array().filter(|a| a.len() == 2).ok_or_else(|| bad("each point is [x, y]"))?;
            let (x, y) = (a[0].as_f64().ok_or_else(|| bad("points are numbers"))?, a[1].as_f64().ok_or_else(|| bad("points are numbers"))?);
            Ok([x as f32, y as f32])
        })
        .collect()
}

pub(crate) fn project_to_picture(s: &Session, args: &Value, positions: &[[f32; 3]]) -> Result<Vec<Option<[f32; 2]>>, CommandError> {
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
    let views = [("three_quarter", 30.0, 20.0), ("front", 0.0, 0.0), ("back", 180.0, 0.0), ("right", 90.0, 0.0), ("left", -90.0, 0.0), ("top", 0.0, 89.9), ("bottom", 0.0, -89.9)];
    let name = args.get("view").and_then(Value::as_str).unwrap_or("front");
    let &(_, yaw0, pitch0) = views.iter().find(|v| v.0 == name).ok_or_else(|| bad(format!("unknown view {name}")))?;
    let yaw = args.get("yaw").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(yaw0);
    let pitch = args.get("pitch").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(pitch0).clamp(-89.9, 89.9);
    let aspect = number(args, "width", 640.0, 1.0, 8192.0)? / number(args, "height", 640.0, 1.0, 8192.0)?;
    let mut frame = Aabb::EMPTY;
    for o in s.scene.objects.iter().filter(|o| o.visible) {
        for &p in &o.mesh.positions {
            frame.grow(Vec3::from(p));
        }
    }
    Ok(match matrix {
        Some(m) => positions
            .iter()
            .map(|&p| {
                let c = m * Vec4::new(p[0], p[1], p[2], 1.0);
                (c.w > 1e-9).then(|| [(c.x / c.w) * 0.5 + 0.5, 0.5 - (c.y / c.w) * 0.5])
            })
            .collect(),
        None => fsc_paint::view_coordinates(frame, yaw.to_radians(), pitch.to_radians(), aspect, positions).into_iter().map(Some).collect(),
    })
}

pub(crate) fn mask_shape(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let shape = args.get("shape").and_then(Value::as_str).unwrap_or("box").to_string();
    let points = picture_points(args)?;
    match shape.as_str() {
        "box" | "ellipse" if points.len() < 2 => return Err(bad("box and ellipse take two corner points")),
        "lasso" if points.len() < 3 => return Err(bad("a lasso needs at least three points")),
        "box" | "ellipse" | "lasso" => {}
        other => return Err(bad(format!("unknown shape {other}; use box, ellipse or lasso"))),
    }
    let op = op(args, "add")?;
    let id = target(s, args)?;
    let screen = project_to_picture(s, args, &s.scene.get(id).expect("checked").mesh.positions)?;
    let (x0, x1) = (points[0][0].min(points[1][0]), points[0][0].max(points[1][0]));
    let (y0, y1) = (points[0][1].min(points[1][1]), points[0][1].max(points[1][1]));
    let new: Vec<f32> = screen
        .iter()
        .map(|q| {
            let Some([x, y]) = *q else { return 0.0 };
            let inside = match shape.as_str() {
                "box" => x >= x0 && x <= x1 && y >= y0 && y <= y1,
                "ellipse" => {
                    let (cx, cy, rx, ry) = ((x0 + x1) * 0.5, (y0 + y1) * 0.5, ((x1 - x0) * 0.5).max(1e-9), ((y1 - y0) * 0.5).max(1e-9));
                    ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
                }
                _ => inside_polygon(x, y, &points),
            };
            if inside {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    let label = format!("Mask {}", match shape.as_str() { "box" => "Box", "ellipse" => "Ellipse", _ => "Lasso" });
    mask_edit(s, args, &label, |_, old| combine(op, old, &new))
}
