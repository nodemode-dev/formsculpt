use std::path::Path;

use fsc_core::ObjectId;
use fsc_io::{Format, GltfObject, Target};
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

fn io(msg: impl Into<String>) -> CommandError {
    CommandError::new("io_failed", msg)
}

fn path_arg(args: &Value) -> Result<&str, CommandError> {
    args.get("path").and_then(Value::as_str).filter(|p| !p.trim().is_empty()).ok_or_else(|| bad("path is required"))
}

pub fn save_data(s: &Session) -> Result<Vec<u8>, CommandError> {
    fsc_io::save_project(&s.scene).map_err(io)
}

pub fn open_data(s: &mut Session, bytes: Vec<u8>) -> Result<Value, CommandError> {
    let mut scene = fsc_io::open_project(bytes).map_err(|e| CommandError::new("bad_project", e))?;
    s.loads += 1;
    for o in scene.objects.iter_mut() {
        o.revision = (s.loads << 20) | (o.revision & 0xF_FFFF);
    }
    scene.reserve(s.scene.high_water());
    s.stroke = None;
    s.scene = scene;
    s.history.clear();
    Ok(json!({"objects": s.scene.objects.iter().map(summary).collect::<Vec<_>>(), "active": s.scene.active}))
}

pub fn import_data(s: &mut Session, file_name: &str, bytes: &[u8], name: Option<&str>) -> Result<Value, CommandError> {
    import_resolved(s, file_name, bytes, name, &|uri: &str| Err(format!("the glTF refers to an outside file ({uri}); import it from its path with import_mesh so the file beside it is read, or use GLB")))
}

fn import_resolved(s: &mut Session, file_name: &str, bytes: &[u8], name: Option<&str>, resolve: fsc_io::Resolve) -> Result<Value, CommandError> {
    let format = Format::parse(file_name).ok_or_else(|| bad(format!("unknown file type; formsculptor reads {}", Format::NAMES.join(", "))))?;
    let meshes = fsc_io::import_with(format, bytes, resolve).map_err(|e| CommandError::new("bad_file", e))?;
    s.history.push(Edit::Scene { label: "Import".into(), objects: s.scene.objects.clone(), active: s.scene.active });
    let stem = Path::new(file_name).file_stem().and_then(|x| x.to_str()).unwrap_or("Imported").to_string();
    let single = meshes.len() == 1;
    let mut added = Vec::new();
    for (part, mesh) in meshes {
        let base = match (name, single) {
            (Some(n), true) => n.to_string(),
            (Some(n), false) => format!("{n} {part}"),
            (None, true) => stem.clone(),
            (None, false) => part,
        };
        let unique = s.scene.unique_name(&base);
        let id = s.scene.add(unique, mesh);
        added.push(summary(s.scene.get(id).expect("added")));
    }
    Ok(json!({"objects": added}))
}

fn chosen(s: &Session, args: &Value) -> Result<Vec<ObjectId>, CommandError> {
    match args.get("ids").and_then(Value::as_array) {
        Some(list) => {
            let ids: Vec<ObjectId> = list.iter().filter_map(|v| v.as_u64().map(|x| x as ObjectId)).collect();
            if let Some(missing) = ids.iter().find(|&&id| s.scene.get(id).is_none()) {
                return Err(CommandError::new("not_found", format!("no object {missing}")));
            }
            Ok(ids)
        }
        None => Ok(s.scene.objects.iter().filter(|o| o.visible).map(|o| o.id).collect()),
    }
}

pub fn export_data(s: &Session, file_name: &str, ids: &[ObjectId]) -> Result<Vec<u8>, CommandError> {
    let format = Format::parse(file_name).ok_or_else(|| bad(format!("unknown file type; formsculptor writes obj, stl, ply, glb, gltf, fbx, usda and 3mf, not {file_name}")))?;
    if ids.is_empty() {
        return Err(bad("there is nothing to export"));
    }
    let textured: Vec<Option<fsc_core::Maps>> = ids.iter().map(|&id| matches!(format, Format::Glb | Format::Gltf).then(|| crate::texture_cmds::export_maps(s.scene.get(id).expect("checked"))).flatten()).collect();
    let objects: Vec<GltfObject> = ids
        .iter()
        .enumerate()
        .map(|(k, &id)| {
            let o = s.scene.get(id).expect("checked");
            GltfObject { name: &o.name, mesh: &o.mesh, targets: o.layers.list.iter().map(|l| Target { name: &l.name, weight: l.shape_scale(), delta: &l.delta }).collect(), maps: textured[k].as_ref().or(o.maps.as_ref()), tiles: crate::udim_cmds::tile_split(o) }
        })
        .collect();
    fsc_io::export(format, &objects).map_err(io)
}

