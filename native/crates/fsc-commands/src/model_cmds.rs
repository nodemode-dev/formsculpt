use fsc_core::time::Instant;
use glam::Vec3;

use serde_json::{json, Value};

use crate::history::Edit;
use crate::multires_cmds::guard;
use crate::object_cmds::set_positions;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::topo_cmds::apply;
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

fn no_mask() -> CommandError {
    CommandError::new("empty_mask", "nothing is masked; mask the area first (faces whose three corners are at least half masked count)")
}

pub(crate) fn quad_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "faces": {"type": "integer", "minimum": 100, "maximum": 400000, "default": 4000, "description": "Target number of quads."},
            "relax": {"type": "integer", "minimum": 0, "maximum": 30, "default": 6, "description": "Smoothing passes that even out the quads while staying on the original surface."},
            "guides": {"type": "array", "maxItems": 64, "items": {"type": "array", "minItems": 2, "maxItems": 4096, "items": crate::curve_cmds::point_schema(3)}, "description": "Guide curves as lists of [x, y, z] points or objects {p, sharp, round} (smoothed through catmull_rom, corners respected; drawn with the Curve stroke tool, or along a limb): the quads' edge loops are turned to run along their mean direction, as ZRemesher's curves steer it. Without guides the grid follows the world axes."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn quad_remesh(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Quad remesh")?;
    let faces = number(args, "faces", 4000.0, 100.0, 400000.0)? as usize;
    let relax = number(args, "relax", 6.0, 0.0, 30.0)? as u32;
    let started = Instant::now();
    let object = s.scene.get(id).expect("checked");
    let guides: Vec<Vec<Vec3>> = match args.get("guides").and_then(Value::as_array) {
        Some(list) => list.iter().map(crate::curve_cmds::guide_positions).collect::<Result<_, _>>()?,
        None => Vec::new(),
    };
    let frame = if guides.is_empty() { None } else { Some(fsc_topo::guide_frame(&guides).ok_or_else(|| CommandError::new("bad_arguments", "the guides have no length"))?) };
    let out = match frame {
        Some(f) => fsc_topo::quad_remesh_aligned(&object.mesh, f, faces, relax),
        None => fsc_topo::quad_remesh(&object.mesh, &object.bvh, faces, relax),
    }
    .map_err(|e| CommandError::new("remesh_failed", e))?;
    let quads = out.mesh.triangle_count() / 2;
    let mask = vec![0.0; out.mesh.vertex_count()];
    let mut reply = apply(s, id, "Quad Remesh", out.mesh, mask)?;
    reply["quads"] = json!(quads);
    reply["ms"] = json!(started.elapsed().as_millis());
    Ok(reply)
}

pub(crate) fn project_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object to move; the active object when omitted."},
            "source_id": {"type": "integer", "minimum": 1, "description": "The object whose surface detail is taken."},
            "distance": {"type": "number", "exclusiveMinimum": 0, "description": "Furthest a vertex may move, in world units; 5 percent of the object's size when omitted."},
            "strength": {"type": "number", "minimum": 0, "maximum": 1, "default": 1}
        },
        "required": ["source_id"],
        "additionalProperties": false
    })
}

pub(crate) fn project_detail(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let source_id = args.get("source_id").and_then(Value::as_u64).ok_or_else(|| bad("source_id is required"))? as u32;
    if source_id == id {
        return Err(bad("source_id must be another object"));
    }
    let src = s.scene.get(source_id).ok_or_else(|| CommandError::new("not_found", format!("no object {source_id}")))?;
    let object = s.scene.get(id).expect("checked");
    let distance = args.get("distance").and_then(Value::as_f64).map(|d| d as f32).unwrap_or_else(|| object.mesh.bounds().extent().length() * 0.05);
    let strength = number(args, "strength", 1.0, 0.0, 1.0)? as f32;
    let (positions, moved) = fsc_topo::project_onto(&object.mesh, &object.mask, &src.mesh, &src.bvh, distance, strength);
    let mut reply = set_positions(s, id, "Project Detail", positions);
    reply["vertices_projected"] = json!(moved);
    Ok(reply)
}

