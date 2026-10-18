use fsc_core::{Mesh, Object, ObjectId};
use fsc_sculpt::VertexEdit;
use fsc_topo::{BooleanOp, Deformer};
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::multires_cmds::guard;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn vec3(args: &Value, key: &str, default: Vec3) -> Result<Vec3, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(n)) => Ok(Vec3::splat(n.as_f64().unwrap_or(1.0) as f32)),
        Some(Value::Array(a)) if a.len() == 3 => {
            let c = |i: usize| a[i].as_f64().map(|x| x as f32).ok_or_else(|| bad(format!("{key} needs three numbers")));
            Ok(Vec3::new(c(0)?, c(1)?, c(2)?))
        }
        _ => Err(bad(format!("{key} must be [x, y, z]"))),
    }
}

fn axis(args: &Value, key: &str, default: usize) -> Result<usize, CommandError> {
    match args.get(key).and_then(Value::as_str) {
        None => Ok(default),
        Some("x") => Ok(0),
        Some("y") => Ok(1),
        Some("z") => Ok(2),
        Some(other) => Err(bad(format!("{key} must be x, y or z, not {other}"))),
    }
}

fn snapshot(s: &mut Session, label: &str) {
    s.history.push(Edit::Scene { label: label.into(), objects: s.scene.objects.clone(), active: s.scene.active });
}

fn ids_of(s: &Session, args: &Value, key: &str) -> Result<ObjectId, CommandError> {
    let id = args.get(key).and_then(Value::as_u64).ok_or_else(|| bad(format!("{key} is required")))? as ObjectId;
    if s.scene.get(id).is_none() {
        return Err(CommandError::new("not_found", format!("no object {id}; get_scene lists object ids")));
    }
    Ok(id)
}

pub(crate) fn set_positions(s: &mut Session, id: ObjectId, label: &str, positions: Vec<[f32; 3]>) -> Value {
    let (reply, edit) = move_positions(s, id, label, positions);
    if let Some(edit) = edit {
        s.history.push(edit);
    }
    reply
}

/// Writes new positions and returns the reply with the undo step (None when nothing moved), for callers that bundle
/// several objects into one step.
pub(crate) fn move_positions(s: &mut Session, id: ObjectId, label: &str, positions: Vec<[f32; 3]>) -> (Value, Option<Edit>) {
    let object = s.scene.get_mut(id).expect("checked");
    let mut edit = VertexEdit::default();
    for (v, p) in positions.iter().enumerate() {
        if object.mesh.positions[v] != *p {
            edit.indices.push(v as u32);
            edit.before.push(object.mesh.positions[v]);
            edit.after.push(*p);
            edit.mask_before.push(object.mask[v]);
            edit.mask_after.push(object.mask[v]);
        }
    }
    object.mesh.positions = positions;
    object.mesh.recompute_normals();
    object.bvh.refit(&object.mesh);
    object.revision += 1;
    let after = summary(object);
    let moved = edit.indices.len();
    let step = (moved > 0).then(|| Edit::Vertices { label: label.into(), object: id, edit });
    (json!({"object": after, "vertices_moved": moved}), step)
}

