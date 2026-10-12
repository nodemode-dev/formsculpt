//! Simulation and grooming on the sculpted mesh: cloth draped over other objects, posing by rotating the unmasked part
//! about a joint, fibers grown from the surface, and the Substance Painter handoff.

use fsc_core::time::Instant;
use fsc_core::{Mesh, ObjectId};
use glam::{Quat, Vec3};
use serde_json::{json, Value};

use crate::history::Edit;
use crate::object_cmds::set_positions;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn num(args: &Value, key: &str, default: f64, lo: f64, hi: f64) -> Result<f32, CommandError> {
    let v = args.get(key).and_then(Value::as_f64).unwrap_or(default);
    if !(lo..=hi).contains(&v) {
        return Err(bad(format!("{key} must be {lo} to {hi}")));
    }
    Ok(v as f32)
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

/// A seeded random stream (xorshift32).
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

pub(crate) fn cloth_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The cloth: usually a plane (add_primitive kind plane, detail 48 to 96) placed above what it should fall on; the active object when omitted."},
            "colliders": {"type": "array", "items": {"type": "integer", "minimum": 1}, "description": "Objects the cloth lands on and cannot pass through; every other visible object when omitted."},
            "steps": {"type": "integer", "minimum": 1, "maximum": 2000, "default": 180, "description": "Simulation steps of 1/60 s; more lets it fall further and settle."},
            "gravity": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Direction and strength of the pull in world units per second squared; [0, -9.8, 0] when omitted."},
            "stiffness": {"type": "number", "minimum": 0.05, "maximum": 1, "default": 0.9, "description": "How firmly edges keep their length (1 is stiff cotton, lower is stretchy)."},
            "thickness": {"type": "number", "exclusiveMinimum": 0, "maximum": 100, "description": "How far the cloth stays off colliders; 1 percent of the cloth's size when omitted."},
            "floor": {"type": "number", "description": "A floor height the cloth rests on; none when omitted."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Drapes a cloth: position-based dynamics (Verlet integration, edge-length constraints, collision with the colliders'
/// surfaces through their BVHs). Masked vertices (mask above 0.5) are pinned where they are. One undo step.
pub(crate) fn cloth_drape(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let started = Instant::now();
    let steps = num(args, "steps", 180.0, 1.0, 2000.0)? as usize;
    let gravity = vec3(args, "gravity")?.unwrap_or(Vec3::new(0.0, -9.8, 0.0));
    let stiffness = num(args, "stiffness", 0.9, 0.05, 1.0)?;
    let floor = args.get("floor").and_then(Value::as_f64).map(|f| f as f32);
    let colliders: Vec<ObjectId> = match args.get("colliders").and_then(Value::as_array) {
        Some(list) => list.iter().filter_map(Value::as_u64).map(|v| v as ObjectId).collect(),
        None => s.scene.objects.iter().filter(|o| o.visible && o.id != id).map(|o| o.id).collect(),
    };
    if let Some(missing) = colliders.iter().find(|c| s.scene.get(**c).is_none() || **c == id) {
        return Err(bad(format!("collider {missing} is not another object in the scene")));
    }
    let cloth = s.scene.get(id).expect("checked");
    let size = cloth.mesh.bounds().extent().length().max(1e-4);
    let thickness = match args.get("thickness").and_then(Value::as_f64) {
        Some(t) if t > 0.0 => t as f32,
        Some(_) => return Err(bad("thickness must be above 0")),
        None => size * 0.01,
    };
    let mut pos: Vec<Vec3> = cloth.mesh.positions.iter().map(|&p| Vec3::from(p)).collect();
    let pinned: Vec<bool> = (0..pos.len()).map(|v| cloth.mask.get(v).copied().unwrap_or(0.0) > 0.5).collect();
    let mut edges: Vec<(u32, u32, f32)> = Vec::new();
    {
        let mut seen = std::collections::HashSet::new();
        for t in cloth.mesh.indices.chunks_exact(3) {
            for k in 0..3 {
                let (a, b) = (t[k].min(t[(k + 1) % 3]), t[k].max(t[(k + 1) % 3]));
                if seen.insert((a, b)) {
                    edges.push((a, b, pos[a as usize].distance(pos[b as usize])));
                }
            }
        }
    }
    let mut prev = pos.clone();
    let dt = 1.0 / 60.0;
    let pull = gravity * dt * dt;
    let solids: Vec<(&Mesh, &fsc_core::Bvh)> = colliders.iter().map(|c| s.scene.get(*c).expect("checked")).map(|o| (&o.mesh, &o.bvh)).collect();
    let mut contacts = 0usize;
    for _ in 0..steps {
        for v in 0..pos.len() {
            if pinned[v] {
                continue;
            }
            let velocity = (pos[v] - prev[v]) * 0.99;
            prev[v] = pos[v];
            pos[v] += velocity + pull;
        }
        for _ in 0..6 {
            for &(a, b, rest) in &edges {
                let (ia, ib) = (a as usize, b as usize);
                let (wa, wb) = (if pinned[ia] { 0.0 } else { 1.0 }, if pinned[ib] { 0.0 } else { 1.0 });
                if wa + wb == 0.0 {
                    continue;
                }
                let delta = pos[ib] - pos[ia];
                let len = delta.length();
                if len < 1e-9 {
                    continue;
                }
                let fix = delta * ((len - rest) / len * stiffness / (wa + wb));
                pos[ia] += fix * wa;
                pos[ib] -= fix * wb;
            }
        }
        contacts = 0;
        for v in 0..pos.len() {
            if pinned[v] {
                continue;
            }
            for &(mesh, bvh) in &solids {
                if let Some((q, n, _)) = fsc_topo::nearest_point(bvh, mesh, pos[v], thickness * 4.0) {
                    let n = n.normalize_or(Vec3::Y);
                    if (pos[v] - q).dot(n) < thickness {
                        pos[v] = q + n * thickness;
                        // Friction: the contact keeps most of its last position along the surface.
                        prev[v] = prev[v].lerp(pos[v], 0.6);
                        contacts += 1;
                    }
                }
            }
            if let Some(f) = floor {
                if pos[v].y < f + thickness {
                    pos[v].y = f + thickness;
                    prev[v] = prev[v].lerp(pos[v], 0.6);
                    contacts += 1;
                }
            }
        }
    }
    let positions: Vec<[f32; 3]> = pos.iter().map(|p| p.to_array()).collect();
    let mut out = set_positions(s, id, "Drape Cloth", positions);
    out["contacts"] = json!(contacts);
    out["ms"] = json!(started.elapsed().as_millis());
    Ok(out)
}

pub(crate) fn pose_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "pivot": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The joint to turn about (a shoulder, an elbow, a neck), in world space."},
            "axis": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The hinge direction; [0, 0, 1] when omitted."},
            "angle": {"type": "number", "minimum": -360, "maximum": 360, "description": "Degrees to turn, counter-clockwise about the axis."},
            "falloff": {"type": "number", "minimum": 0, "maximum": 100, "default": 0, "description": "Spread the bend over this distance from the pivot (world units), so the joint bends smoothly instead of hinging."}
        },
        "required": ["pivot", "angle"],
        "additionalProperties": false
    })
}

