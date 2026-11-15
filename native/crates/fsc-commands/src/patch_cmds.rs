use fsc_core::{Mesh, ObjectId};
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn vec3_of(v: &Value, what: &str) -> Result<Vec3, CommandError> {
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| bad(format!("{what} is [x, y, z]")))?;
    let n = |i: usize| a[i].as_f64().filter(|x| x.is_finite()).map(|x| x as f32).ok_or_else(|| bad(format!("{what} is [x, y, z] of numbers")));
    Ok(Vec3::new(n(0)?, n(1)?, n(2)?))
}

pub(crate) fn wire_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object whose edges to list (a retopology object); the active object when omitted."},
            "eye": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A camera position; each edge then says whether it is in view from there."},
            "occluder": {"type": "integer", "minimum": 1, "description": "The object that can hide the edges (the sculpt under a retopology object); the object itself when omitted."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// A mesh's edges as segments for drawing a wireframe: a quad mesh's quad edges (no diagonals), else its triangle
/// edges, at most 20000; given an eye, each says whether the occluder hides its middle from there. Read-only.
pub(crate) fn get_wire(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = crate::sculpt_cmds::target(s, args)?;
    let eye = match args.get("eye").filter(|v| !v.is_null()) {
        Some(v) => Some(vec3_of(v, "eye")?),
        None => None,
    };
    let occluder_id = args.get("occluder").and_then(Value::as_u64).map(|v| v as ObjectId).unwrap_or(id);
    let occluder = s.scene.get(occluder_id).ok_or_else(|| CommandError::new("not_found", format!("no object {occluder_id}")))?;
    let object = s.scene.get(id).expect("checked");
    let m = &object.mesh;
    let mut edges: Vec<(u32, u32)> = match m.quad_faces() {
        Some(quads) => quads.iter().flat_map(|q| (0..4).map(move |k| (q[k].min(q[(k + 1) % 4]), q[k].max(q[(k + 1) % 4])))).collect(),
        None => (0..m.triangle_count()).flat_map(|t| {
            let [a, b, c] = m.triangle(t);
            [(a.min(b), a.max(b)), (b.min(c), b.max(c)), (c.min(a), c.max(a))]
        }).collect(),
    };
    edges.sort_unstable();
    edges.dedup();
    let total = edges.len();
    edges.truncate(20000);
    let size = occluder.mesh.bounds().extent().length().max(1e-6);
    let segments: Vec<Value> = edges.iter().map(|&(a, b)| {
        let (pa, pb) = (m.position(a), m.position(b));
        let visible = eye.map(|e| {
            let mid = (pa + pb) * 0.5;
            let d = mid.distance(e);
            // A straight edge between two surface points dips under a curved surface by about length squared over
            // eight radii; a quarter of its length covers any curve tighter than half its length.
            let slack = size * 2e-3 + pa.distance(pb) * 0.25;
            match occluder.bvh.raycast(&occluder.mesh, e, (mid - e) / d.max(1e-9)) {
                Some(h) => h.t >= d - slack,
                None => true,
            }
        });
        json!({"a": pa.to_array(), "b": pb.to_array(), "visible": visible})
    }).collect();
    Ok(json!({"id": id, "revision": object.revision, "edges": segments, "total": total}))
}

pub(crate) fn patch_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "target": {"type": "integer", "minimum": 1, "description": "The sculpt to retopologize; the patch is laid onto its surface."},
            "into": {"type": "integer", "minimum": 1, "description": "The retopology object to add to (from retopo_face or an earlier patch); a new one when omitted."},
            "patch": {"type": "string", "enum": fsc_topo::PATCH_KINDS.iter().chain(fsc_topo::WRAP_KINDS).chain(&["skeleton"]).collect::<Vec<_>>(), "description": "grid: columns by rows of quads; strip: a single row; eye_loop: rings of quads around an almond opening (an eye socket); mouth_loop: rings around a wider, flatter opening; disc: rings closed in the middle by a fan of quads (the centre is a pole of points / 2 edges, so 10 points make a five-edge pole). Whole-object templates, which wrap the entire target instead of a patch of it (no center or size): sphere: a closed quad sphere (a cube's faces split columns by columns, eight three-edge poles); head: that sphere with two eye loops and a mouth loop of rows rings each on the side facing front. The template is scaled to the target's extent along up and front, put on its outermost surface by rays from its middle (so it suits heads, blobs and other shapes seen whole from their middle) and relaxed along the surface; folded in the reply counts quads that came out facing in. skeleton: for an object made by skeleton_mesh (a figure, an animal, a hand), all quads over the whole object with edge loops ringing each limb and running along it, from a field remesh held along every bone away from the joints (faces sets the count)."},
            "faces": {"type": "integer", "minimum": 100, "maximum": 100000, "default": 2000, "description": "skeleton: about how many quads."},
            "center": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A point on or near the target's surface where the patch's middle goes. Not used by head and sphere."},
            "front": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "head and sphere: the direction the template's front faces (toward the viewer; the window sends the direction to the camera); +z when omitted."},
            "size": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000, "description": "Half the patch's width in world units; 6 percent of the target's size (its bounding diagonal) when omitted."},
            "up": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Which way the patch's top points, flattened onto the surface (the window sends the screen's up); world up when omitted."},
            "angle": {"type": "number", "minimum": -360, "maximum": 360, "default": 0, "description": "A further turn about the surface normal, in degrees."},
            "columns": {"type": "integer", "minimum": 1, "maximum": 64, "description": "grid and strip: quads across (default 4). Loops and disc: points around (even, default 12). head and sphere: quads along each cube edge (2 to 32, default 8; head default 10, rounded to twice an odd number from 6, so the mouth sits on the middle column)."},
            "rows": {"type": "integer", "minimum": 1, "maximum": 64, "description": "grid: quads up (default 4). Loops and disc: rings (default 3). head: rings around each eye and the mouth (1 to 6, default 2)."},
            "weld": {"type": "number", "exclusiveMinimum": 0, "maximum": 100, "description": "Patch points within this distance of an existing retopology vertex reuse it, so patches join; when omitted, 1.5 percent of the target's size or a quarter of the patch's shortest edge, whichever is smaller."}
        },
        "required": ["target", "patch"],
        "additionalProperties": false
    })
}