pub(crate) fn transform_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object or group id; the active object when omitted. Everything under it in the hierarchy moves with it, whole; a group turns and scales about the centre of what is under it."},
            "translate": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "World offset."},
            "rotate": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Degrees about x, y and z (applied in that order) around the object's bounds centre."},
            "scale": {"type": ["number", "array"], "items": {"type": "number"}, "description": "A factor or [x, y, z] factors around the bounds centre."},
            "pivot": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Turn and scale about this world point instead of the bounds centre (the transform gizmo's picked pivot or the world origin)."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn transform(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let (translate, rotate_deg, scale) = (vec3(args, "translate", Vec3::ZERO)?, vec3(args, "rotate", Vec3::ZERO)?, vec3(args, "scale", Vec3::ONE)?);
    // A group id moves everything under the group; an object moves with everything under it in the hierarchy.
    let group = args.get("id").and_then(Value::as_u64).map(|i| i as ObjectId).filter(|i| s.scene.group(*i).is_some());
    let id = match group {
        Some(g) => g,
        None => target(s, args)?,
    };
    let movers: Vec<ObjectId> = if group.is_some() { s.scene.descendants(id) } else { std::iter::once(id).chain(s.scene.descendants(id)).collect() };
    if movers.is_empty() {
        return Err(CommandError::new("empty_group", format!("group {id} has no objects under it to move")));
    }
    // Every part turns and scales about one point: the pivot given, else the object's bounds centre, or for a group
    // the centre of everything under it.
    let centre = match args.get("pivot").filter(|v| !v.is_null()) {
        Some(_) => vec3(args, "pivot", Vec3::ZERO)?,
        None => {
            let mut b = fsc_core::Aabb::EMPTY;
            for &m in if group.is_some() { &movers[..] } else { &movers[..1] } {
                let mb = s.scene.get(m).expect("listed").mesh.bounds();
                b.grow(mb.min);
                b.grow(mb.max);
            }
            b.center()
        }
    };
    let r = glam::Quat::from_euler(glam::EulerRot::XYZ, rotate_deg.x.to_radians(), rotate_deg.y.to_radians(), rotate_deg.z.to_radians());
    let mut edits = Vec::new();
    let (mut moved, mut reply) = (0u64, Value::Null);
    for &m in &movers {
        let object = s.scene.get(m).expect("listed");
        // The deformer turns and scales about the bounds centre C; about a pivot P instead is the same change plus a
        // move: P + t + RS(p - P) = C + t' + RS(p - C) with t' = t + (P - C) - RS(P - C).
        let c = object.mesh.bounds().center();
        let d = Deformer::Transform { translate: translate + (centre - c) - r * ((centre - c) * scale), rotate_deg, scale };
        // The object asked for keeps its mask (topology masking); what hangs under it moves whole, and so does a
        // triplanar, whose mask is the shape it describes rather than a part to hold still.
        let triplanar = object.primitive.as_ref().is_some_and(|(a, _)| a.contains("\"triplanar\"")) && fsc_core::triplanar::divisions(&object.mesh).is_some();
        let positions = if m == id && !triplanar { fsc_topo::deform(&object.mesh, &object.mask, d) } else { fsc_topo::deform(&object.mesh, &vec![0.0; object.mask.len()], d) };
        let (r, edit) = move_positions(s, m, "Transform", positions);
        moved += r["vertices_moved"].as_u64().unwrap_or(0);
        if m == id || reply.is_null() {
            reply = r;
        }
        edits.extend(edit);
    }
    match edits.len() {
        0 => {}
        1 => s.history.push(edits.pop().expect("one")),
        _ => s.history.push(Edit::Many { label: "Transform".into(), edits }),
    }
    reply["vertices_moved"] = json!(moved);
    reply["objects_moved"] = json!(movers.len());
    if group.is_some() {
        reply["group"] = json!(id);
    }
    Ok(reply)
}

pub(crate) fn lattice_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "divisions": {"type": "array", "items": {"type": "integer", "minimum": 2, "maximum": 6}, "minItems": 3, "maxItems": 3, "default": [3, 3, 3], "description": "Control points along x, y and z. Point [i, j, k] starts at min + (i / (nx - 1), j / (ny - 1), k / (nz - 1)) * size of the object's bounds grown by 2 percent of their largest side."},
            "moves": {"type": "array", "minItems": 1, "maxItems": 216, "items": {"type": "object", "properties": {
                "point": {"type": "array", "items": {"type": "integer", "minimum": 0}, "minItems": 3, "maxItems": 3, "description": "[i, j, k] of the control point."},
                "offset": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "How far it moves, in world units."}
            }, "required": ["point", "offset"], "additionalProperties": false}}
        },
        "required": ["moves"],
        "additionalProperties": false
    })
}