/// Poses a part as ZBrush's Transpose does: mask what must stay, and the unmasked part turns about the joint; partial
/// mask turns partly, and falloff spreads the bend near the pivot. One undo step.
pub(crate) fn pose_rotate(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let pivot = vec3(args, "pivot")?.ok_or_else(|| bad("pivot is required"))?;
    let axis = vec3(args, "axis")?.unwrap_or(Vec3::Z).try_normalize().ok_or_else(|| bad("axis must not be zero"))?;
    let angle = num(args, "angle", 0.0, -360.0, 360.0)?.to_radians();
    if args.get("angle").is_none() {
        return Err(bad("angle is required"));
    }
    let falloff = num(args, "falloff", 0.0, 0.0, 100.0)?;
    let object = s.scene.get(id).expect("checked");
    if object.mask.iter().all(|&m| m <= 0.0) {
        return Err(CommandError::new("no_mask", "mask the part that must stay first (mask_shape, the mask brush or mask_by_group); the unmasked part turns"));
    }
    let positions: Vec<[f32; 3]> = object
        .mesh
        .positions
        .iter()
        .enumerate()
        .map(|(v, &p)| {
            let p = Vec3::from(p);
            let mut w = 1.0 - object.mask[v].clamp(0.0, 1.0);
            if falloff > 0.0 {
                let d = p.distance(pivot) / falloff;
                w *= (d * d * (3.0 - 2.0 * d)).clamp(0.0, 1.0).max(if d >= 1.0 { 1.0 } else { 0.0 });
            }
            if w <= 0.0 {
                return p.to_array();
            }
            (pivot + Quat::from_axis_angle(axis, angle * w) * (p - pivot)).to_array()
        })
        .collect();
    Ok(set_positions(s, id, "Pose", positions))
}

