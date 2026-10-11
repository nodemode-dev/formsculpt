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
    if moved > 0 {
        s.history.push(Edit::Vertices { label: label.into(), object: id, edit });
    }
    json!({"object": after, "vertices_moved": moved})
}

pub(crate) fn transform_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "translate": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "World offset."},
            "rotate": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Degrees about x, y and z (applied in that order) around the object's bounds centre."},
            "scale": {"type": ["number", "array"], "items": {"type": "number"}, "description": "A factor or [x, y, z] factors around the bounds centre."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn transform(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let d = Deformer::Transform { translate: vec3(args, "translate", Vec3::ZERO)?, rotate_deg: vec3(args, "rotate", Vec3::ZERO)?, scale: vec3(args, "scale", Vec3::ONE)? };
    let object = s.scene.get(id).expect("checked");
    let positions = fsc_topo::deform(&object.mesh, &object.mask, d);
    Ok(set_positions(s, id, "Transform", positions))
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

pub(crate) fn boolean_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "a": {"type": "integer", "minimum": 1, "description": "The object to keep or cut from."},
            "b": {"type": "integer", "minimum": 1, "description": "The object to add, cut away or intersect with."},
            "op": {"type": "string", "enum": ["union", "difference", "intersection"], "default": "union"},
            "resolution": {"type": "integer", "minimum": 16, "maximum": 320, "default": 160, "description": "Voxels along the longest side of both objects together; higher keeps sharper detail."},
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
    let (oa, ob): (&Object, &Object) = (s.scene.get(a).expect("checked"), s.scene.get(b).expect("checked"));
    let mut mesh = fsc_topo::voxel_boolean(&oa.mesh, &oa.bvh, &ob.mesh, &ob.bvh, op, resolution as u32).map_err(|e| CommandError::new("boolean_failed", e))?;
    if oa.mesh.has_paint() || ob.mesh.has_paint() {
        fsc_topo::transfer_paint(&[(&oa.mesh, &oa.bvh), (&ob.mesh, &ob.bvh)], &mut mesh);
    }
    let label = match op {
        BooleanOp::Union => "Union",
        BooleanOp::Difference => "Difference",
        BooleanOp::Intersection => "Intersection",
    };
    let name = s.scene.unique_name(&format!("{} {}", oa.name, label.to_lowercase()));
    snapshot(s, label);
    let at = s.scene.index_of(a).expect("checked");
    if !keep {
        s.scene.remove(a);
        s.scene.remove(b);
    }
    let new_id = s.scene.add(name, mesh);
    let o = s.scene.remove(new_id).expect("added");
    s.scene.insert(at.min(s.scene.objects.len()), o);
    Ok(summary(s.scene.get(new_id).expect("added")))
}
