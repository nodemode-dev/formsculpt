use fsc_core::{Mesh, ObjectId};
use fsc_sculpt::VertexEdit;
use fsc_topo::PolyHit;
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::{push_vertices, target};
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn vec3_of(v: &Value, what: &str) -> Result<Vec3, CommandError> {
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| bad(format!("{what} is [x, y, z]")))?;
    let n = |i: usize| a[i].as_f64().filter(|x| x.is_finite()).map(|x| x as f32).ok_or_else(|| bad(format!("{what} is [x, y, z] of numbers")));
    Ok(Vec3::new(n(0)?, n(1)?, n(2)?))
}

fn num(args: &Value, key: &str, default: f32, lo: f32, hi: f32) -> Result<f32, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v.as_f64().map(|x| x as f32).filter(|x| x.is_finite() && (lo..=hi).contains(x)).ok_or_else(|| bad(format!("{key} must be a number from {lo} to {hi}"))),
    }
}

fn flag(args: &Value, key: &str) -> Result<bool, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(bad(format!("{key} is true or false"))),
    }
}

/// Swaps in a new crease list as one undo step; the mesh, its UVs and textures are otherwise untouched.
fn crease_step(s: &mut Session, id: ObjectId, label: &str, creases: Vec<(u32, u32)>) -> Value {
    let object = s.scene.get_mut(id).expect("checked");
    let old_mesh = object.mesh.clone();
    let old_mask = object.mask.clone();
    object.mesh.creases = creases;
    object.revision += 1;
    let after = summary(object);
    s.history.push(Edit::Topology { label: label.into(), object: id, mesh: old_mesh, mask: old_mask });
    after
}

/// Moves vertices to new positions as one vertex edit (normals and BVH refreshed, UVs kept, an active sculpt layer
/// records it); nothing is pushed when nothing moves.
fn move_vertices(s: &mut Session, id: ObjectId, label: &str, moves: &[(u32, Vec3)]) -> Value {
    let object = s.scene.get_mut(id).expect("checked");
    let mut edit = VertexEdit::default();
    for &(v, q) in moves {
        edit.indices.push(v);
        edit.before.push(object.mesh.positions[v as usize]);
        edit.after.push(q.to_array());
        edit.mask_before.push(object.mask[v as usize]);
        edit.mask_after.push(object.mask[v as usize]);
    }
    if edit.indices.is_empty() {
        return summary(object);
    }
    edit.apply(object, true);
    let after = summary(object);
    push_vertices(s, id, label.into(), edit);
    after
}

pub(crate) fn creases_schema() -> Value {
    json!({"type": "object", "properties": {
        "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
        "eye": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A camera position; each crease then says whether the object hides its middle from there."}
    }, "required": [], "additionalProperties": false})
}

/// The creased edges as segments, for drawing them: each with its two end points and, given an eye, whether it is in
/// view. Read-only.
pub(crate) fn get_creases(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let eye = match args.get("eye").filter(|v| !v.is_null()) {
        Some(v) => Some(vec3_of(v, "eye")?),
        None => None,
    };
    let object = s.scene.get(id).expect("checked");
    let m = &object.mesh;
    let edges: std::collections::HashSet<(u32, u32)> = (0..m.triangle_count()).flat_map(|t| {
        let [a, b, c] = m.triangle(t);
        [(a.min(b), a.max(b)), (b.min(c), b.max(c)), (c.min(a), c.max(a))]
    }).collect();
    let size = m.bounds().extent().length().max(1e-6);
    let segments: Vec<Value> = m.creases.iter().filter(|e| edges.contains(e)).map(|&(a, b)| {
        let (pa, pb) = (m.position(a), m.position(b));
        let visible = eye.map(|e| {
            let mid = (pa + pb) * 0.5;
            let d = mid.distance(e);
            match object.bvh.raycast(m, e, (mid - e) / d.max(1e-9)) {
                Some(h) => h.t >= d - size * 2e-3,
                None => true,
            }
        });
        json!({"a": pa.to_array(), "b": pb.to_array(), "visible": visible})
    }).collect();
    Ok(json!({"id": id, "revision": object.revision, "creases": segments}))
}

