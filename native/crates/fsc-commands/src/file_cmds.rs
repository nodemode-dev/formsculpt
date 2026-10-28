use std::path::Path;

use fsc_core::ObjectId;
use fsc_io::{Format, GltfObject, Target};
use glam::Vec3;
use serde_json::{json, Value};

use crate::exchange::{self, Exchange};
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
    import_data_with(s, file_name, bytes, name, &Exchange::default())
}

/// import_data with import options: a scale, the file's up axis and joining its parts into one object.
pub fn import_data_with(s: &mut Session, file_name: &str, bytes: &[u8], name: Option<&str>, ex: &Exchange) -> Result<Value, CommandError> {
    import_resolved(s, file_name, bytes, name, &|uri: &str| Err(format!("the glTF refers to an outside file ({uri}); import it from its path with import_mesh so the file beside it is read, or use GLB")), ex)
}

fn import_resolved(s: &mut Session, file_name: &str, bytes: &[u8], name: Option<&str>, resolve: fsc_io::Resolve, ex: &Exchange) -> Result<Value, CommandError> {
    let format = Format::parse(file_name).ok_or_else(|| bad(format!("unknown file type; formsculpt reads {}", Format::NAMES.join(", "))))?;
    let mut meshes = fsc_io::import_with(format, bytes, resolve).map_err(|e| CommandError::new("bad_file", e))?;
    let turned = exchange::turns_in(format);
    for (_, mesh) in meshes.iter_mut() {
        exchange::inward_mesh(mesh, ex.scale, ex.z_up && turned);
    }
    if ex.merge && meshes.len() > 1 {
        let parts: Vec<&fsc_core::Mesh> = meshes.iter().map(|(_, m)| m).collect();
        let joined = fsc_topo::merge(&parts);
        meshes = vec![(String::new(), joined)];
    }
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
    export_data_with(s, file_name, ids, &Exchange::default())
}

/// The scale an export is written at: the option's, times millimetres per unit for the print formats when the scene
/// has a real-world scale (3MF declares millimetre as its unit and STL carries none, so this makes the file print at
/// the size set).
fn written_scale(s: &Session, format: Format, ex: &Exchange) -> f32 {
    match (s.scene.mm_per_unit, format) {
        (Some(mm), Format::Stl | Format::ThreeMf) => ex.scale * mm,
        _ => ex.scale,
    }
}

/// Owned copies of the meshes and layer deltas an export writes, turned and scaled, or None for an object written as
/// it is.
type Prepared = Vec<Option<(fsc_core::Mesh, Vec<Vec<[f32; 3]>>)>>;

fn prepare(s: &Session, ids: &[ObjectId], scale: f32, z_up: bool, triangulate: bool) -> Prepared {
    ids.iter()
        .map(|&id| {
            let o = s.scene.get(id).expect("checked");
            let mesh = exchange::outward_mesh(&o.mesh, scale, z_up, triangulate)?;
            // A layer's delta is a direction, so it turns and scales without the move.
            let deltas = o.layers.list.iter().map(|l| l.delta.iter().map(|d| exchange::outward(*d, scale, z_up)).collect()).collect();
            Some((mesh, deltas))
        })
        .collect()
}

fn objects<'a>(s: &'a Session, ids: &[ObjectId], prepared: &'a Prepared, textured: &'a [Option<fsc_core::Maps>]) -> Vec<GltfObject<'a>> {
    ids.iter()
        .enumerate()
        .map(|(k, &id)| {
            let o = s.scene.get(id).expect("checked");
            let (mesh, deltas) = match &prepared[k] {
                Some((m, d)) => (m, Some(d)),
                None => (&o.mesh, None),
            };
            let targets = o.layers.list.iter().enumerate().map(|(j, l)| Target { name: &l.name, weight: l.shape_scale(), delta: deltas.map(|d| d[j].as_slice()).unwrap_or(&l.delta) }).collect();
            GltfObject { name: &o.name, mesh, targets, maps: textured[k].as_ref().or(o.maps.as_ref()), tiles: crate::udim_cmds::tile_split(o) }
        })
        .collect()
}