/// Lays a patch from the library onto the target's surface: the flat layout is placed in the tangent plane at the
/// centre (turned by up and angle), each point is dropped onto the surface along the normal, points weld to existing
/// retopology vertices, and each quad is turned to face out. One undo step.
pub(crate) fn retopo_patch(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let target_id = args.get("target").and_then(Value::as_u64).ok_or_else(|| bad("target is required"))? as ObjectId;
    let target = s.scene.get(target_id).ok_or_else(|| CommandError::new("not_found", format!("no object {target_id}")))?;
    let kind = args.get("patch").and_then(Value::as_str).ok_or_else(|| bad("patch is required"))?;
    let wrap = fsc_topo::WRAP_KINDS.contains(&kind);
    let loops = kind != "grid" && kind != "strip";
    let count = |k: &str, d: u64| -> Result<usize, CommandError> {
        match args.get(k) {
            None | Some(Value::Null) => Ok(d as usize),
            Some(v) => v.as_u64().filter(|n| (1..=64).contains(n)).map(|n| n as usize).ok_or_else(|| bad(format!("{k} is an integer from 1 to 64"))),
        }
    };
    let m = &target.mesh;
    let bsize = m.bounds().extent().length().max(1e-4);
    let num = |k: &str, d: f32, lo: f32, hi: f32| -> Result<f32, CommandError> {
        match args.get(k) {
            None | Some(Value::Null) => Ok(d),
            Some(v) => v.as_f64().map(|x| x as f32).filter(|x| x.is_finite() && *x >= lo && *x <= hi).ok_or_else(|| bad(format!("{k} must be a number from {lo} to {hi}"))),
        }
    };
    let up_arg = match args.get("up").filter(|v| !v.is_null()) {
        Some(v) => vec3_of(v, "up")?,
        None => Vec3::Y,
    };
    if kind == "skeleton" {
        // Quads following the target's own skeleton: a field remesh held along each bone, so loops ring every limb.
        let (record, _) = crate::rig_cmds::made(s, target_id, "skeleton").map_err(|_| CommandError::new("no_skeleton", "the skeleton template needs an object made by skeleton_mesh in this session (Build > Skeleton); use head or sphere for other sculpts"))?;
        let point = |v: &Value| -> Option<Vec3> { Some(Vec3::new(v.get(0)?.as_f64()? as f32, v.get(1)?.as_f64()? as f32, v.get(2)?.as_f64()? as f32)) };
        let spheres: Vec<Vec3> = record["spheres"].as_array().map(|a| a.iter().filter_map(point).collect()).unwrap_or_default();
        let bones: Vec<(Vec3, Vec3)> = record["links"]
            .as_array()
            .map(|a| a.iter().filter_map(|l| Some((*spheres.get(l.get(0)?.as_u64()? as usize)?, *spheres.get(l.get(1)?.as_u64()? as usize)?))).collect())
            .unwrap_or_default();
        if bones.is_empty() {
            return Err(CommandError::new("no_skeleton", "the skeleton has no bones (links) to follow"));
        }
        let faces = match args.get("faces") {
            None | Some(Value::Null) => 2000,
            Some(v) => v.as_u64().filter(|n| (100..=100_000).contains(n)).ok_or_else(|| bad("faces is an integer from 100 to 100000"))? as usize,
        };
        let opts = fsc_topo::FieldOptions { target_faces: faces, hard_angle: 60.0, ..Default::default() };
        let out = fsc_topo::field_remesh_steered(m, &target.bvh, opts, &[], &bones).map_err(|e| CommandError::new("remesh_failed", e))?;
        let quads = out.mesh.quad_faces().ok_or_else(|| CommandError::new("remesh_failed", "the remesh did not come out as quads"))?;
        let points: Vec<Vec3> = out.mesh.positions.iter().map(|&p| Vec3::from(p)).collect();
        let weld = num("weld", if args.get("into").is_some_and(|v| !v.is_null()) { bsize * 0.005 } else { 1e-6 }, 1e-6, 100.0)?;
        let mut reply = lay(s, args, target_id, kind, &points, &quads, None, weld, None)?;
        reply["bone_vertices"] = json!(out.bone_vertices);
        reply["bones"] = json!(bones.len());
        return Ok(reply);
    }
    if wrap {
        // A whole-object template: no centre or size, it wraps the target as seen from front.
        let res = count("columns", if kind == "head" { 10 } else { 8 })?;
        let rings = count("rows", 2)?;
        let front = match args.get("front").filter(|v| !v.is_null()) {
            Some(v) => vec3_of(v, "front")?,
            None => Vec3::Z,
        };
        let w = fsc_topo::wrap_template(kind, res, rings, m, &target.bvh, up_arg, front, 8).map_err(bad)?;
        let shortest = w.quads.iter().flat_map(|q| (0..4).map(move |k| (q[k], q[(k + 1) % 4]))).map(|(a, b)| w.points[a as usize].distance(w.points[b as usize])).fold(f32::MAX, f32::min);
        // Welding joins a template to a retopology object it is added into; in a new object its own points must stay
        // apart even where the surface squeezes two of them together.
        let default_weld = if args.get("into").is_some_and(|v| !v.is_null()) { (bsize * 0.015).min(shortest * 0.25) } else { 1e-6 };
        let weld = num("weld", default_weld, 1e-6, 100.0)?;
        return lay(s, args, target_id, kind, &w.points, &w.quads, None, weld, Some(w.folded));
    }
    let flat = fsc_topo::patch(kind, count("columns", if loops { 12 } else { 4 })?, count("rows", if loops { 3 } else { 4 })?).map_err(bad)?;
    let size = num("size", bsize * 0.06, 1e-6, 1000.0)?;
    // By default a quarter of the patch's shortest edge at most, so a fine patch never welds onto itself.
    let shortest = flat.quads.iter().flat_map(|q| (0..4).map(move |k| (q[k], q[(k + 1) % 4]))).map(|(a, b)| {
        let (p, q) = (flat.points[a as usize], flat.points[b as usize]);
        ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt()
    }).fold(f32::MAX, f32::min) * size;
    let weld = num("weld", (bsize * 0.015).min(shortest * 0.25), 1e-6, 100.0)?;
    let angle = num("angle", 0.0, -360.0, 360.0)?.to_radians();
    let center = vec3_of(args.get("center").ok_or_else(|| bad("center is required"))?, "center")?;
    let (c, n, _) = fsc_topo::nearest_point(&target.bvh, m, center, bsize).ok_or_else(|| bad("center is too far from the target's surface"))?;
    let n = n.try_normalize().ok_or_else(|| bad("the surface has no normal there"))?;
    let up = up_arg;
    // The patch's y axis: up flattened onto the tangent plane (world x when up runs along the normal), then turned.
    let mut y = (up - n * up.dot(n)).try_normalize().unwrap_or_else(|| (Vec3::X - n * n.x).normalize_or_zero());
    let x0 = y.cross(n);
    let (sin, cos) = angle.sin_cos();
    let x = x0 * cos + y * sin;
    y = n.cross(x);
    let mut dropped = Vec::with_capacity(flat.points.len());
    let mut normals = Vec::with_capacity(flat.points.len());
    for p in &flat.points {
        let w = c + x * (p[0] * size) + y * (p[1] * size);
        // Straight down onto the surface along the normal, else the nearest surface point.
        let hit = target.bvh.raycast(m, w + n * size, -n).filter(|h| h.t < size * 2.5);
        let (q, qn) = match hit {
            Some(h) => {
                let [a, b, cc] = m.triangle(h.triangle as usize).map(|v| m.position(v));
                (h.point, (b - a).cross(cc - a).normalize_or_zero())
            }
            None => {
                let (q, qn, _) = fsc_topo::nearest_point(&target.bvh, m, w, bsize).ok_or_else(|| bad("the patch reaches past the target's surface"))?;
                (q, qn)
            }
        };
        dropped.push(q);
        normals.push(qn);
    }
    lay(s, args, target_id, kind, &dropped, &flat.quads, Some(&normals), weld, None)
}