/// Lattice deformation: moves control points of a lattice around the object and lets the surface follow smoothly.
pub(crate) fn lattice(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let div: [usize; 3] = match args.get("divisions") {
        None | Some(Value::Null) => [3, 3, 3],
        Some(Value::Array(a)) if a.len() == 3 => {
            let mut d = [0usize; 3];
            for (k, v) in a.iter().enumerate() {
                let n = v.as_u64().ok_or_else(|| bad("divisions are three whole numbers"))? as usize;
                if !(2..=6).contains(&n) {
                    return Err(bad("each division must be from 2 to 6"));
                }
                d[k] = n;
            }
            d
        }
        Some(_) => return Err(bad("divisions is [nx, ny, nz]")),
    };
    let mut offsets = vec![Vec3::ZERO; div[0] * div[1] * div[2]];
    let moves = args.get("moves").and_then(Value::as_array).filter(|m| !m.is_empty()).ok_or_else(|| bad("give moves: [{point: [i, j, k], offset: [dx, dy, dz]}]"))?;
    for m in moves {
        let p: Vec<usize> = m.get("point").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).map(|x| x as usize).collect()).unwrap_or_default();
        if p.len() != 3 || p.iter().zip(div).any(|(i, n)| *i >= n) {
            return Err(bad(format!("each point is [i, j, k] below the divisions {div:?}")));
        }
        let o: Vec<f32> = m.get("offset").and_then(Value::as_array).map(|a| a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect()).unwrap_or_default();
        if o.len() != 3 || o.iter().any(|x| !x.is_finite()) {
            return Err(bad("each offset is [dx, dy, dz]"));
        }
        offsets[p[0] + p[1] * div[0] + p[2] * div[0] * div[1]] += Vec3::new(o[0], o[1], o[2]);
    }
    let object = s.scene.get(id).expect("checked");
    let (lo, hi) = fsc_topo::lattice_box(&object.mesh);
    let positions = fsc_topo::lattice_deform(&object.mesh, &object.mask, div, &offsets);
    let mut reply = set_positions(s, id, "Lattice", positions);
    reply["lattice"] = json!({"min": lo.to_array(), "max": hi.to_array(), "divisions": div});
    Ok(reply)
}

pub(crate) fn deform_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "kind": {"type": "string", "enum": ["bend", "twist", "taper"]},
            "axis": {"type": "string", "enum": ["x", "y", "z"], "default": "y", "description": "The axis the deformation runs along, over the object's bounds."},
            "toward": {"type": "string", "enum": ["x", "y", "z"], "default": "x", "description": "Bend only: the direction the axis curves toward."},
            "angle": {"type": "number", "minimum": -720, "maximum": 720, "description": "Bend and twist: total degrees across the object's length."},
            "amount": {"type": "number", "minimum": -2, "maximum": 2, "description": "Taper: change in width from one end to the other (0.5 is half again wider at the positive end)."}
        },
        "required": ["kind"],
        "additionalProperties": false
    })
}

pub(crate) fn deform(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let a = axis(args, "axis", 1)?;
    let angle = args.get("angle").and_then(Value::as_f64).unwrap_or(45.0) as f32;
    let d = match args.get("kind").and_then(Value::as_str) {
        Some("bend") => {
            let toward = axis(args, "toward", if a == 0 { 1 } else { 0 })?;
            if toward == a {
                return Err(bad("toward must differ from axis"));
            }
            Deformer::Bend { axis: a, toward, angle: angle.to_radians() }
        }
        Some("twist") => Deformer::Twist { axis: a, angle: angle.to_radians() },
        Some("taper") => Deformer::Taper { axis: a, amount: args.get("amount").and_then(Value::as_f64).unwrap_or(0.5) as f32 },
        _ => return Err(bad("kind must be bend, twist or taper")),
    };
    let object = s.scene.get(id).expect("checked");
    let positions = fsc_topo::deform(&object.mesh, &object.mask, d);
    let label = match d {
        Deformer::Bend { .. } => "Bend",
        Deformer::Twist { .. } => "Twist",
        _ => "Taper",
    };
    Ok(set_positions(s, id, label, positions))
}

pub(crate) fn duplicate_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "offset": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Move the copy by this much; default none."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn duplicate(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let offset = vec3(args, "offset", Vec3::ZERO)?;
    let source = s.scene.get(id).expect("checked");
    let name = s.scene.unique_name(&source.name);
    let mut mesh = source.mesh.clone();
    for p in mesh.positions.iter_mut() {
        *p = (Vec3::from_array(*p) + offset).to_array();
    }
    let mask = source.mask.clone();
    let new_id = s.scene.add(name, mesh);
    let object = s.scene.get_mut(new_id).expect("added");
    object.mask = mask;
    let index = s.scene.index_of(new_id).expect("added");
    s.history.push(Edit::Add { id: new_id, index, parked: None });
    Ok(summary(s.scene.get(new_id).expect("added")))
}