fn aim_props() -> serde_json::Map<String, Value> {
    json!({
        "id": {"type": "integer", "minimum": 1, "description": "A quad mesh (quad_remesh, a retopology object or a quad import); the active object when omitted."},
        "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "A point on or near the surface, in object space (from pick); the face under it, its nearest edge and nearest corner are the targets."},
        "ray": {"type": "object", "properties": {"origin": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}, "dir": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}}, "required": ["origin", "dir"], "additionalProperties": false, "description": "A ray in object space instead of point; the first face it hits."}
    })
    .as_object()
    .cloned()
    .expect("object")
}

/// The vertex, edge and face aimed at, or None when a ray misses.
fn aim(s: &Session, id: ObjectId, args: &Value) -> Result<Option<(PolyHit, Vec3)>, CommandError> {
    let object = s.scene.get(id).expect("checked");
    let m = &object.mesh;
    let (triangle, point) = match (args.get("point").filter(|v| !v.is_null()), args.get("ray").filter(|v| !v.is_null())) {
        (Some(p), None) => {
            let p = vec3_of(p, "point")?;
            let reach = m.bounds().extent().length() + p.distance(m.bounds().center());
            let (q, t, _, _) = fsc_topo::nearest_feature(&object.bvh, m, p, reach).ok_or_else(|| CommandError::new("no_hit", "the object has no surface"))?;
            (t as usize, q)
        }
        (None, Some(r)) => {
            let origin = vec3_of(&r["origin"], "ray.origin")?;
            let dir = vec3_of(&r["dir"], "ray.dir")?.try_normalize().ok_or_else(|| bad("ray.dir must not be zero"))?;
            match object.bvh.raycast(m, origin, dir) {
                Some(h) => (h.triangle as usize, h.point),
                None => return Ok(None),
            }
        }
        _ => return Err(bad("give point or ray")),
    };
    let hit = fsc_topo::poly_hit(m, triangle, point).map_err(|e| CommandError::new("not_quads", e))?;
    Ok(Some((hit, point)))
}

fn hit_json(m: &Mesh, hit: &PolyHit) -> Value {
    let p = |v: u32| m.position(v).to_array();
    let e = (hit.edge.0.min(hit.edge.1), hit.edge.0.max(hit.edge.1));
    json!({
        "face": hit.quad, "face_corners": hit.corners.map(p), "edge": [hit.edge.0, hit.edge.1], "edge_points": [p(hit.edge.0), p(hit.edge.1)],
        "border": hit.border, "creased": m.creases.contains(&e), "vertex": hit.vertex, "vertex_point": p(hit.vertex), "creases": m.creases.len()
    })
}

pub(crate) fn pick_schema() -> Value {
    json!({"type": "object", "properties": aim_props(), "required": [], "additionalProperties": false})
}

/// The face, edge and vertex under a point or ray on a quad mesh, for hover highlights and for aiming ZModeler actions.
pub(crate) fn poly_pick(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    match aim(s, id, args)? {
        None => Ok(json!({"id": id, "hit": false})),
        Some((hit, _)) => {
            let mut out = hit_json(&s.scene.get(id).expect("checked").mesh, &hit);
            out["id"] = json!(id);
            out["hit"] = json!(true);
            Ok(out)
        }
    }
}

fn replace(s: &mut Session, id: ObjectId, label: &str, mesh: Mesh) -> Value {
    let object = s.scene.get_mut(id).expect("checked");
    let mask = vec![0.0; mesh.vertex_count()];
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    object.clear_maps();
    let after = summary(object);
    s.history.push(Edit::Topology { label: label.into(), object: id, mesh: old_mesh, mask: old_mask });
    after
}

fn aimed(s: &mut Session, args: &Value, what: &str) -> Result<(ObjectId, PolyHit, Vec3), CommandError> {
    let id = target(s, args)?;
    crate::multires_cmds::guard(s, id, what)?;
    let (hit, point) = aim(s, id, args)?.ok_or_else(|| CommandError::new("no_hit", "the ray misses the object"))?;
    Ok((id, hit, point))
}

