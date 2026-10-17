use fsc_core::Mesh;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::multires_cmds::guard;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

const MAX_TRIANGLES: usize = 8_000_000;

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn apply(s: &mut Session, id: u32, label: &str, mut mesh: Mesh, mask: Vec<f32>) -> Result<Value, CommandError> {
    if mesh.triangle_count() == 0 {
        return Err(CommandError::new("empty_result", format!("{label} produced no triangles; nothing was changed")));
    }
    let object = s.scene.get_mut(id).expect("checked");
    if object.mesh.has_paint() && !mesh.has_paint() {
        fsc_topo::transfer_paint(&[(&object.mesh, &object.bvh)], &mut mesh);
    }
    let before_tris = object.mesh.triangle_count();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    let after = summary(object);
    s.history.push(Edit::Topology { label: label.into(), object: id, mesh: old_mesh, mask: old_mask });
    Ok(json!({"object": after, "triangles_before": before_tris}))
}

pub(crate) fn subdivide_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "levels": {"type": "integer", "minimum": 1, "maximum": 4, "default": 1, "description": "Each level multiplies the triangles by four."},
            "smooth": {"type": "boolean", "default": true, "description": "Loop subdivision (smooths the shape); false splits flat."},
            "sharp_border": {"type": "boolean", "default": false, "description": "Keep open borders where they are while the rest smooths (a plane keeps its corners, a cut shell its rim)."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn subdivide(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Subdivide")?;
    let levels = args.get("levels").and_then(Value::as_u64).unwrap_or(1);
    if !(1..=4).contains(&levels) {
        return Err(bad("levels must be 1 to 4"));
    }
    let smooth = args.get("smooth").and_then(Value::as_bool).unwrap_or(true);
    let sharp_border = args.get("sharp_border").and_then(Value::as_bool).unwrap_or(false);
    let object = s.scene.get(id).expect("checked");
    let projected = object.mesh.triangle_count() * 4usize.pow(levels as u32);
    if projected > MAX_TRIANGLES {
        return Err(bad(format!("that would make {projected} triangles; the limit is {MAX_TRIANGLES}. Use fewer levels or decimate first")));
    }
    let mut mesh = object.mesh.clone();
    let mut mask = object.mask.clone();
    for _ in 0..levels {
        let out = fsc_topo::loop_subdivide_with(&mesh, smooth, sharp_border);
        mask = out.parents.iter().map(|&(a, b)| 0.5 * (mask[a as usize] + mask[b as usize])).collect();
        mesh = out.mesh;
    }
    apply(s, id, "Subdivide", mesh, mask)
}

pub(crate) fn remesh_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "resolution": {"type": "integer", "minimum": 8, "maximum": 320, "default": 128, "description": "Voxels along the object's longest side; higher keeps more detail and makes more triangles."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn remesh_voxel(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Voxel remesh")?;
    let resolution = args.get("resolution").and_then(Value::as_u64).unwrap_or(128);
    if !(8..=320).contains(&resolution) {
        return Err(bad("resolution must be 8 to 320"));
    }
    let object = s.scene.get(id).expect("checked");
    let mesh = fsc_topo::voxel_remesh(&object.mesh, &object.bvh, resolution as u32).map_err(|e| CommandError::new("remesh_failed", e))?;
    let mask = vec![0.0; mesh.vertex_count()];
    apply(s, id, "Voxel Remesh", mesh, mask)
}

pub(crate) fn decimate_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "ratio": {"type": "number", "exclusiveMinimum": 0, "maximum": 1, "default": 0.5, "description": "Fraction of triangles to keep."},
            "triangles": {"type": "integer", "minimum": 4, "description": "Target triangle count; overrides ratio."},
            "preserve_borders": {"type": "boolean", "default": true},
            "preserve_paint": {"type": "boolean", "default": false, "description": "Carry each kept vertex's paint exactly and collapse edges across a colour change last, so painted boundaries stay crisp (without it the paint is transferred from the old surface afterwards)."},
            "preserve_groups": {"type": "boolean", "default": false, "description": "Keep face groups on the faces that remain and hold the borders between groups in place, as open borders are held."},
            "preserve_uvs": {"type": "boolean", "default": false, "description": "Keep UVs (and so a painted texture): vertices on a UV seam do not move, the rest carry their UV to where they collapse. Without it UVs and textures are dropped."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn decimate(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Decimate")?;
    let object = s.scene.get(id).expect("checked");
    let current = object.mesh.triangle_count();
    let target_tris = match args.get("triangles").and_then(Value::as_u64) {
        Some(n) => n as usize,
        None => {
            let ratio = args.get("ratio").and_then(Value::as_f64).unwrap_or(0.5);
            if !(ratio > 0.0 && ratio <= 1.0) {
                return Err(bad("ratio must be above 0 and at most 1"));
            }
            ((current as f64) * ratio) as usize
        }
    };
    if target_tris < 4 {
        return Err(bad("the target must be at least 4 triangles"));
    }
    let flag = |k: &str, d: bool| args.get(k).and_then(Value::as_bool).unwrap_or(d);
    let opts = fsc_topo::DecimateOptions {
        preserve_borders: flag("preserve_borders", true),
        preserve_paint: flag("preserve_paint", false),
        preserve_groups: flag("preserve_groups", false),
        preserve_uvs: flag("preserve_uvs", false),
    };
    let mesh = fsc_topo::decimate_with(&object.mesh, target_tris, opts);
    let mask = vec![0.0; mesh.vertex_count()];
    let reached = mesh.triangle_count();
    let mut out = apply(s, id, "Decimate", mesh, mask)?;
    out["triangles"] = json!(reached);
    out["target"] = json!(target_tris);
    Ok(out)
}

pub(crate) fn fill_holes(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Fill holes")?;
    let object = s.scene.get(id).expect("checked");
    let (mesh, filled) = fsc_topo::fill_holes(&object.mesh);
    if filled == 0 {
        return Ok(json!({"id": id, "holes_filled": 0}));
    }
    let mut mask = object.mask.clone();
    mask.resize(mesh.vertex_count(), 0.0);
    let mut out = apply(s, id, "Fill Holes", mesh, mask)?;
    out["holes_filled"] = json!(filled);
    Ok(out)
}

pub(crate) fn mesh_check(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get(id).expect("checked");
    let edges = fsc_topo::edge_faces(&object.mesh);
    let boundary = edges.values().filter(|f| f.len() == 1).count();
    let non_manifold = edges.values().filter(|f| f.len() > 2).count();
    Ok(json!({
        "id": id,
        "vertices": object.mesh.vertex_count(),
        "triangles": object.mesh.triangle_count(),
        "edges": edges.len(),
        "boundary_edges": boundary,
        "non_manifold_edges": non_manifold,
        "watertight": boundary == 0 && non_manifold == 0,
    }))
}

pub(crate) fn bevel_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "radius": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000, "description": "The round's radius in world units; a twentieth of the object's largest side when omitted."},
            "edges": {"type": "string", "enum": ["convex", "concave", "both"], "default": "convex", "description": "convex rounds outer edges (a box's corners), concave fills inner ones (where a handle meets a cup), both does each."},
            "resolution": {"type": "integer", "minimum": 32, "maximum": 320, "default": 192, "description": "Voxels along the longest side; the object is remeshed at this resolution."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Rounds every edge sharper than the radius, convex, concave or both, as one undo step.
pub(crate) fn bevel_edges(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Bevel")?;
    let object = s.scene.get(id).expect("checked");
    let b = object.mesh.bounds();
    let radius = match args.get("radius") {
        None | Some(Value::Null) => (b.max - b.min).max_element() * 0.05,
        Some(v) => v.as_f64().filter(|r| *r > 0.0 && *r <= 1000.0).ok_or_else(|| bad("radius must be above 0 and at most 1000"))? as f32,
    };
    let which = match args.get("edges").and_then(Value::as_str).unwrap_or("convex") {
        "convex" => fsc_topo::RoundEdges::Convex,
        "concave" => fsc_topo::RoundEdges::Concave,
        "both" => fsc_topo::RoundEdges::Both,
        other => return Err(bad(format!("edges must be convex, concave or both, not {other}"))),
    };
    let resolution = args.get("resolution").and_then(Value::as_u64).unwrap_or(192);
    if !(32..=320).contains(&resolution) {
        return Err(bad("resolution must be 32 to 320"));
    }
    let mesh = fsc_topo::round_edges(&object.mesh, &object.bvh, radius, which, resolution as u32).map_err(|e| CommandError::new("bevel_failed", e))?;
    let mask = vec![0.0; mesh.vertex_count()];
    let mut reply = apply(s, id, "Bevel", mesh, mask)?;
    reply["radius"] = json!(radius);
    Ok(reply)
}

pub(crate) fn cut_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "outline": {"type": "array", "minItems": 2, "maxItems": 2048, "items": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}, "description": "The shape's points as [x, y, z] world points, drawn as seen along direction (only their positions across the direction count). Three or more for lasso, polygon and curve, two or more for path, exactly two for line."},
            "shape": {"type": "string", "enum": ["lasso", "polygon", "curve", "path", "line"], "default": "lasso", "description": "lasso and polygon: the points are the corners of a closed outline. curve: control points of a closed smooth outline (Catmull-Rom, 8 samples per span). path: an open polyline thickened to stroke_width (keep bends gentle; a very sharp turn can overlap itself). line: the half-plane on the left of the direction from point 0 to point 1, in the plane across the view where the first axis runs to the screen's right and the second down for a front view (y down); it is clipped to the object's extent."},
            "stroke_width": {"type": "number", "exclusiveMinimum": 0, "maximum": 100000, "description": "path only: the stroke's full width in world units; 5 percent of the object's largest side when omitted."},
            "flip": {"type": "boolean", "default": false, "description": "Invert the shape: swaps what keep outside and keep inside take."},
            "direction": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The way the outline is seen, from the eye into the scene; the cut goes all the way through the object along it."},
            "keep": {"type": "string", "enum": ["outside", "inside"], "default": "outside", "description": "outside removes what the outline covers (a lasso cut, a hole, a notch); inside keeps only what it covers (trimming a block to a profile)."},
            "round": {"type": "number", "minimum": 0, "maximum": 100, "default": 0, "description": "Rounds the cut edges by this many world units; 0 leaves them sharp."},
            "resolution": {"type": "integer", "minimum": 32, "maximum": 320, "default": 192, "description": "Voxels along the longest side; the object is remeshed at this resolution."}
        },
        "required": ["outline", "direction"],
        "additionalProperties": false
    })
}