pub(crate) fn fibers_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object to grow from; the active object when omitted. Fibers grow from its masked area, or everywhere without a mask."},
            "count": {"type": "integer", "minimum": 1, "maximum": 50000, "default": 1500},
            "length": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000, "description": "Fiber length; 15 percent of the object's size when omitted."},
            "radius": {"type": "number", "exclusiveMinimum": 0, "maximum": 100, "description": "Root radius (fibers taper to a third at the tip); a fortieth of the length when omitted."},
            "droop": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.4, "description": "How much fibers bend toward gravity (0 stands straight out, 1 hangs)."},
            "clump": {"type": "number", "minimum": 0, "maximum": 1, "default": 0, "description": "How much tips gather toward their neighbours' tips (locks of hair)."},
            "segments": {"type": "integer", "minimum": 2, "maximum": 16, "default": 5, "description": "Points along each fiber."},
            "seed": {"type": "integer", "minimum": 0, "default": 1},
            "color": {"type": ["string", "array"], "description": "Paint the fibers this colour (#rrggbb or [r, g, b])."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// FiberMesh: grows fibers from the surface as one new object of thin tapered tubes, placed at random by area over the
/// masked faces (or all faces), each leaving along the surface normal and drooping toward gravity. One undo step.
pub(crate) fn fibers_grow(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let started = Instant::now();
    let count = num(args, "count", 1500.0, 1.0, 50000.0)? as usize;
    let segments = num(args, "segments", 5.0, 2.0, 16.0)? as usize;
    let droop = num(args, "droop", 0.4, 0.0, 1.0)?;
    let clump = num(args, "clump", 0.0, 0.0, 1.0)?;
    let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(1) as u32;
    let color = crate::sculpt_cmds::parse_color(args.get("color"))?;
    let object = s.scene.get(id).expect("checked");
    let mesh = &object.mesh;
    let size = mesh.bounds().extent().length().max(1e-4);
    let length = match args.get("length").and_then(Value::as_f64) {
        Some(l) => l as f32,
        None => size * 0.15,
    };
    let radius = match args.get("radius").and_then(Value::as_f64) {
        Some(r) => r as f32,
        None => length / 40.0,
    };
    let masked = object.mask.iter().any(|&m| m > 0.5);
    let faces: Vec<usize> = (0..mesh.triangle_count()).filter(|&t| !masked || mesh.triangle(t).iter().all(|&v| object.mask[v as usize] > 0.5)).collect();
    if faces.is_empty() {
        return Err(bad("no faces to grow from"));
    }
    let area = |t: usize| {
        let [a, b, c] = mesh.triangle(t).map(|v| mesh.position(v));
        (b - a).cross(c - a).length() * 0.5
    };
    let mut cumulative = Vec::with_capacity(faces.len());
    let mut total = 0.0;
    for &t in &faces {
        total += area(t);
        cumulative.push(total);
    }
    let mut rng = Rng(seed.wrapping_mul(2_654_435_761) | 1);
    let mut roots: Vec<(Vec3, Vec3)> = Vec::with_capacity(count);
    for _ in 0..count {
        let pick = rng.next() * total;
        let k = cumulative.partition_point(|&c| c < pick).min(faces.len() - 1);
        let [ia, ib, ic] = mesh.triangle(faces[k]);
        let (mut u, mut v) = (rng.next(), rng.next());
        if u + v > 1.0 {
            u = 1.0 - u;
            v = 1.0 - v;
        }
        let w = 1.0 - u - v;
        let p = mesh.position(ia) * w + mesh.position(ib) * u + mesh.position(ic) * v;
        let n = (Vec3::from(mesh.normals[ia as usize]) * w + Vec3::from(mesh.normals[ib as usize]) * u + Vec3::from(mesh.normals[ic as usize]) * v).normalize_or(Vec3::Y);
        roots.push((p, n));
    }
    // Clumping pulls each tip toward the mean tip of a few seeded clump centres nearby.
    let centres: Vec<Vec3> = roots.iter().step_by(24.max(count / 60)).map(|r| r.0).collect();
    let down = Vec3::NEG_Y;
    let mut curves: Vec<Vec<(Vec3, f32)>> = Vec::with_capacity(roots.len());
    for (p, n) in &roots {
        let jitter = Vec3::new(rng.next() - 0.5, rng.next() - 0.5, rng.next() - 0.5) * 0.3;
        let toward = centres.iter().min_by(|a, b| a.distance(*p).total_cmp(&b.distance(*p))).copied().unwrap_or(*p);
        let mut pts: Vec<(Vec3, f32)> = Vec::with_capacity(segments);
        let mut at = *p - *n * radius * 0.5;
        let step = length / (segments - 1) as f32;
        for k in 0..segments {
            let t = k as f32 / (segments - 1) as f32;
            pts.push((at + (toward - *p) * (clump * t * t), radius * (1.0 - t * 0.66)));
            let dir = ((*n + jitter) * (1.0 - droop * t) + down * (droop * t * 1.5)).normalize_or(*n);
            at += dir * step;
        }
        curves.push(pts);
    }
    let out = crate::rig_cmds::fiber_mesh(&curves, color);
    if out.indices.is_empty() {
        return Err(bad("the fibers came out empty; give a larger length or radius"));
    }
    let name = s.scene.unique_name(&format!("{} fibers", s.scene.get(id).expect("checked").name));
    let new_id = s.scene.add(name, out);
    let index = s.scene.index_of(new_id).expect("just added");
    s.history.push(Edit::Add { id: new_id, index, parked: None });
    // The guide curves stay with the object so groom_fibers can comb, cut, curl and lengthen them.
    if let Some(o) = s.scene.get_mut(new_id) {
        let mut record = crate::rig_cmds::fiber_record(&curves, color);
        record["fp"] = json!(crate::rig_cmds::fingerprint(&o.mesh));
        o.primitive = Some((record.to_string(), u64::MAX));
    }
    Ok(json!({"object": summary(s.scene.get(new_id).expect("just added")), "fibers": roots.len(), "active": new_id, "ms": started.elapsed().as_millis()}))
}

pub(crate) fn retopo_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "target": {"type": "integer", "minimum": 1, "description": "The sculpt to retopologize; the points snap onto its surface."},
            "into": {"type": "integer", "minimum": 1, "description": "The retopology object to add to (from an earlier call's id); a new one when omitted."},
            "points": {"type": "array", "items": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}, "minItems": 3, "maxItems": 4, "description": "Three or four corners of the new face, in order around it, near the target's surface."},
            "weld": {"type": "number", "exclusiveMinimum": 0, "maximum": 100, "description": "Corners within this distance of an existing vertex reuse it, so faces join; 1.5 percent of the target's size when omitted."}
        },
        "required": ["target", "points"],
        "additionalProperties": false
    })
}