pub(crate) fn edge_schema() -> Value {
    let mut p = aim_props();
    p.insert("action".into(), json!({"type": "string", "enum": ["insert", "extrude", "close", "crease", "uncrease", "align", "slide"], "description": "insert: an edge loop across the ring of quads through the edge, at fraction at; extrude: an open-border edge pulled out in its face's plane by distance as a new quad; close: the hole the border edge belongs to filled with quads (an even number of sides); crease and uncrease: mark the edge to stay sharp under subdivision, or clear it; align: the edge loop through the edge straightened (open loop, onto the line between its ends) or flattened (closed loop, onto its best-fit plane); slide: that edge loop moved toward the side of the aimed face by amount."}));
    p.insert("amount".into(), json!({"type": "number", "minimum": 0, "maximum": 0.95, "default": 0.25, "description": "slide: how far each loop vertex moves toward its neighbour on the aimed face's side, as a share of that edge."}));
    p.insert("crease".into(), json!({"type": "boolean", "default": false, "description": "insert: crease the new loop's edges (auto-crease)."}));
    p.insert("at".into(), json!({"type": "number", "minimum": 0.02, "maximum": 0.98, "default": 0.5, "description": "insert: where the loop crosses each quad, from one side of the ring to the other."}));
    p.insert("distance".into(), json!({"type": "number", "minimum": -1000, "maximum": 1000, "description": "extrude: how far, in world units; the edge's own length when omitted."}));
    json!({"type": "object", "properties": p, "required": ["action"], "additionalProperties": false})
}

/// ZModeler edge actions on the edge nearest the aimed point. One undo step.
pub(crate) fn zmodel_edge(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let action = args.get("action").and_then(Value::as_str).ok_or_else(|| bad("action is insert, extrude, close, crease or uncrease"))?;
    let (id, hit, point) = aimed(s, args, "A ZModeler edge action")?;
    let mesh = &s.scene.get(id).expect("checked").mesh;
    if action == "crease" || action == "uncrease" {
        let (list, changed) = fsc_topo::set_creases(mesh, &hit, false, action == "crease").map_err(|e| CommandError::new("not_quads", e))?;
        let after = crease_step(s, id, if action == "crease" { "Crease Edge" } else { "Uncrease Edge" }, list);
        return Ok(json!({"object": after, "action": action, "edge": [hit.edge.0, hit.edge.1], "edges_changed": changed}));
    }
    if action == "align" || action == "slide" {
        let amount = num(args, "amount", 0.25, 0.0, 0.95)?;
        let moves = fsc_topo::loop_moves(mesh, &hit, action == "align", amount).map_err(|e| CommandError::new("not_quads", e))?;
        let n = moves.len();
        let after = move_vertices(s, id, if action == "align" { "Align Edge Loop" } else { "Slide Edge Loop" }, &moves);
        return Ok(json!({"object": after, "action": action, "edge": [hit.edge.0, hit.edge.1], "vertices_moved": n}));
    }
    let (out, extra) = match action {
        "insert" => {
            let at = num(args, "at", 0.5, 0.02, 0.98)?;
            let (m, n) = fsc_topo::insert_loop_at(mesh, point, at, flag(args, "crease")?).map_err(|e| CommandError::new("not_quads", e))?;
            (m, json!({"quads_split": n}))
        }
        "extrude" => {
            let len = mesh.position(hit.edge.0).distance(mesh.position(hit.edge.1));
            let d = num(args, "distance", len, -1000.0, 1000.0)?;
            if d.abs() < 1e-7 {
                return Err(bad("distance must not be zero"));
            }
            (fsc_topo::extrude_edge(mesh, &hit, d).map_err(|e| CommandError::new("not_border", e))?, json!({"quads_added": 1}))
        }
        "close" => {
            let (m, n) = fsc_topo::close_border(mesh, &hit).map_err(|e| CommandError::new("cannot_close", e))?;
            (m, json!({"quads_added": n}))
        }
        other => return Err(bad(format!("unknown action {other}; use insert, extrude, close, crease or uncrease"))),
    };
    let label = match action {
        "insert" => "Insert Edge Loop",
        "extrude" => "Extrude Edge",
        _ => "Close Hole",
    };
    let after = replace(s, id, label, out);
    let mut r = json!({"object": after, "action": action, "edge": [hit.edge.0, hit.edge.1]});
    for (k, v) in extra.as_object().expect("object") {
        r[k] = v.clone();
    }
    Ok(r)
}

pub(crate) fn face_schema() -> Value {
    let mut p = aim_props();
    p.insert("action".into(), json!({"type": "string", "enum": ["inset", "extrude", "delete"], "description": "inset: a smaller face inside the face with a ring of quads around it; extrude: the face lifted along its normal with walls; delete: the face removed, opening the mesh."}));
    p.insert("inset".into(), json!({"type": "number", "minimum": 0, "maximum": 0.95, "description": "How far the inner face shrinks toward the centre (0.25 for inset, 0 for extrude when omitted)."}));
    p.insert("height".into(), json!({"type": "number", "minimum": -1000, "maximum": 1000, "description": "How far the inner face moves along the face normal in world units (0 for inset; the face's mean side length for extrude when omitted); negative pushes in."}));
    p.insert("crease".into(), json!({"type": "boolean", "default": false, "description": "inset and extrude: crease the inner face's edges and the original outline (auto-crease)."}));
    json!({"type": "object", "properties": p, "required": ["action"], "additionalProperties": false})
}