pub(crate) fn rename_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "name": {"type": "string", "minLength": 1, "maxLength": 120}
        },
        "required": ["name"],
        "additionalProperties": false
    })
}

pub(crate) fn rename(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let name = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad("name is required"))?;
    let object = s.scene.get_mut(id).expect("checked");
    let old = std::mem::replace(&mut object.name, name.to_string());
    object.revision += 1;
    s.history.push(Edit::Rename { object: id, name: old });
    Ok(summary(s.scene.get(id).expect("checked")))
}

pub(crate) fn merge_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"ids": {"type": "array", "items": {"type": "integer", "minimum": 1}, "minItems": 2, "description": "Objects to join into the first one, which keeps its name."}},
        "required": ["ids"],
        "additionalProperties": false
    })
}

pub(crate) fn merge(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let ids: Vec<ObjectId> = args.get("ids").and_then(Value::as_array).ok_or_else(|| bad("ids is required"))?.iter().filter_map(|v| v.as_u64().map(|x| x as ObjectId)).collect();
    if ids.len() < 2 {
        return Err(bad("give at least two object ids"));
    }
    for &id in &ids {
        if s.scene.get(id).is_none() {
            return Err(CommandError::new("not_found", format!("no object {id}")));
        }
        guard(s, id, "Merge")?;
    }
    let mesh = {
        let meshes: Vec<&Mesh> = ids.iter().map(|&id| &s.scene.get(id).expect("checked").mesh).collect();
        let mut merged = fsc_topo::merge(&meshes);
        if meshes.iter().any(|m| m.has_paint()) {
            merged.paint = meshes.iter().flat_map(|m| if m.has_paint() { m.paint.clone() } else { vec![fsc_core::DEFAULT_PAINT; m.vertex_count()] }).collect();
        }
        merged
    };
    let mask: Vec<f32> = ids.iter().flat_map(|&id| s.scene.get(id).expect("checked").mask.clone()).collect();
    snapshot(s, "Merge");
    for &id in &ids[1..] {
        s.scene.remove(id);
    }
    let object = s.scene.get_mut(ids[0]).expect("checked");
    object.replace_mesh(mesh, mask);
    s.scene.active = Some(ids[0]);
    Ok(summary(s.scene.get(ids[0]).expect("checked")))
}

/// The mask_shape arguments without op, plus close.
pub(crate) fn split_shape_schema() -> Value {
    let mut schema = crate::mask_cmds::shape_schema();
    let props = schema["properties"].as_object_mut().expect("properties");
    props.remove("op");
    props.insert("close".into(), json!({"type": "boolean", "default": false, "description": "Fill the holes along the cut on both pieces; false leaves the cut edge open."}));
    schema
}

pub(crate) fn split(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Split")?;
    let object = s.scene.get(id).expect("checked");
    let mut parts = fsc_topo::split_parts(&object.mesh);
    if object.mesh.has_paint() {
        for part in parts.iter_mut() {
            fsc_topo::transfer_paint(&[(&object.mesh, &object.bvh)], part);
        }
    }
    if parts.len() < 2 {
        return Ok(json!({"id": id, "parts": 1}));
    }
    let base = object.name.clone();
    snapshot(s, "Split Parts");
    let at = s.scene.index_of(id).expect("checked");
    let removed = s.scene.remove(id).expect("checked");
    let mut ids = Vec::new();
    for (k, mesh) in parts.into_iter().enumerate() {
        let name = s.scene.unique_name(&format!("{base} {}", k + 1));
        let new_id = s.scene.add(name, mesh);
        let o = s.scene.remove(new_id).expect("added");
        s.scene.insert(at + k, o);
        ids.push(new_id);
    }
    drop(removed);
    Ok(json!({"parts": ids.len(), "ids": ids}))
}