pub(crate) fn path_schema(extra: Value) -> Value {
    let mut props = json!({"path": {"type": "string", "minLength": 1, "description": "A file path on this computer."}});
    if let (Some(p), Some(e)) = (props.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            p.insert(k.clone(), v.clone());
        }
    }
    json!({"type": "object", "properties": props, "required": ["path"], "additionalProperties": false})
}

pub(crate) fn save_schema() -> Value {
    path_schema(json!({}))
}

pub(crate) fn save(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let path = path_arg(args)?;
    if !path.to_ascii_lowercase().ends_with(".fsc") {
        return Err(bad("a project path ends in .fsc"));
    }
    let bytes = save_data(s)?;
    std::fs::write(path, &bytes).map_err(|e| io(format!("could not write {path}: {e}")))?;
    Ok(json!({"path": path, "bytes": bytes.len(), "objects": s.scene.objects.len()}))
}

pub(crate) fn open(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let path = path_arg(args)?;
    let bytes = std::fs::read(path).map_err(|e| io(format!("could not read {path}: {e}")))?;
    open_data(s, bytes)
}

pub(crate) fn import_schema() -> Value {
    path_schema(json!({"name": {"type": "string", "minLength": 1, "maxLength": 120, "description": "Name for the new object; the file name when omitted."}}))
}

pub(crate) fn import(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let path = path_arg(args)?;
    let bytes = std::fs::read(path).map_err(|e| io(format!("could not read {path}: {e}")))?;
    // A .gltf's buffers may be files beside it, named relative to it.
    let dir = Path::new(path).parent().map(Path::to_path_buf).unwrap_or_default();
    let beside = move |uri: &str| -> Result<Vec<u8>, String> {
        let decoded = uri.replace("%20", " ");
        if decoded.contains("://") || Path::new(&decoded).is_absolute() {
            return Err(format!("the glTF points at {uri}, which is not a file beside it"));
        }
        std::fs::read(dir.join(&decoded)).map_err(|e| format!("could not read {decoded} beside the glTF: {e}"))
    };
    import_resolved(s, path, &bytes, args.get("name").and_then(Value::as_str), &beside)
}

pub(crate) fn export_schema() -> Value {
    path_schema(json!({"ids": {"type": "array", "items": {"type": "integer", "minimum": 1}, "description": "Objects to write; every visible object when omitted. STL and PLY join them into one mesh."}}))
}

pub(crate) fn bridge_send_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "path": {"type": "string", "minLength": 1, "description": "The file to hand over, by extension: .obj, .glb, .fbx, .ply, .stl or .3mf (OBJ is the most widely read; FBX for Maya and 3ds Max)."}
        },
        "required": ["path"],
        "additionalProperties": false
    })
}

pub(crate) fn id_free_schema() -> Value {
    json!({"type": "object", "properties": {}, "required": [], "additionalProperties": false})
}