/// ZModeler face actions on the face under the aimed point. One undo step.
pub(crate) fn zmodel_face(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let action = args.get("action").and_then(Value::as_str).ok_or_else(|| bad("action is inset, extrude or delete"))?;
    let (id, hit, _) = aimed(s, args, "A ZModeler face action")?;
    let mesh = &s.scene.get(id).expect("checked").mesh;
    let side = (0..4).map(|i| mesh.position(hit.corners[i]).distance(mesh.position(hit.corners[(i + 1) % 4]))).sum::<f32>() / 4.0;
    let (out, label) = match action {
        "inset" | "extrude" => {
            let (di, dh) = if action == "inset" { (0.25, 0.0) } else { (0.0, side) };
            let inset = num(args, "inset", di, 0.0, 0.95)?;
            let height = num(args, "height", dh, -1000.0, 1000.0)?;
            if inset < 1e-6 && height.abs() < 1e-7 {
                return Err(bad("give a nonzero inset or height"));
            }
            (fsc_topo::inset_face(mesh, &hit, inset, height, flag(args, "crease")?).map_err(|e| CommandError::new("not_quads", e))?, if action == "inset" { "Inset Face" } else { "Extrude Face" })
        }
        "delete" => (fsc_topo::delete_face(mesh, &hit).map_err(|e| CommandError::new("cannot_delete", e))?, "Delete Face"),
        other => return Err(bad(format!("unknown action {other}; use inset, extrude or delete"))),
    };
    let after = replace(s, id, label, out);
    Ok(json!({"object": after, "action": action, "face": hit.quad}))
}

pub(crate) fn point_schema() -> Value {
    let mut p = aim_props();
    p.insert("action".into(), json!({"type": "string", "enum": ["slide", "crease", "uncrease"], "description": "slide: the nearest corner moved toward `to`, kept in the plane of its normal so the surface around it does not lift; crease and uncrease: every quad edge meeting the corner kept sharp under subdivision, or cleared."}));
    p.insert("to".into(), json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "slide: where the corner goes, in object space (projected onto its tangent plane)."}));
    json!({"type": "object", "properties": p, "required": ["action"], "additionalProperties": false})
}

/// ZModeler point actions on the corner nearest the aimed point. One undo step.
pub(crate) fn zmodel_point(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let action = args.get("action").and_then(Value::as_str).ok_or_else(|| bad("action is slide, crease or uncrease"))?;
    if action == "crease" || action == "uncrease" {
        let (id, hit, _) = aimed(s, args, "A ZModeler point action")?;
        let (list, changed) = fsc_topo::set_creases(&s.scene.get(id).expect("checked").mesh, &hit, true, action == "crease").map_err(|e| CommandError::new("not_quads", e))?;
        let after = crease_step(s, id, if action == "crease" { "Crease Point" } else { "Uncrease Point" }, list);
        return Ok(json!({"object": after, "action": action, "vertex": hit.vertex, "edges_changed": changed}));
    }
    if action != "slide" {
        return Err(bad(format!("unknown action {action}; use slide, crease or uncrease")));
    }
    let to = vec3_of(args.get("to").ok_or_else(|| bad("to is required"))?, "to")?;
    let (id, hit, _) = aimed(s, args, "A ZModeler point action")?;
    let object = s.scene.get_mut(id).expect("checked");
    let v = hit.vertex;
    let p = object.mesh.position(v);
    let n = Vec3::from(object.mesh.normals[v as usize]);
    let moved = to - n * (to - p).dot(n);
    let mut edit = VertexEdit::default();
    edit.indices.push(v);
    edit.before.push(p.to_array());
    edit.after.push(moved.to_array());
    edit.mask_before.push(object.mask[v as usize]);
    edit.mask_after.push(object.mask[v as usize]);
    edit.apply(object, true);
    let after = summary(object);
    push_vertices(s, id, "Slide Point".into(), edit);
    Ok(json!({"object": after, "action": action, "vertex": v, "point": moved.to_array()}))
}