/// Manual retopology, a face at a time: the corners snap onto the target's surface, reuse retopology vertices within
/// the weld distance so faces share edges, and the face turns to face out like the surface under it. Quads keep the
/// object quad-dominant for OBJ export, subdivision and projecting detail back. One undo step.
pub(crate) fn retopo_face(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let target_id = args.get("target").and_then(Value::as_u64).ok_or_else(|| bad("target is required"))? as ObjectId;
    let target_obj = s.scene.get(target_id).ok_or_else(|| CommandError::new("not_found", format!("no object {target_id}")))?;
    let pts: Vec<Vec3> = args
        .get("points")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("points is required"))?
        .iter()
        .map(|p| p.as_array().filter(|a| a.len() == 3).and_then(|a| Some(Vec3::new(a[0].as_f64()? as f32, a[1].as_f64()? as f32, a[2].as_f64()? as f32))).ok_or_else(|| bad("each point is [x, y, z]")))
        .collect::<Result<_, _>>()?;
    if !(3..=4).contains(&pts.len()) {
        return Err(bad("give three or four points"));
    }
    let size = target_obj.mesh.bounds().extent().length().max(1e-4);
    let weld = args.get("weld").and_then(Value::as_f64).map(|w| w as f32).unwrap_or(size * 0.015);
    let mut snapped = Vec::with_capacity(4);
    let mut normal = Vec3::ZERO;
    for p in &pts {
        let (q, n, _) = fsc_topo::nearest_point(&target_obj.bvh, &target_obj.mesh, *p, size).ok_or_else(|| bad("a point is too far from the target's surface"))?;
        snapped.push(q);
        normal += n;
    }
    let into = args.get("into").and_then(Value::as_u64).map(|v| v as ObjectId);
    let mut mesh = match into {
        Some(id) => s.scene.get(id).ok_or_else(|| CommandError::new("not_found", format!("no object {id}")))?.mesh.clone(),
        None => Mesh::new(Vec::new(), Vec::new()),
    };
    let was_quads = mesh.quads || mesh.indices.is_empty();
    let mut corner = Vec::with_capacity(4);
    for q in &snapped {
        let found = mesh.positions.iter().position(|&p| Vec3::from(p).distance(*q) <= weld);
        corner.push(match found {
            Some(v) => v as u32,
            None => {
                mesh.positions.push(q.to_array());
                mesh.positions.len() as u32 - 1
            }
        });
    }
    let mut unique = corner.clone();
    unique.sort_unstable();
    unique.dedup();
    if unique.len() < corner.len() {
        return Err(bad("two corners landed on the same vertex; place them further apart or lower weld"));
    }
    let face_n = if corner.len() == 4 {
        (snapped[2] - snapped[0]).cross(snapped[3] - snapped[1])
    } else {
        (snapped[1] - snapped[0]).cross(snapped[2] - snapped[0])
    };
    if face_n.dot(normal) < 0.0 {
        corner.reverse();
    }
    if corner.len() == 4 {
        mesh.indices.extend_from_slice(&[corner[0], corner[1], corner[2], corner[0], corner[2], corner[3]]);
    } else {
        mesh.indices.extend_from_slice(&corner);
    }
    let positions = std::mem::take(&mut mesh.positions);
    let indices = std::mem::take(&mut mesh.indices);
    let mut rebuilt = Mesh::new(positions, indices);
    rebuilt.quads = was_quads && corner.len() == 4;
    let faces = if rebuilt.quads { rebuilt.triangle_count() / 2 } else { rebuilt.triangle_count() };
    let id = match into {
        Some(id) => {
            let object = s.scene.get_mut(id).expect("checked");
            let mask = vec![0.0; rebuilt.vertex_count()];
            let (old_mesh, old_mask) = object.replace_mesh(rebuilt, mask);
            s.history.push(Edit::Topology { label: "Retopology Face".into(), object: id, mesh: old_mesh, mask: old_mask });
            id
        }
        None => {
            let name = s.scene.unique_name(&format!("{} retopo", target_obj.name));
            let id = s.scene.add(name, rebuilt);
            let index = s.scene.index_of(id).expect("just added");
            s.history.push(Edit::Add { id, index, parked: None });
            id
        }
    };
    // The sculpt stays active so the next clicks land on it.
    s.scene.active = Some(target_id);
    Ok(json!({"object": summary(s.scene.get(id).expect("exists")), "id": id, "faces": faces, "quads": s.scene.get(id).expect("exists").mesh.quads}))
}

