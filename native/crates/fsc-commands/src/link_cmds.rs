//! The live link: another app (the Blender add-on in `integrations/blender`) reads an object's mesh as raw arrays and
//! writes one back, over the same dispatch every client uses, with the object's revision telling either side when
//! the other has changed it. No file sits between them.

use base64::Engine as _;
use fsc_core::Mesh;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::multires_cmds::guard;
use crate::object_cmds::set_positions;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

/// The largest mesh either way: a sculpt at this size is already a few hundred megabytes of base64.
const VERTEX_LIMIT: usize = 8_000_000;

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

pub(crate) fn link_pull_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "since": {"type": "integer", "minimum": 0, "description": "The revision the caller already holds; when the object is still at it the reply carries no arrays and changed is false."}
        },
        "additionalProperties": false
    })
}

pub(crate) fn link_pull(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get(id).expect("checked");
    let mesh = &object.mesh;
    let mut reply = json!({
        "id": id, "name": object.name, "revision": object.revision,
        "vertices": mesh.vertex_count(), "triangles": mesh.triangle_count(), "changed": true
    });
    if args.get("since").and_then(Value::as_u64) == Some(object.revision) {
        reply["changed"] = json!(false);
        return Ok(reply);
    }
    let indices: Vec<u8> = mesh.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
    reply["positions"] = json!(b64().encode(mesh.positions_bytes()));
    reply["indices"] = json!(b64().encode(indices));
    Ok(reply)
}

pub(crate) fn link_push_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object to update; a new object is made when omitted."},
            "name": {"type": "string", "minLength": 1, "maxLength": 120, "description": "Name for a new object (made unique); default Blender."},
            "positions": {"type": "string", "description": "Base64 little-endian f32 x, y, z per vertex, in formsculpt's axes (y up)."},
            "indices": {"type": "string", "description": "Base64 little-endian u32, three per triangle. May be left out when id is given and only the positions changed."}
        },
        "required": ["positions"],
        "additionalProperties": false
    })
}

fn decode_positions(text: &str) -> Result<Vec<[f32; 3]>, CommandError> {
    let bytes = b64().decode(text.trim()).map_err(|e| bad(format!("positions is not base64: {e}")))?;
    if !bytes.len().is_multiple_of(12) {
        return Err(bad(format!("positions is {} bytes, not a whole number of f32 triples", bytes.len())));
    }
    if bytes.len() / 12 > VERTEX_LIMIT {
        return Err(bad(format!("{} vertices is more than the link carries ({VERTEX_LIMIT})", bytes.len() / 12)));
    }
    let f = |c: &[u8]| f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
    let positions: Vec<[f32; 3]> = bytes.as_chunks::<12>().0.iter().map(|c| [f(&c[0..4]), f(&c[4..8]), f(&c[8..12])]).collect();
    if positions.iter().flatten().any(|c| !c.is_finite()) {
        return Err(bad("positions holds a value that is not a finite number"));
    }
    Ok(positions)
}

fn decode_indices(text: &str) -> Result<Vec<u32>, CommandError> {
    let bytes = b64().decode(text.trim()).map_err(|e| bad(format!("indices is not base64: {e}")))?;
    if !bytes.len().is_multiple_of(12) {
        return Err(bad(format!("indices is {} bytes, not a whole number of u32 triangles", bytes.len())));
    }
    Ok(bytes.as_chunks::<4>().0.iter().map(|&c| u32::from_le_bytes(c)).collect())
}

/// Writes a mesh from the other app. When the triangles are the ones the object already has, only the positions move
/// (one Vertices step), so its paint, mask, face groups and UVs stay; otherwise the mesh is replaced (one Topology
/// step) as bridge_sync does. Without id it is a new object.
pub(crate) fn link_push(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let positions = decode_positions(args.get("positions").and_then(Value::as_str).ok_or_else(|| bad("positions is required"))?)?;
    let indices = match args.get("indices").and_then(Value::as_str) {
        Some(text) => Some(decode_indices(text)?),
        None => None,
    };
    let Some(id) = args.get("id").and_then(Value::as_u64).map(|i| i as u32) else {
        let indices = indices.ok_or_else(|| bad("a new object needs indices"))?;
        let mesh = checked(positions, indices)?;
        let base = args.get("name").and_then(Value::as_str).unwrap_or("Blender");
        s.history.push(Edit::Scene { label: "Blender Link".into(), objects: s.scene.objects.clone(), active: s.scene.active });
        let unique = s.scene.unique_name(base);
        let id = s.scene.add(unique, mesh);
        let object = s.scene.get(id).expect("added");
        return Ok(json!({"object": summary(object), "revision": object.revision, "created": true, "topology": true}));
    };
    let Some(object) = s.scene.get(id) else {
        return Err(CommandError::new("not_found", format!("no object {id}; get_scene lists object ids")));
    };
    let same = positions.len() == object.mesh.vertex_count() && indices.as_ref().is_none_or(|i| *i == object.mesh.indices);
    if same {
        let mut reply = set_positions(s, id, "Blender Link", positions);
        let revision = s.scene.get(id).expect("checked").revision;
        reply["revision"] = json!(revision);
        reply["created"] = json!(false);
        reply["topology"] = json!(false);
        return Ok(reply);
    }
    let indices = indices.ok_or_else(|| bad(format!("the vertex count changed ({} to {}); send indices too", object.mesh.vertex_count(), positions.len())))?;
    guard(s, id, "A topology change from the link")?;
    let mesh = checked(positions, indices)?;
    let object = s.scene.get_mut(id).expect("checked");
    let mask = vec![0.0; mesh.vertex_count()];
    let (old_mesh, old_mask, texture) = crate::history::replace_mesh(object, mesh, mask);
    object.revision += 1;
    s.history.push(Edit::Topology { label: "Blender Link".into(), object: id, mesh: old_mesh, mask: old_mask }.with_texture(id, texture));
    let object = s.scene.get(id).expect("checked");
    Ok(json!({"object": summary(object), "revision": object.revision, "created": false, "topology": true}))
}

fn checked(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Result<Mesh, CommandError> {
    if indices.is_empty() {
        return Err(bad("the mesh has no triangles"));
    }
    let n = positions.len() as u32;
    if let Some(i) = indices.iter().find(|&&i| i >= n) {
        return Err(bad(format!("index {i} is out of range for {n} vertices")));
    }
    Ok(Mesh::new(positions, indices))
}