fn modified(path: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Bridge out: writes the object to a file another app opens and remembers it, so bridge_sync can bring the edits back.
pub(crate) fn bridge_send(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let path = path_arg(args)?.to_string();
    let format = Format::parse(&path).ok_or_else(|| bad("the bridge file must end in .obj, .glb, .fbx, .ply, .stl or .3mf"))?;
    if matches!(format, Format::Gltf) {
        return Err(bad("use .glb for the bridge; a .gltf is two files"));
    }
    let bytes = export_data(s, &path, &[id])?;
    std::fs::write(&path, &bytes).map_err(|e| io(format!("could not write {path}: {e}")))?;
    let seen = modified(&path).ok_or_else(|| io(format!("could not read back {path}")))?;
    s.bridges.retain(|(i, p, _)| *i != id && *p != path);
    s.bridges.push((id, path.clone(), seen));
    Ok(json!({"id": id, "path": path, "bytes": bytes.len(), "bridges": s.bridges.len()}))
}

/// Bridge back: every bridged file changed since it was sent or last synced replaces its object's mesh (same object,
/// same name, one undo step each). A file that is gone, or an object that is gone, ends that bridge.
pub(crate) fn bridge_sync(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    let mut updated = Vec::new();
    let mut problems = Vec::new();
    let mut keep = Vec::new();
    for (id, path, seen) in std::mem::take(&mut s.bridges) {
        let (Some(now), true) = (modified(&path), s.scene.get(id).is_some()) else { continue };
        if now <= seen {
            keep.push((id, path, seen));
            continue;
        }
        let read = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| {
            let format = Format::parse(&path).ok_or("unknown file type")?;
            fsc_io::import(format, &bytes)
        });
        match read {
            Ok(meshes) if !meshes.is_empty() => {
                let parts: Vec<&fsc_core::Mesh> = meshes.iter().map(|(_, m)| m).collect();
                let mesh = if parts.len() == 1 { parts[0].clone() } else { fsc_topo::merge(&parts) };
                let object = s.scene.get_mut(id).expect("checked");
                let mask = vec![0.0; mesh.vertex_count()];
                let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
                object.revision += 1;
                s.history.push(Edit::Topology { label: "Bridge Update".into(), object: id, mesh: old_mesh, mask: old_mask });
                updated.push(summary(s.scene.get(id).expect("checked")));
            }
            // A file caught half written reads as broken; it is tried again on the next sync.
            Ok(_) => problems.push(json!({"path": path, "message": "the file has no mesh"})),
            Err(e) => problems.push(json!({"path": path, "message": e})),
        }
        let seen = if updated.last().map(|o| o["id"] == id).unwrap_or(false) { now } else { seen };
        keep.push((id, path, seen));
    }
    s.bridges = keep;
    Ok(json!({"updated": updated, "problems": problems, "bridges": s.bridges.iter().map(|(id, path, _)| json!({"id": id, "path": path})).collect::<Vec<_>>()}))
}

pub(crate) fn export(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let path = path_arg(args)?;
    let ids = chosen(s, args)?;
    if path.to_ascii_lowercase().ends_with(".gltf") {
        // The separate form: the .gltf document and a .bin of the same name beside it.
        if ids.is_empty() {
            return Err(bad("there is nothing to export"));
        }
        let p = Path::new(path);
        let bin_name = format!("{}.bin", p.file_stem().and_then(|x| x.to_str()).unwrap_or("scene"));
        let textured: Vec<Option<fsc_core::Maps>> = ids.iter().map(|&id| crate::texture_cmds::export_maps(s.scene.get(id).expect("checked"))).collect();
        let objects: Vec<GltfObject> = ids
            .iter()
            .enumerate()
            .map(|(k, &id)| {
                let o = s.scene.get(id).expect("checked");
                GltfObject { name: &o.name, mesh: &o.mesh, targets: o.layers.list.iter().map(|l| Target { name: &l.name, weight: l.shape_scale(), delta: &l.delta }).collect(), maps: textured[k].as_ref().or(o.maps.as_ref()), tiles: crate::udim_cmds::tile_split(o) }
            })
            .collect();
        let (doc, bin) = fsc_io::export_gltf(&objects, &bin_name);
        let bin_path = p.with_file_name(&bin_name);
        std::fs::write(&bin_path, &bin).map_err(|e| io(format!("could not write {}: {e}", bin_path.display())))?;
        std::fs::write(path, &doc).map_err(|e| io(format!("could not write {path}: {e}")))?;
        return Ok(json!({"path": path, "bin": bin_path.to_string_lossy(), "bytes": doc.len() + bin.len(), "objects": ids.len()}));
    }
    let bytes = export_data(s, path, &ids)?;
    std::fs::write(path, &bytes).map_err(|e| io(format!("could not write {path}: {e}")))?;
    Ok(json!({"path": path, "bytes": bytes.len(), "objects": ids.len()}))
}