pub(crate) fn edge_loop_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "A quad mesh (quad_remesh, a retopology object or a quad import); the active object when omitted."},
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A point on or near the edge the new loop crosses (from pick); the loop runs across it, around the ring of quads."}
        },
        "required": ["point"],
        "additionalProperties": false
    })
}

fn replace_topology(s: &mut Session, id: ObjectId, label: &str, mesh: Mesh) -> Value {
    let object = s.scene.get_mut(id).expect("checked");
    let mask = vec![0.0; mesh.vertex_count()];
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    object.maps = None;
    let after = summary(object);
    s.history.push(Edit::Topology { label: label.into(), object: id, mesh: old_mesh, mask: old_mask });
    after
}

/// Inserts an edge loop across the ring of quads through the edge nearest a point. One undo step.
pub(crate) fn edge_loop(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    crate::multires_cmds::guard(s, id, "An edge loop")?;
    let point = vec3(args, "point")?.ok_or_else(|| bad("point is required"))?;
    let (mesh, split) = fsc_topo::insert_edge_loop(&s.scene.get(id).expect("checked").mesh, point).map_err(|e| CommandError::new("not_quads", e))?;
    let after = replace_topology(s, id, "Insert Edge Loop", mesh);
    Ok(json!({"object": after, "quads_split": split}))
}