/// Adds laid quads (points on the target, quads over them) to the retopology object `into` or a new one, welding
/// points onto existing vertices within `weld`. With `normals` each quad is turned to face like the surface under it
/// (a flat patch); without, the quads keep their winding (a whole template, already wound outward). One undo step.
#[allow(clippy::too_many_arguments)]
fn lay(s: &mut Session, args: &Value, target_id: ObjectId, kind: &str, dropped: &[Vec3], patch_quads: &[[u32; 4]], normals: Option<&[Vec3]>, weld: f32, folded: Option<usize>) -> Result<Value, CommandError> {
    let target_name = s.scene.get(target_id).expect("checked").name.clone();
    let into = args.get("into").and_then(Value::as_u64).map(|v| v as ObjectId);
    if let Some(id) = into.filter(|id| s.scene.get(*id).is_some()) {
        crate::multires_cmds::guard(s, id, "Adding retopology faces")?;
    }
    let mut mesh = match into {
        Some(id) => s.scene.get(id).ok_or_else(|| CommandError::new("not_found", format!("no object {id}")))?.mesh.clone(),
        None => Mesh::new(Vec::new(), Vec::new()),
    };
    let was_quads = mesh.quads || mesh.indices.is_empty();
    let before = mesh.vertex_count();
    let mut index = Vec::with_capacity(dropped.len());
    // Weld partners come from a grid of weld-sized cells (the neighbouring cells hold every point in reach), so a
    // template of many thousand points costs no more per point than a small patch.
    let cell = weld.max(1e-5);
    let key = |p: Vec3| ((p.x / cell).floor() as i64, (p.y / cell).floor() as i64, (p.z / cell).floor() as i64);
    let mut grid: std::collections::HashMap<(i64, i64, i64), Vec<u32>> = std::collections::HashMap::new();
    for (i, &p) in mesh.positions.iter().enumerate() {
        grid.entry(key(Vec3::from(p))).or_default().push(i as u32);
    }
    for q in dropped {
        let (cx, cy, cz) = key(*q);
        let mut found: Option<u32> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(list) = grid.get(&(cx + dx, cy + dy, cz + dz)) {
                        // The lowest index in reach, as the linear search before it chose.
                        for &v in list {
                            if Vec3::from(mesh.positions[v as usize]).distance(*q) <= weld && found.is_none_or(|f| v < f) {
                                found = Some(v);
                            }
                        }
                    }
                }
            }
        }
        index.push(match found {
            Some(v) => v,
            None => {
                mesh.positions.push(q.to_array());
                let v = mesh.positions.len() as u32 - 1;
                grid.entry((cx, cy, cz)).or_default().push(v);
                v
            }
        });
    }
    let welded = dropped.len() - (mesh.vertex_count() - before);
    let mut faces = 0;
    for quad in patch_quads {
        let mut f = quad.map(|k| index[k as usize]);
        let mut u = f.to_vec();
        u.sort_unstable();
        u.dedup();
        if u.len() < 4 {
            return Err(bad("two patch points welded onto one vertex; make the patch larger or lower weld"));
        }
        if let Some(normals) = normals {
            let p = quad.map(|k| dropped[k as usize]);
            let surface: Vec3 = quad.iter().map(|&k| normals[k as usize]).sum();
            if (p[2] - p[0]).cross(p[3] - p[1]).dot(surface) < 0.0 {
                f.reverse();
            }
        }
        mesh.indices.extend_from_slice(&[f[0], f[1], f[2], f[0], f[2], f[3]]);
        faces += 1;
    }
    let positions = std::mem::take(&mut mesh.positions);
    let indices = std::mem::take(&mut mesh.indices);
    let mut rebuilt = Mesh::new(positions, indices);
    rebuilt.quads = was_quads;
    let total = if rebuilt.quads { rebuilt.triangle_count() / 2 } else { rebuilt.triangle_count() };
    let id = match into {
        Some(id) => {
            let object = s.scene.get_mut(id).expect("checked");
            let mask = vec![0.0; rebuilt.vertex_count()];
            let (old_mesh, old_mask, texture) = crate::history::replace_mesh(object, rebuilt, mask);
            s.history.push(Edit::Topology { label: "Retopology Patch".into(), object: id, mesh: old_mesh, mask: old_mask }.with_texture(id, texture));
            id
        }
        None => {
            let name = s.scene.unique_name(&format!("{target_name} retopo"));
            let id = s.scene.add(name, rebuilt);
            let index = s.scene.index_of(id).expect("just added");
            s.history.push(Edit::Add { id, index, parked: None });
            id
        }
    };
    // The sculpt stays active so the next clicks land on it.
    s.scene.active = Some(target_id);
    let object = s.scene.get(id).expect("exists");
    let mut reply = json!({"object": summary(object), "id": id, "patch": kind, "faces_added": faces, "faces": total, "points_welded": welded, "quads": object.mesh.quads});
    if let Some(n) = folded {
        reply["folded"] = json!(n);
    }
    Ok(reply)
}