pub(crate) fn extract_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "thickness": {"type": "number", "minimum": -10000, "maximum": 10000, "description": "Thickness in world units (a shell's offset along the surface normal, a balloon's dome height, an extrusion's or badge's depth); 2 percent of the object's size when omitted. Negative grows the other way."},
            "mode": {"type": "string", "enum": ["shell", "balloon", "extrude", "splat", "project"], "default": "shell", "description": "shell: the patch and a copy offset outward along the normals, joined at the border. balloon: a copy of the patch pushed out along the normals by thickness times a dome profile (1 in the middle, 0 at the border, from the distance along the surface to the border), welded to the original border, with the unmoved patch as the back. extrude: the patch moved straight along its area-weighted average normal by thickness, with side walls. splat: like shell, but the back is flat, on the plane perpendicular to the patch's average normal touching its lowest point (shell is already outward-only with its back on the surface), and the front is the surface offset by thickness. project: the patch flattened onto the plane through its centroid perpendicular to direction, then extruded by thickness along it, a flat badge in the mask's outline."},
            "direction": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "project only: the projection direction as [x, y, z], not zero; the patch's average normal when omitted."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn extract(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get(id).expect("checked");
    let thickness = args.get("thickness").and_then(Value::as_f64).map(|t| t as f32).unwrap_or_else(|| object.mesh.bounds().extent().length() * 0.02);
    let direction = match args.get("direction") {
        None | Some(Value::Null) => None,
        Some(Value::Array(a)) if a.len() == 3 => {
            let f: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
            let d = Vec3::new(f[0], f[1], f[2]);
            if !d.is_finite() || d.length_squared() < 1e-12 {
                return Err(bad("direction is [x, y, z], not zero"));
            }
            Some(d)
        }
        Some(_) => return Err(bad("direction is [x, y, z]")),
    };
    let patch = match args.get("mode").and_then(Value::as_str).unwrap_or("shell") {
        "shell" => None,
        "balloon" => Some(fsc_topo::PatchMode::Balloon),
        "extrude" => Some(fsc_topo::PatchMode::Extrude),
        "splat" => Some(fsc_topo::PatchMode::Splat),
        "project" => Some(fsc_topo::PatchMode::Project),
        other => return Err(bad(format!("unknown mode {other}; use shell, balloon, extrude, splat or project"))),
    };
    let mesh = match patch {
        None => fsc_topo::extract_shell(&object.mesh, &object.mask, thickness).ok_or_else(no_mask)?,
        Some(mode) => {
            if !fsc_topo::region_faces(&object.mesh, &object.mask).iter().any(|&x| x) {
                return Err(no_mask());
            }
            fsc_topo::extract_patch(&object.mesh, &object.mask, mode, thickness, direction).map_err(|e| CommandError::new("extract_failed", e))?
        }
    };
    let name = s.scene.unique_name(&format!("{} extract", object.name));
    let new_id = s.scene.add(name, mesh);
    let index = s.scene.index_of(new_id).expect("added");
    s.history.push(Edit::Add { id: new_id, index, parked: None });
    Ok(summary(s.scene.get(new_id).expect("added")))
}

pub(crate) fn extrude_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "distance": {"type": "number", "minimum": -10000, "maximum": 10000, "description": "How far the masked faces move along their normals, in world units; 3 percent of the object's size when omitted. Negative pushes in."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn extrude(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Extrude")?;
    let object = s.scene.get(id).expect("checked");
    let distance = args.get("distance").and_then(Value::as_f64).map(|d| d as f32).unwrap_or_else(|| object.mesh.bounds().extent().length() * 0.03);
    let (mesh, mask) = fsc_topo::extrude_region(&object.mesh, &object.mask, distance, 0.0).ok_or_else(no_mask)?;
    apply(s, id, "Extrude", mesh, mask)
}

pub(crate) fn inset_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "amount": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.3, "description": "How far the masked area's border pulls inward, as a fraction of the border faces."},
            "depth": {"type": "number", "minimum": -10000, "maximum": 10000, "default": 0, "description": "Also move the inset faces along their normals by this much."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn inset(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Inset")?;
    let amount = number(args, "amount", 0.3, 0.0, 1.0)? as f32;
    let depth = number(args, "depth", 0.0, -10000.0, 10000.0)? as f32;
    let object = s.scene.get(id).expect("checked");
    let (mesh, mask) = fsc_topo::extrude_region(&object.mesh, &object.mask, depth, amount).ok_or_else(no_mask)?;
    apply(s, id, "Inset", mesh, mask)
}