fn faces_to_mesh(mesh: &Mesh, inside: &[bool], want: bool) -> (Mesh, Vec<u32>) {
    let mut local = vec![u32::MAX; mesh.vertex_count()];
    let (mut old, mut positions, mut indices, mut groups, mut hidden) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for t in (0..mesh.triangle_count()).filter(|&t| inside[t] == want) {
        for v in mesh.triangle(t) {
            if local[v as usize] == u32::MAX {
                local[v as usize] = positions.len() as u32;
                positions.push(mesh.positions[v as usize]);
                old.push(v);
            }
            indices.push(local[v as usize]);
        }
        groups.push(if mesh.has_groups() { mesh.groups[t] } else { 0 });
        hidden.push(mesh.face_hidden(t));
    }
    let mut out = Mesh::new(positions, indices);
    if mesh.has_paint() {
        out.paint = old.iter().map(|&v| mesh.paint[v as usize]).collect();
    }
    if mesh.uvs.len() == mesh.vertex_count() {
        out.uvs = old.iter().map(|&v| mesh.uvs[v as usize]).collect();
    }
    if mesh.has_groups() {
        out.groups = groups;
    }
    if hidden.iter().any(|&h| h) {
        out.hidden = hidden;
    }
    (out, old)
}

fn closed_piece(piece: Mesh, mask: &mut Vec<f32>) -> Mesh {
    let (mut filled, _) = fsc_topo::fill_holes(&piece);
    if piece.has_paint() {
        filled.paint = piece.paint.clone();
        filled.paint.resize(filled.vertex_count(), fsc_core::DEFAULT_PAINT);
    }
    if piece.uvs.len() == piece.vertex_count() {
        filled.uvs = piece.uvs.clone();
        filled.uvs.resize(filled.vertex_count(), [0.0; 2]);
    }
    if piece.has_groups() {
        let extra = filled.triangle_count() - piece.triangle_count();
        filled.groups = piece.groups.iter().copied().chain(std::iter::repeat_n(piece.groups.last().copied().unwrap_or(0), extra)).collect();
    }
    mask.resize(filled.vertex_count(), 0.0);
    filled
}

/// Moves the faces whose centre projects inside a screen shape into a new object, as one undo step; close fills the holes on both pieces.
pub(crate) fn split_shape(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let outline = crate::mask_cmds::Outline::from_args(args, "box")?;
    let close = match args.get("close") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(bad("close is true or false")),
    };
    let id = target(s, args)?;
    guard(s, id, "Split")?;
    let inside = crate::mask_cmds::faces_inside(s, id, args, &outline)?;
    let count = inside.iter().filter(|&&b| b).count();
    if count == 0 {
        return Err(CommandError::new("nothing_inside", "no face has its middle inside the shape; check the points and the view they refer to"));
    }
    if count == inside.len() {
        return Err(CommandError::new("everything_inside", "every face is inside the shape; nothing would be left behind"));
    }
    let object = s.scene.get(id).expect("checked");
    let (mut piece, taken) = faces_to_mesh(&object.mesh, &inside, true);
    let (mut rest, kept) = faces_to_mesh(&object.mesh, &inside, false);
    let mut piece_mask: Vec<f32> = taken.iter().map(|&v| object.mask[v as usize]).collect();
    let mut rest_mask: Vec<f32> = kept.iter().map(|&v| object.mask[v as usize]).collect();
    if close {
        piece = closed_piece(piece, &mut piece_mask);
        rest = closed_piece(rest, &mut rest_mask);
    }
    let name = s.scene.unique_name(&format!("{} split", object.name));
    snapshot(s, "Split Shape");
    let at = s.scene.index_of(id).expect("checked");
    s.scene.get_mut(id).expect("checked").replace_mesh(rest, rest_mask);
    let new_id = s.scene.add(name, piece);
    let mut o = s.scene.remove(new_id).expect("added");
    o.mask = piece_mask;
    s.scene.insert(at + 1, o);
    Ok(json!({"id": id, "new_id": new_id, "faces_moved": count, "faces_left": inside.len() - count, "closed": close}))
}

pub(crate) fn boolean_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "a": {"type": "integer", "minimum": 1, "description": "The object to keep or cut from."},
            "b": {"type": "integer", "minimum": 1, "description": "The object to add, cut away or intersect with."},
            "op": {"type": "string", "enum": ["union", "difference", "intersection"], "default": "union"},
            "method": {"type": "string", "enum": ["exact", "voxel"], "default": "exact", "description": "exact cuts the two surfaces along their true intersection, keeping every detail, sharp edges and flat faces (both must be closed; repair_mesh first if not); voxel rebuilds the result through a grid, which also mends open or self-crossing input and is what blend uses. An exact boolean on input that is not closed falls back to voxel and says so."},
            "resolution": {"type": "integer", "minimum": 16, "maximum": 320, "default": 160, "description": "Voxel method: voxels along the longest side of both objects together; higher keeps sharper detail."},
            "blend": {"type": "number", "minimum": 0, "maximum": 100, "default": 0, "description": "Fillets the seam over this many world units: a union fuses the two with a soft web where they meet (an arm into a shoulder, a horn into a head), a difference leaves a rounded cut edge. 0 is a sharp boolean."},
            "keep_inputs": {"type": "boolean", "default": false, "description": "Keep a and b instead of replacing them with the result."}
        },
        "required": ["a", "b"],
        "additionalProperties": false
    })
}