pub(crate) fn bridge_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "An object with open borders (merge two open objects, or open holes with delete_faces); the active object when omitted."},
            "near_a": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A point near the first border, when the object has more than two."},
            "near_b": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A point near the second border."},
            "segments": {"type": "integer", "minimum": 1, "maximum": 64, "default": 4, "description": "Rows of quads along the bridge."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Bridges two open borders with a band of quads, joining them. One undo step.
pub(crate) fn bridge(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    crate::multires_cmds::guard(s, id, "A bridge")?;
    let near = match (vec3(args, "near_a")?, vec3(args, "near_b")?) {
        (Some(a), Some(b)) => Some((a, b)),
        (None, None) => None,
        _ => return Err(bad("give both near_a and near_b, or neither")),
    };
    let segments = num(args, "segments", 4.0, 1.0, 64.0)? as usize;
    let mesh = fsc_topo::bridge_borders(&s.scene.get(id).expect("checked").mesh, near, segments).map_err(|e| CommandError::new("cannot_bridge", e))?;
    let after = replace_topology(s, id, "Bridge", mesh);
    let borders = fsc_topo::border_loops(&s.scene.get(id).expect("checked").mesh).len();
    Ok(json!({"object": after, "open_borders_left": borders}))
}

pub(crate) fn delete_faces_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "which": {"type": "string", "enum": ["hidden", "masked"], "default": "hidden", "description": "Delete the hidden faces (hide_faces), or the faces whose corners are all masked."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Deletes faces, opening the mesh there (for bridge, or to cut a part away), and drops vertices left unused. One undo
/// step.
pub(crate) fn delete_faces(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    crate::multires_cmds::guard(s, id, "Deleting faces")?;
    let which = args.get("which").and_then(Value::as_str).unwrap_or("hidden");
    let object = s.scene.get(id).expect("checked");
    let mesh = &object.mesh;
    let gone: Vec<bool> = match which {
        "hidden" if mesh.has_hidden() => mesh.hidden.clone(),
        "hidden" => return Err(CommandError::new("nothing_hidden", "no faces are hidden; hide_faces first")),
        "masked" => (0..mesh.triangle_count()).map(|t| mesh.triangle(t).iter().all(|&v| object.mask[v as usize] > 0.5)).collect(),
        other => return Err(bad(format!("which must be hidden or masked, not {other}"))),
    };
    let removed = gone.iter().filter(|&&g| g).count();
    if removed == 0 {
        return Err(bad("no faces matched"));
    }
    if removed == gone.len() {
        return Err(bad("that would delete every face; delete the object instead"));
    }
    let mut remap = vec![u32::MAX; mesh.vertex_count()];
    let (mut positions, mut paint, mut mask, mut indices, mut groups) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (t, &dropped) in gone.iter().enumerate() {
        if dropped {
            continue;
        }
        for v in mesh.triangle(t) {
            if remap[v as usize] == u32::MAX {
                remap[v as usize] = positions.len() as u32;
                positions.push(mesh.positions[v as usize]);
                mask.push(object.mask[v as usize]);
                if mesh.has_paint() {
                    paint.push(mesh.paint[v as usize]);
                }
            }
            indices.push(remap[v as usize]);
        }
        if mesh.has_groups() {
            groups.push(mesh.groups[t]);
        }
    }
    // Quads survive when each deleted quad went whole.
    let quads = mesh.quads && gone.chunks_exact(2).all(|p| p[0] == p[1]);
    let mut out = Mesh::new(positions, indices);
    out.paint = paint;
    out.groups = groups;
    out.quads = quads;
    let object = s.scene.get_mut(id).expect("checked");
    let (old_mesh, old_mask) = object.replace_mesh(out, mask);
    object.maps = None;
    let after = summary(object);
    s.history.push(Edit::Topology { label: "Delete Faces".into(), object: id, mesh: old_mesh, mask: old_mask });
    Ok(json!({"object": after, "deleted": removed}))
}

pub(crate) fn substance_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object to texture; the active object when omitted. It needs UVs (uv_unwrap)."},
            "folder": {"type": "string", "minLength": 1, "description": "Where to write the package (created if missing)."}
        },
        "required": ["folder"],
        "additionalProperties": false
    })
}