pub(crate) fn print_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "units_per_mm": {"type": "number", "exclusiveMinimum": 0, "default": 0.1, "description": "Scene units in one millimetre (0.1 means the default 2-unit sphere prints 20 mm across)."},
            "min_wall_mm": {"type": "number", "exclusiveMinimum": 0, "default": 1.0, "description": "Thinnest wall the printer can make."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn print_check(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let per_mm = args.get("units_per_mm").and_then(Value::as_f64).unwrap_or(0.1) as f32;
    let min_wall = args.get("min_wall_mm").and_then(Value::as_f64).unwrap_or(1.0) as f32;
    if per_mm <= 0.0 || min_wall <= 0.0 {
        return Err(bad("units_per_mm and min_wall_mm must be positive"));
    }
    let o = s.scene.get(id).expect("checked");
    let edges = fsc_topo::edge_faces(&o.mesh);
    let boundary = edges.values().filter(|f| f.len() == 1).count();
    let non_manifold = edges.values().filter(|f| f.len() > 2).count();
    let (mut volume, mut area) = (0.0f64, 0.0f64);
    for t in 0..o.mesh.triangle_count() {
        let [a, b, c] = o.mesh.triangle(t);
        let (pa, pb, pc) = (o.mesh.position(a), o.mesh.position(b), o.mesh.position(c));
        volume += (pa.dot(pb.cross(pc)) / 6.0) as f64;
        area += ((pb - pa).cross(pc - pa).length() * 0.5) as f64;
    }
    let n = o.mesh.vertex_count();
    let step = (n / 4000).max(1);
    let mut walls: Vec<f32> = Vec::new();
    for v in (0..n).step_by(step) {
        let p = o.mesh.position(v as u32);
        let normal = Vec3::from_array(o.mesh.normals[v]);
        let start = p - normal * 1e-4 * per_mm.max(1e-3);
        if let Some(hit) = o.bvh.raycast(&o.mesh, start, -normal) {
            walls.push((hit.t + 1e-4 * per_mm.max(1e-3)) / per_mm);
        }
    }
    walls.sort_by(|a, b| a.total_cmp(b));
    let thin = walls.iter().filter(|&&w| w < min_wall).count();
    let pct = |q: f32| walls.get(((walls.len() as f32 - 1.0) * q).round() as usize).copied();
    let size = o.mesh.bounds().extent() / per_mm;
    let mm3 = (volume.abs() / (per_mm as f64).powi(3)) as f32;
    let watertight = boundary == 0 && non_manifold == 0;
    let mut problems = Vec::new();
    if boundary > 0 {
        problems.push(format!("{boundary} open edges (fill_holes or repair_mesh)"));
    }
    if non_manifold > 0 {
        problems.push(format!("{non_manifold} non-manifold edges (repair_mesh)"));
    }
    if volume < 0.0 {
        problems.push("the mesh is inside out".to_string());
    }
    if thin > 0 {
        problems.push(format!("{thin} of {} sampled points have walls under {min_wall} mm", walls.len()));
    }
    Ok(json!({
        "id": id,
        "printable": watertight && volume > 0.0 && thin == 0,
        "watertight": watertight,
        "boundary_edges": boundary,
        "non_manifold_edges": non_manifold,
        "size_mm": size.to_array(),
        "volume_mm3": mm3,
        "area_mm2": (area / (per_mm as f64).powi(2)) as f32,
        "wall_mm": {"min": pct(0.0), "p5": pct(0.05), "median": pct(0.5), "samples": walls.len(), "thin": thin},
        "problems": problems,
    }))
}

pub(crate) fn repair(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    guard(s, id, "Repair")?;
    let o = s.scene.get(id).expect("checked");
    let (split, splits) = fsc_topo::split_nonmanifold(&o.mesh);
    let (filled, holes) = fsc_topo::fill_holes(&split);
    let mut indices = Vec::with_capacity(filled.indices.len());
    let mut degenerate = 0;
    for t in filled.indices.chunks_exact(3) {
        let (a, b, c) = (filled.position(t[0]), filled.position(t[1]), filled.position(t[2]));
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] || (b - a).cross(c - a).length_squared() == 0.0 {
            degenerate += 1;
        } else {
            indices.extend_from_slice(t);
        }
    }
    let mut mesh = fsc_core::Mesh::new(filled.positions, indices);
    let mut mask = vec![0.0; mesh.vertex_count()];
    let orphans = fsc_topo::compact(&mut mesh, &mut mask);
    mesh.recompute_normals();
    if splits + holes + degenerate + orphans == 0 {
        return Ok(json!({"id": id, "changed": false}));
    }
    let object = s.scene.get_mut(id).expect("checked");
    if object.mesh.has_paint() {
        fsc_topo::transfer_paint(&[(&object.mesh, &object.bvh)], &mut mesh);
    }
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    s.history.push(Edit::Topology { label: "Repair".into(), object: id, mesh: old_mesh, mask: old_mask });
    Ok(json!({"id": id, "changed": true, "non_manifold_vertices_split": splits, "holes_filled": holes, "degenerate_removed": degenerate, "orphans_removed": orphans, "object": summary(s.scene.get(id).expect("checked"))}))
}
