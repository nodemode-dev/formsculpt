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
            "preserve_borders": {"type": "boolean", "default": true}
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
    let preserve = args.get("preserve_borders").and_then(Value::as_bool).unwrap_or(true);
    let mesh = fsc_topo::decimate(&object.mesh, target_tris, preserve);
    let mask = vec![0.0; mesh.vertex_count()];
    apply(s, id, "Decimate", mesh, mask)
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