/// Hard-surface cut: the outline extruded through the object along the view, taken away (or kept) by a voxel boolean,
/// as one undo step. The result is watertight; flat cut faces are as sharp as the resolution allows.
pub(crate) fn cut_shape(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    use glam::{Vec2, Vec3};
    let id = target(s, args)?;
    guard(s, id, "Cut")?;
    let vec3 = |v: &Value| -> Option<Vec3> {
        let a = v.as_array().filter(|a| a.len() == 3)?;
        let f: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
        f.iter().all(|x| x.is_finite()).then(|| Vec3::new(f[0], f[1], f[2]))
    };
    let d = args.get("direction").and_then(vec3).filter(|d| d.length_squared() > 1e-12).ok_or_else(|| bad("direction is [x, y, z], not zero"))?.normalize();
    let pts: Vec<Vec3> = args.get("outline").and_then(Value::as_array).ok_or_else(|| bad("outline is a list of [x, y, z] points"))?.iter().map(|p| vec3(p).ok_or_else(|| bad("each outline point is [x, y, z]"))).collect::<Result<_, _>>()?;
    let shape = args.get("shape").and_then(Value::as_str).unwrap_or("lasso");
    match shape {
        "lasso" | "polygon" | "curve" if pts.len() < 3 => return Err(bad("outline needs at least three points")),
        "path" if pts.len() < 2 => return Err(bad("a path needs at least two points")),
        "line" if pts.len() != 2 => return Err(bad("a line takes exactly two points")),
        "lasso" | "polygon" | "curve" | "path" | "line" => {}
        other => return Err(bad(format!("unknown shape {other}; use lasso, polygon, curve, path or line"))),
    }
    let flip = match args.get("flip") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(bad("flip is true or false")),
    };
    let inside = match args.get("keep").and_then(Value::as_str).unwrap_or("outside") {
        "outside" => false,
        "inside" => true,
        other => return Err(bad(format!("keep must be outside or inside, not {other}"))),
    } != flip;
    let round = args.get("round").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 100.0) as f32;
    let resolution = args.get("resolution").and_then(Value::as_u64).unwrap_or(192);
    if !(32..=320).contains(&resolution) {
        return Err(bad("resolution must be 32 to 320"));
    }
    // A right-handed frame with z along the view: u, v across it.
    let seed = if d.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = (seed - d * seed.dot(d)).normalize();
    let v = d.cross(u);
    let base: Vec<[f32; 2]> = pts.iter().map(|p| [p.dot(u), p.dot(v)]).collect();
    let object = s.scene.get(id).expect("checked");
    let b = object.mesh.bounds();
    let corners = (0..8).map(|i| Vec3::new(if i & 1 == 0 { b.min.x } else { b.max.x }, if i & 2 == 0 { b.min.y } else { b.max.y }, if i & 4 == 0 { b.min.z } else { b.max.z }).dot(d));
    let (near, far) = corners.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), x| (lo.min(x), hi.max(x)));
    let size = (b.max - b.min).max_element().max(1e-4);
    let depth = (far - near) + size * 0.5;
    let flat: Vec<Vec2> = match shape {
        "curve" => crate::mask_cmds::catmull_rom_closed(&base).into_iter().map(|p| Vec2::new(p[0], p[1])).collect(),
        "path" => {
            let width = match args.get("stroke_width") {
                None | Some(Value::Null) => size * 0.05,
                Some(w) => w.as_f64().filter(|w| *w > 0.0 && *w <= 1e5).ok_or_else(|| bad("stroke_width must be above 0 and at most 100000"))? as f32,
            };
            crate::mask_cmds::stroke_polygon(&base, width * 0.5).into_iter().map(|p| Vec2::new(p[0], p[1])).collect()
        }
        "line" => {
            let (a, c) = (Vec2::new(base[0][0], base[0][1]), Vec2::new(base[1][0], base[1][1]));
            if (c - a).length_squared() < 1e-12 {
                return Err(bad("the two line points must differ"));
            }
            let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
            for i in 0..8 {
                let q = Vec3::new(if i & 1 == 0 { b.min.x } else { b.max.x }, if i & 2 == 0 { b.min.y } else { b.max.y }, if i & 4 == 0 { b.min.z } else { b.max.z });
                let q = Vec2::new(q.dot(u), q.dot(v));
                lo = lo.min(q);
                hi = hi.max(q);
            }
            let margin = Vec2::splat(size * 0.25);
            let (lo, hi) = (lo - margin, hi + margin);
            let side = |q: Vec2| (q.x - a.x) * (c.y - a.y) - (q.y - a.y) * (c.x - a.x);
            let rect = [lo, Vec2::new(hi.x, lo.y), hi, Vec2::new(lo.x, hi.y)];
            let mut out = Vec::new();
            for k in 0..4 {
                let (p, q) = (rect[k], rect[(k + 1) % 4]);
                let (fp, fq) = (side(p), side(q));
                if fp > 0.0 {
                    out.push(p);
                }
                if (fp > 0.0) != (fq > 0.0) {
                    out.push(p + (q - p) * (fp / (fp - fq)));
                }
            }
            if out.len() < 3 {
                return Err(CommandError::new("empty_result", "the line does not cross the object's view"));
            }
            out
        }
        _ => base.iter().map(|p| Vec2::new(p[0], p[1])).collect(),
    };
    let mut cutter = fsc_topo::extrude_shape(&[flat], depth, 0.0, Some(resolution.min(256) as u32)).map_err(|e| CommandError::new("cut_failed", e))?;
    let middle = (near + far) * 0.5;
    for p in cutter.positions.iter_mut() {
        let q = Vec3::from_array(*p);
        *p = (u * q.x + v * q.y + d * (q.z + middle)).to_array();
    }
    cutter.recompute_normals();
    let cutter_bvh = fsc_core::Bvh::build(&cutter);
    let op = if inside { fsc_topo::BooleanOp::Intersection } else { fsc_topo::BooleanOp::Difference };
    let mesh = fsc_topo::voxel_boolean_blend(&object.mesh, &object.bvh, &cutter, &cutter_bvh, op, round, resolution as u32).map_err(|e| CommandError::new("cut_failed", e))?;
    if mesh.triangle_count() == 0 {
        return Err(CommandError::new("empty_result", if inside { "the outline covers none of the object" } else { "the outline covers the whole object" }));
    }
    let mask = vec![0.0; mesh.vertex_count()];
    apply(s, id, "Cut", mesh, mask)
}
