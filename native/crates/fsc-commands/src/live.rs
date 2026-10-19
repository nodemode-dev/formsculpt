//! The live engine surface the window uses beside the command registry: the mesh payload the viewport draws, live
//! strokes with their per-sample vertex updates, and the baked maps as one zip. The Python module (fsc-py) and the
//! browser build (fsc-wasm) both call these, so the window sees the same bytes whichever engine it talks to.

use fsc_core::Object;
use fsc_sculpt::{BrushKind, Stroke};
use glam::Vec3;
use serde_json::{json, Value};

use crate::{CommandError, Session};

/// Payload flags after the FSCM header.
pub const FLAG_MASK: u32 = 1;
pub const FLAG_PAINT: u32 = 2;
pub const FLAG_GROUPS: u32 = 4;
pub const FLAG_HIDDEN: u32 = 8;

fn not_found(id: u32) -> CommandError {
    CommandError::new("not_found", format!("no object {id}"))
}

fn header(vertices: usize, index_bytes: usize, revision: u64, flags: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(20);
    out.extend_from_slice(b"FSCM");
    out.extend_from_slice(&(vertices as u32).to_le_bytes());
    out.extend_from_slice(&((index_bytes / 4) as u32).to_le_bytes());
    out.extend_from_slice(&(revision as u32).to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out
}

/// An object's mesh as the window draws it: the FSCM header (magic, vertex count, index count, revision, flags), then
/// positions, normals, indices, mask, paint, face groups and hidden flags, each present as its flag says. With
/// `smooth` 1 to 3 it is the display-only smooth preview, carrying paint only.
pub fn mesh_payload(s: &Session, id: u32, smooth: u32) -> Result<Vec<u8>, CommandError> {
    let object = s.scene.get(id).ok_or_else(|| not_found(id))?;
    if smooth > 0 {
        let mesh = fsc_topo::smooth_preview(&object.mesh, smooth.min(3));
        let paint = mesh.paint_bytes();
        let indices = mesh.indices_bytes();
        let mut out = header(mesh.vertex_count(), indices.len(), object.revision, if paint.is_empty() { 0 } else { FLAG_PAINT });
        out.extend_from_slice(&mesh.positions_bytes());
        out.extend_from_slice(&mesh.normals_bytes());
        out.extend_from_slice(&indices);
        out.extend_from_slice(&paint);
        return Ok(out);
    }
    let mesh = &object.mesh;
    let (paint, groups, hidden, indices) = (mesh.paint_bytes(), mesh.groups_bytes(), mesh.hidden_bytes(), mesh.indices_bytes());
    let flags = FLAG_MASK | if paint.is_empty() { 0 } else { FLAG_PAINT } | if groups.is_empty() { 0 } else { FLAG_GROUPS } | if hidden.is_empty() { 0 } else { FLAG_HIDDEN };
    let mut out = header(mesh.vertex_count(), indices.len(), object.revision, flags);
    out.extend_from_slice(&mesh.positions_bytes());
    out.extend_from_slice(&mesh.normals_bytes());
    out.extend_from_slice(&indices);
    out.extend_from_slice(&object.mask_bytes());
    out.extend_from_slice(&paint);
    out.extend_from_slice(&groups);
    out.extend_from_slice(&hidden);
    Ok(out)
}

/// A live boolean preview: the exact union, difference or intersection of objects `a` and `b` as an FSCM payload
/// (revision 0, no mask), computed without changing the scene, for the window to draw in their place while either moves.
pub fn boolean_payload(s: &Session, a: u32, b: u32, op: &str) -> Result<Vec<u8>, CommandError> {
    let oa = s.scene.get(a).ok_or_else(|| not_found(a))?;
    let ob = s.scene.get(b).ok_or_else(|| not_found(b))?;
    let op = match op {
        "union" => fsc_topo::BooleanOp::Union,
        "difference" => fsc_topo::BooleanOp::Difference,
        "intersection" => fsc_topo::BooleanOp::Intersection,
        other => return Err(CommandError::new("bad_arguments", format!("op must be union, difference or intersection, not {other}"))),
    };
    let mesh = fsc_topo::exact_boolean(&oa.mesh, &ob.mesh, op).map_err(|e| CommandError::new("boolean_failed", e))?;
    let (paint, indices) = (mesh.paint_bytes(), mesh.indices_bytes());
    let mut out = header(mesh.vertex_count(), indices.len(), 0, FLAG_MASK | if paint.is_empty() { 0 } else { FLAG_PAINT });
    out.extend_from_slice(&mesh.positions_bytes());
    out.extend_from_slice(&mesh.normals_bytes());
    out.extend_from_slice(&indices);
    out.extend(std::iter::repeat_n(0u8, mesh.vertex_count() * 4));
    out.extend_from_slice(&paint);
    Ok(out)
}

/// The solid a triplanar's three masks make now, as an FSCM payload for the window to draw beside the planes while
/// they are masked (Nomad's Triplanar shows its shape as it is painted); the scene is unchanged. `resolution` is the
/// voxel count along the longest side (16 to 256; the window asks for a coarse one so it keeps up).
pub fn triplanar_payload(s: &Session, id: u32, resolution: u32) -> Result<Vec<u8>, CommandError> {
    let object = s.scene.get(id).ok_or_else(|| not_found(id))?;
    if !object.primitive.as_ref().is_some_and(|(a, _)| a.contains("\"triplanar\"")) || fsc_core::triplanar::divisions(&object.mesh).is_none() {
        return Err(CommandError::new("not_triplanar", format!("object {id} is not a triplanar's three planes")));
    }
    let mesh = fsc_topo::triplanar_solid(&object.mesh, &object.mask, 0.04, resolution.clamp(16, 256)).map_err(|e| CommandError::new("build_failed", e))?;
    let indices = mesh.indices_bytes();
    let mut out = header(mesh.vertex_count(), indices.len(), object.revision, FLAG_MASK);
    out.extend_from_slice(&mesh.positions_bytes());
    out.extend_from_slice(&mesh.normals_bytes());
    out.extend_from_slice(&indices);
    out.extend(std::iter::repeat_n(0u8, mesh.vertex_count() * 4));
    Ok(out)
}

/// Starts a live stroke on an object with brush settings as the window sends them (sculpt_stroke's brush keys).
/// Dynamic topology is off on objects with levels or sculpt layers, which need a fixed vertex order.
pub fn stroke_begin(s: &mut Session, id: u32, settings: &Value) -> Result<(), CommandError> {
    let mut brush = crate::parse_brush(settings)?;
    let object = s.scene.get(id).ok_or_else(|| not_found(id))?;
    crate::sculpt_cmds::local_symmetry(&mut brush, object);
    if brush.texture && object.live_texture().is_none() {
        return Err(crate::sculpt_cmds::no_texture());
    }
    if object.multires.is_some() || !object.layers.list.is_empty() {
        brush.dyntopo = None;
        brush.dyntopo_size = None;
    }
    let alpha = crate::stroke_alpha(s, settings)?;
    let mut stroke = Stroke::new(brush);
    stroke.set_alpha(alpha);
    s.stroke = Some((id, stroke));
    Ok(())
}

/// One stroke sample along a ray: whether it hit, the FSCU update of the vertices it changed (empty when the topology
/// changed, since the whole mesh must then be fetched again), and whether the topology changed.
pub fn stroke_sample(s: &mut Session, origin: [f32; 3], dir: [f32; 3], pressure: f32, tilt: f32) -> Result<(bool, Vec<u8>, bool), CommandError> {
    let (id, stroke) = s.stroke.as_mut().ok_or_else(|| CommandError::new("no_stroke", "no stroke in progress"))?;
    stroke.set_tilt(tilt);
    let object = s.scene.get_mut(*id).ok_or_else(|| CommandError::new("not_found", "the stroke's object is gone"))?;
    let hit = stroke.sample_ray(object, Vec3::from_array(origin), Vec3::from_array(dir), pressure);
    let changed = stroke.drain_changed();
    if stroke.take_topology_dirty() {
        object.revision += 1;
        return Ok((hit, Vec::new(), true));
    }
    Ok((hit, update_bytes(object, &changed), false))
}

/// Ends the live stroke as one undo step; the reply is what the window reads (`ok`, and the result or null).
pub fn stroke_end(s: &mut Session) -> Value {
    let Some((id, stroke)) = s.stroke.take() else { return json!({"ok": true, "result": null}) };
    let kind = stroke.settings.kind;
    let on_texture = stroke.settings.texture;
    let dabs = stroke.dabs;
    let no_layer = stroke.no_active_layer;
    let Some(object) = s.scene.get_mut(id) else { return json!({"ok": false, "error": {"code": "not_found", "message": "the stroke's object is gone"}}) };
    let result = stroke.finish(object);
    let revision = object.revision;
    let topology = matches!(result, fsc_sculpt::StrokeEdit::Topology { .. });
    let name = BrushKind::NAMES[kind as usize];
    let label = format!("{}{} Stroke", name[..1].to_ascii_uppercase(), name[1..].replace('_', " "));
    let changed = crate::push_stroke(s, id, label, result);
    crate::sync_instances(s);
    let texture_revision = s.scene.get(id).and_then(|o| o.texture.as_ref()).map(|t| t.revision);
    if on_texture {
        return json!({"ok": true, "result": {"id": id, "dabs": dabs, "vertices_changed": 0, "texels_changed": changed, "texture_revision": texture_revision, "revision": revision, "topology": false, "no_active_layer": no_layer}});
    }
    json!({"ok": true, "result": {"id": id, "dabs": dabs, "vertices_changed": changed, "revision": revision, "topology": topology, "no_active_layer": no_layer}})
}

/// The FSCU vertex update: magic, count, revision, object id, then indices, positions, normals, mask and (when the
/// object is painted) paint of the changed vertices.
pub fn update_bytes(object: &Object, changed: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + changed.len() * (32 + 4 * fsc_core::PAINT_LEN));
    out.extend_from_slice(b"FSCU");
    out.extend_from_slice(&(changed.len() as u32).to_le_bytes());
    out.extend_from_slice(&(object.revision as u32).to_le_bytes());
    out.extend_from_slice(&object.id.to_le_bytes());
    for &v in changed {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for &v in changed {
        for c in object.mesh.positions[v as usize] {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    for &v in changed {
        for c in object.mesh.normals[v as usize] {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    for &v in changed {
        out.extend_from_slice(&object.mask[v as usize].to_le_bytes());
    }
    if object.mesh.has_paint() {
        for &v in changed {
            for c in object.mesh.paint[v as usize] {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
    }
    out
}

/// The baked map files of an object, named as the maps download names them; None when nothing is baked.
pub fn map_files(object: &Object) -> Option<Vec<(String, Vec<u8>)>> {
    let m = object.maps.as_ref()?;
    let stem: String = object.name.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    let mut files = vec![
        (format!("{stem}_color.png"), m.color_png.clone()),
        (format!("{stem}_roughness_metal.png"), m.mr_png.clone()),
        (format!("{stem}_normal.png"), m.normal_png.clone()),
        (format!("{stem}_displacement.exr"), m.displacement_exr.clone()),
    ];
    if !m.emissive_png.is_empty() {
        files.push((format!("{stem}_emissive.png"), m.emissive_png.clone()));
    }
    Some(files)
}

/// The baked maps of an object as one zip.
pub fn maps_zip(s: &Session, id: u32) -> Result<Vec<u8>, CommandError> {
    let object = s.scene.get(id).ok_or_else(|| not_found(id))?;
    let files = map_files(object).ok_or_else(|| CommandError::new("no_maps", "the object has no baked maps; run bake_maps first"))?;
    let refs: Vec<(String, &[u8])> = files.iter().map(|(n, d)| (n.clone(), d.as_slice())).collect();
    fsc_io::zip_files(&refs).map_err(|e| CommandError::new("encode_failed", e))
}