/// The Substance Painter handoff: a folder with the low mesh to texture (OBJ with UVs, at the lowest subdivision level
/// when the object has levels), the high mesh to bake detail from (its highest level, as OBJ), any baked maps under
/// Painter's names, and a README saying how to open it. Read-only for the scene.
pub(crate) fn substance_send(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let folder = args.get("folder").and_then(Value::as_str).filter(|f| !f.trim().is_empty()).ok_or_else(|| bad("folder is required"))?;
    let object = s.scene.get(id).expect("checked");
    let dir = std::path::Path::new(folder);
    std::fs::create_dir_all(dir).map_err(|e| CommandError::new("io_failed", format!("could not make {folder}: {e}")))?;
    let stem: String = object.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    // At a higher level the object's mesh is the detail to bake, and level 0 is what gets textured.
    let (low, high) = match (&object.multires, object.levels().0) {
        (Some(m), level) if level > 0 => (m.levels.first().map(|l| &l.mesh).unwrap_or(&object.mesh), &object.mesh),
        _ => (&object.mesh, &object.mesh),
    };
    if low.uvs.len() != low.indices.len() {
        return Err(CommandError::new("no_uvs", "the mesh to texture has no UVs; run uv_unwrap first (at level 0 when the object has levels)"));
    }
    let mut written = Vec::new();
    let mut write = |name: String, bytes: &[u8]| -> Result<(), CommandError> {
        let path = dir.join(&name);
        std::fs::write(&path, bytes).map_err(|e| CommandError::new("io_failed", format!("could not write {}: {e}", path.display())))?;
        written.push(path.display().to_string());
        Ok(())
    };
    write(format!("{stem}_low.obj"), &fsc_io::export(fsc_io::Format::Obj, &[fsc_io::GltfObject { name: &stem, mesh: low, targets: Vec::new(), maps: None }]).map_err(|e| CommandError::new("io_failed", e))?)?;
    let has_high = !std::ptr::eq(low, high);
    if has_high {
        write(format!("{stem}_high.obj"), &fsc_io::export(fsc_io::Format::Obj, &[fsc_io::GltfObject { name: &stem, mesh: high, targets: Vec::new(), maps: None }]).map_err(|e| CommandError::new("io_failed", e))?)?;
    }
    let mut maps_written = false;
    if let Some(files) = crate::live::map_files(object) {
        maps_written = true;
        for (name, bytes) in files {
            // Painter's own map names: BaseColor, Normal (OpenGL), Height, Emissive.
            let renamed = name.replace("_color.png", "_BaseColor.png").replace("_normal.png", "_Normal.png").replace("_displacement.exr", "_Height.exr").replace("_emissive.png", "_Emissive.png").replace("_roughness_metal.png", "_RoughnessMetal.png");
            write(renamed, &bytes)?;
        }
    }
    let readme = format!(
        "{stem} for Substance 3D Painter, from formsculptor {}\n\nFile > New: choose {stem}_low.obj as the mesh, Normal map format OpenGL, and add the maps below if any.\n{}Texture Set Settings > Bake Mesh Maps: High Definition Meshes {}.\nExport textures back as PNG, then paint_project or bake again in formsculptor as needed.\n",
        fsc_core::engine_version(),
        if maps_written { "Baked maps from formsculptor are beside it (BaseColor, Normal, Height, RoughnessMetal).\n" } else { "" },
        if has_high { format!("= {stem}_high.obj") } else { "= the same file (the object has no subdivision levels)".to_string() }
    );
    write(format!("{stem}_README.txt"), readme.as_bytes())?;
    Ok(json!({"folder": folder, "files": written, "low_triangles": low.triangle_count(), "high_triangles": high.triangle_count()}))
}