/// export_data with export options: a scale, a z-up axis for the formats without one of their own, and triangles.
pub fn export_data_with(s: &Session, file_name: &str, ids: &[ObjectId], ex: &Exchange) -> Result<Vec<u8>, CommandError> {
    let format = Format::parse(file_name).ok_or_else(|| bad(format!("unknown file type; formsculpt writes obj, stl, ply, glb, gltf, fbx, usda and 3mf, not {file_name}")))?;
    if ids.is_empty() {
        return Err(bad("there is nothing to export"));
    }
    let textured: Vec<Option<fsc_core::Maps>> = ids.iter().map(|&id| matches!(format, Format::Glb | Format::Gltf).then(|| crate::texture_cmds::export_maps(s.scene.get(id).expect("checked"))).flatten()).collect();
    let prepared = prepare(s, ids, written_scale(s, format, ex), ex.z_up && exchange::turns(format), ex.triangulate);
    fsc_io::export(format, &objects(s, ids, &prepared, &textured)).map_err(io)
}

pub(crate) fn real_size_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object to measure by; the active object when omitted."},
            "size_mm": {"type": "number", "exclusiveMinimum": 0, "maximum": 100000, "description": "How big that object is in the real world along axis, in millimetres; sets the scene's scale from it."},
            "axis": {"type": "string", "enum": ["longest", "x", "y", "z"], "default": "longest", "description": "Which of the object's sides size_mm measures."},
            "mm_per_unit": {"type": ["number", "null"], "exclusiveMinimum": 0, "description": "Or set the scale directly: millimetres in one scene unit; null clears it."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Sets the scene's real-world scale, from an object's size in millimetres or directly. Nothing moves: the scale only
/// says how big a scene unit is, for print check and the print formats. Reports every object's size in millimetres.
pub(crate) fn set_real_size(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    match (args.get("size_mm"), args.get("mm_per_unit")) {
        (Some(_), Some(_)) => return Err(bad("give size_mm or mm_per_unit, not both")),
        (Some(v), None) => {
            let mm = v.as_f64().filter(|x| *x > 0.0).ok_or_else(|| bad("size_mm must be a positive number"))? as f32;
            let id = target(s, args)?;
            let e = s.scene.get(id).expect("checked").mesh.bounds().extent();
            let side = match args.get("axis").and_then(Value::as_str).unwrap_or("longest") {
                "x" => e.x,
                "y" => e.y,
                "z" => e.z,
                "longest" => e.max_element(),
                other => return Err(bad(format!("axis must be longest, x, y or z, not {other}"))),
            };
            if side <= 1e-9 {
                return Err(bad("the object has no size along that axis"));
            }
            s.scene.mm_per_unit = Some(mm / side);
        }
        (None, Some(Value::Null)) => s.scene.mm_per_unit = None,
        (None, Some(v)) => s.scene.mm_per_unit = Some(v.as_f64().filter(|x| *x > 0.0).ok_or_else(|| bad("mm_per_unit must be a positive number or null"))? as f32),
        (None, None) => return Err(bad("give size_mm (with id and axis) or mm_per_unit")),
    }
    let mm = s.scene.mm_per_unit;
    let sizes: Vec<Value> = s.scene.objects.iter().map(|o| {
        let e = o.mesh.bounds().extent();
        json!({"id": o.id, "name": o.name, "size_mm": mm.map(|m| [e.x * m, e.y * m, e.z * m])})
    }).collect();
    Ok(json!({"mm_per_unit": mm, "objects": sizes}))
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
    let mut extra = exchange::properties(false);
    extra["name"] = json!({"type": "string", "minLength": 1, "maxLength": 120, "description": "Name for the new object; the file name when omitted."});
    path_schema(extra)
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
    let ex = exchange::parse(args)?;
    let mut reply = import_resolved(s, path, &bytes, args.get("name").and_then(Value::as_str), &beside, &ex)?;
    reply["options"] = exchange::applied(&ex, Format::parse(path).is_some_and(exchange::turns_in));
    Ok(reply)
}

pub(crate) fn export_schema() -> Value {
    let mut extra = exchange::properties(true);
    extra["ids"] = json!({"type": "array", "items": {"type": "integer", "minimum": 1}, "description": "Objects to write; every visible object when omitted. STL and PLY join them into one mesh."});
    path_schema(extra)
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
    let bytes = bridge_write(s, id, &path, 1.0, true)?;
    Ok(json!({"id": id, "path": path, "bytes": bytes, "bridges": s.bridges.len()}))
}

/// Writes object id to path at scale and opens a bridge on it. exclusive closes the object's other bridges, as
/// bridge_send does (one file per object); GoF keeps them, so every app an object was sent to can send it back.
/// Returns the bytes written.
pub(crate) fn bridge_write(s: &mut Session, id: ObjectId, path: &str, scale: f32, exclusive: bool) -> Result<usize, CommandError> {
    let ex = Exchange { scale, ..Exchange::default() };
    let bytes = export_data_with(s, path, &[id], &ex)?;
    std::fs::write(path, &bytes).map_err(|e| io(format!("could not write {path}: {e}")))?;
    let seen = modified(path).ok_or_else(|| io(format!("could not read back {path}")))?;
    s.bridges.retain(|(i, p, _, _)| p != path && !(exclusive && *i == id));
    s.bridges.push((id, path.to_string(), seen, scale));
    Ok(bytes.len())
}

/// Bridge back: every bridged file changed since it was sent or last synced replaces its object's mesh (same object,
/// same name, one undo step each). A file that is gone, or an object that is gone, ends that bridge.
pub(crate) fn bridge_sync(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    let mut updated = Vec::new();
    let mut problems = Vec::new();
    let mut keep = Vec::new();
    for (id, path, seen, scale) in std::mem::take(&mut s.bridges) {
        let (Some(now), true) = (modified(&path), s.scene.get(id).is_some()) else { continue };
        if now <= seen {
            keep.push((id, path, seen, scale));
            continue;
        }
        let read = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| {
            let format = Format::parse(&path).ok_or("unknown file type")?;
            fsc_io::import(format, &bytes)
        });
        match read {
            Ok(meshes) if !meshes.is_empty() => {
                let parts: Vec<&fsc_core::Mesh> = meshes.iter().map(|(_, m)| m).collect();
                let mut mesh = if parts.len() == 1 { parts[0].clone() } else { fsc_topo::merge(&parts) };
                // Written at a scale (GoF's units), so read back at its inverse; normals do not change.
                if (scale - 1.0).abs() > f32::EPSILON {
                    for p in &mut mesh.positions {
                        *p = [p[0] / scale, p[1] / scale, p[2] / scale];
                    }
                }
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
        keep.push((id, path, seen, scale));
    }
    s.bridges = keep;
    Ok(json!({"updated": updated, "problems": problems, "bridges": s.bridges.iter().map(|(id, path, _, scale)| json!({"id": id, "path": path, "scale": scale})).collect::<Vec<_>>()}))
}

pub(crate) fn export(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let path = path_arg(args)?;
    let ids = chosen(s, args)?;
    let ex = exchange::parse(args)?;
    let options = exchange::applied(&ex, Format::parse(path).is_some_and(exchange::turns));
    if path.to_ascii_lowercase().ends_with(".gltf") {
        // The separate form: the .gltf document and a .bin of the same name beside it.
        if ids.is_empty() {
            return Err(bad("there is nothing to export"));
        }
        let p = Path::new(path);
        let bin_name = format!("{}.bin", p.file_stem().and_then(|x| x.to_str()).unwrap_or("scene"));
        let textured: Vec<Option<fsc_core::Maps>> = ids.iter().map(|&id| crate::texture_cmds::export_maps(s.scene.get(id).expect("checked"))).collect();
        // glTF is y up by its specification, so only the scale applies.
        let prepared = prepare(s, &ids, ex.scale, false, false);
        let (doc, bin) = fsc_io::export_gltf(&objects(s, &ids, &prepared, &textured), &bin_name);
        let bin_path = p.with_file_name(&bin_name);
        std::fs::write(&bin_path, &bin).map_err(|e| io(format!("could not write {}: {e}", bin_path.display())))?;
        std::fs::write(path, &doc).map_err(|e| io(format!("could not write {path}: {e}")))?;
        return Ok(json!({"path": path, "bin": bin_path.to_string_lossy(), "bytes": doc.len() + bin.len(), "objects": ids.len(), "options": options}));
    }
    let bytes = export_data_with(s, path, &ids, &ex)?;
    std::fs::write(path, &bytes).map_err(|e| io(format!("could not write {path}: {e}")))?;
    Ok(json!({"path": path, "bytes": bytes.len(), "objects": ids.len(), "options": options}))
}

pub(crate) fn print_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "units_per_mm": {"type": "number", "exclusiveMinimum": 0, "description": "Scene units in one millimetre; the scene's real-world scale (set_real_size) when set, else 0.1 (the default 2-unit sphere prints 20 mm across)."},
            "min_wall_mm": {"type": "number", "exclusiveMinimum": 0, "default": 1.0, "description": "Thinnest wall the printer can make."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn print_check(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let per_mm = args.get("units_per_mm").and_then(Value::as_f64).map(|v| v as f32).or(s.scene.mm_per_unit.map(|m| 1.0 / m)).unwrap_or(0.1);
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
    for t in filled.indices.as_chunks::<3>().0 {
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