pub(crate) fn boolean(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let a = ids_of(s, args, "a")?;
    let b = ids_of(s, args, "b")?;
    if a == b {
        return Err(bad("a and b must be different objects"));
    }
    let op = match args.get("op").and_then(Value::as_str).unwrap_or("union") {
        "union" => BooleanOp::Union,
        "difference" => BooleanOp::Difference,
        "intersection" => BooleanOp::Intersection,
        other => return Err(bad(format!("op must be union, difference or intersection, not {other}"))),
    };
    let resolution = args.get("resolution").and_then(Value::as_u64).unwrap_or(160);
    if !(16..=320).contains(&resolution) {
        return Err(bad("resolution must be 16 to 320"));
    }
    let keep = args.get("keep_inputs").and_then(Value::as_bool).unwrap_or(false);
    let blend = match args.get("blend") {
        None | Some(Value::Null) => 0.0,
        Some(v) => v.as_f64().filter(|b| (0.0..=100.0).contains(b)).ok_or_else(|| bad("blend must be from 0 to 100"))? as f32,
    };
    let method = args.get("method").and_then(Value::as_str).unwrap_or("exact");
    if method != "exact" && method != "voxel" {
        return Err(bad(format!("method must be exact or voxel, not {method}")));
    }
    let (oa, ob): (&Object, &Object) = (s.scene.get(a).expect("checked"), s.scene.get(b).expect("checked"));
    // Exact unless a blend asks for the voxel fillet; input that is not closed falls back to voxel with the reason.
    let mut fallback: Option<String> = None;
    let exact = if method == "exact" && blend <= 0.0 {
        match fsc_topo::exact_boolean(&oa.mesh, &ob.mesh, op) {
            Ok(mesh) => Some(mesh),
            Err(e) if e.contains("closed manifold") => {
                fallback = Some(e);
                None
            }
            Err(e) => return Err(CommandError::new("boolean_failed", e)),
        }
    } else {
        None
    };
    let used = if exact.is_some() { "exact" } else { "voxel" };
    let mesh = match exact {
        Some(mesh) => mesh,
        None => {
            let mut mesh = fsc_topo::voxel_boolean_blend(&oa.mesh, &oa.bvh, &ob.mesh, &ob.bvh, op, blend, resolution as u32).map_err(|e| CommandError::new("boolean_failed", e))?;
            if oa.mesh.has_paint() || ob.mesh.has_paint() {
                fsc_topo::transfer_paint(&[(&oa.mesh, &oa.bvh), (&ob.mesh, &ob.bvh)], &mut mesh);
            }
            mesh
        }
    };
    let label = match op {
        BooleanOp::Union => "Union",
        BooleanOp::Difference => "Difference",
        BooleanOp::Intersection => "Intersection",
    };
    // Replacing the inputs, the result is a cut or joined: it keeps a's name as it keeps a's place in the list (a
    // head with nostrils cut is still the head). Kept inputs sit beside it, so the result takes a name of its own.
    let name = if keep { s.scene.unique_name(&format!("{} {}", oa.name, label.to_lowercase())) } else { oa.name.clone() };
    snapshot(s, label);
    let at = s.scene.index_of(a).expect("checked");
    if !keep {
        s.scene.remove(a);
        s.scene.remove(b);
    }
    let new_id = s.scene.add(name, mesh);
    let o = s.scene.remove(new_id).expect("added");
    s.scene.insert(at.min(s.scene.objects.len()), o);
    let mut out = summary(s.scene.get(new_id).expect("added"));
    out["method"] = json!(used);
    if let Some(why) = fallback {
        out["fallback"] = json!(why);
    }
    Ok(out)
}
