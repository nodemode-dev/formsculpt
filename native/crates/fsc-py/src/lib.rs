use std::sync::Mutex;

use fsc_commands::Session;
use fsc_sculpt::{BrushKind, Stroke};
use glam::Vec3;
use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde_json::{json, Value};

type MapFiles<'py> = (String, Vec<(String, Bound<'py, PyBytes>)>);
type MeshBuffers<'py> = (Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>, u64, Bound<'py, PyBytes>);

#[pyclass]
struct Engine {
    session: Mutex<Session>,
}

#[pymethods]
impl Engine {
    #[new]
    fn new() -> Self {
        Engine { session: Mutex::new(Session::default()) }
    }

    fn dispatch(&self, name: &str, args: &str) -> PyResult<String> {
        let parsed: Value = if args.trim().is_empty() { json!({}) } else { serde_json::from_str(args).map_err(|e| PyValueError::new_err(format!("arguments are not JSON: {e}")))? };
        let mut session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let reply = match fsc_commands::dispatch(&mut session, name, &parsed) {
            Ok(result) => json!({"ok": true, "result": result}),
            Err(err) => json!({"ok": false, "error": err}),
        };
        Ok(reply.to_string())
    }

    fn save_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let bytes = fsc_commands::save_data(&session).map_err(|e| PyValueError::new_err(e.message))?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn open_bytes(&self, data: &[u8]) -> PyResult<String> {
        let mut session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        Ok(reply(fsc_commands::open_data(&mut session, data.to_vec())))
    }

    #[pyo3(signature = (file_name, data, name=None))]
    fn import_bytes(&self, file_name: &str, data: &[u8], name: Option<&str>) -> PyResult<String> {
        let mut session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        Ok(reply(fsc_commands::import_data(&mut session, file_name, data, name)))
    }

    #[pyo3(signature = (file_name, ids=None))]
    fn export_bytes<'py>(&self, py: Python<'py>, file_name: &str, ids: Option<Vec<u32>>) -> PyResult<Bound<'py, PyBytes>> {
        let session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let ids = ids.unwrap_or_else(|| session.scene.objects.iter().filter(|o| o.visible).map(|o| o.id).collect());
        if let Some(missing) = ids.iter().find(|&&id| session.scene.get(id).is_none()) {
            return Err(PyKeyError::new_err(format!("no object {missing}")));
        }
        let bytes = fsc_commands::export_data(&session, file_name, &ids).map_err(|e| PyValueError::new_err(e.message))?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn maps<'py>(&self, py: Python<'py>, id: u32) -> PyResult<Option<MapFiles<'py>>> {
        let session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let object = session.scene.get(id).ok_or_else(|| PyKeyError::new_err(format!("no object {id}")))?;
        Ok(object.maps.as_ref().map(|m| {
            let files = vec![
                ("color.png".to_string(), PyBytes::new(py, &m.color_png)),
                ("roughness_metal.png".to_string(), PyBytes::new(py, &m.mr_png)),
                ("normal.png".to_string(), PyBytes::new(py, &m.normal_png)),
                ("displacement.exr".to_string(), PyBytes::new(py, &m.displacement_exr)),
            ];
            (object.name.clone(), files)
        }))
    }

    /// Face groups (u32 per triangle) and hidden flags (u8 per triangle); each empty when the object has none.
    fn faces<'py>(&self, py: Python<'py>, id: u32) -> PyResult<(Bound<'py, PyBytes>, Bound<'py, PyBytes>)> {
        let session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let object = session.scene.get(id).ok_or_else(|| PyKeyError::new_err(format!("no object {id}")))?;
        Ok((PyBytes::new(py, &object.mesh.groups_bytes()), PyBytes::new(py, &object.mesh.hidden_bytes())))
    }

    fn mesh<'py>(&self, py: Python<'py>, id: u32) -> PyResult<MeshBuffers<'py>> {
        let session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let object = session.scene.get(id).ok_or_else(|| PyKeyError::new_err(format!("no object {id}")))?;
        Ok((
            PyBytes::new(py, &object.mesh.positions_bytes()),
            PyBytes::new(py, &object.mesh.normals_bytes()),
            PyBytes::new(py, &object.mesh.indices_bytes()),
            PyBytes::new(py, &object.mask_bytes()),
            object.revision,
            PyBytes::new(py, &object.mesh.paint_bytes()),
        ))
    }

    fn stroke_begin(&self, id: u32, settings: &str) -> PyResult<()> {
        let parsed: Value = serde_json::from_str(settings).map_err(|e| PyValueError::new_err(format!("settings are not JSON: {e}")))?;
        let mut brush = fsc_commands::parse_brush(&parsed).map_err(|e| PyValueError::new_err(e.message))?;
        let mut session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let Some(object) = session.scene.get(id) else {
            return Err(PyKeyError::new_err(format!("no object {id}")));
        };
        if object.multires.is_some() || !object.layers.list.is_empty() {
            brush.dyntopo = None;
            brush.dyntopo_size = None;
        }
        session.stroke = Some((id, Stroke::new(brush)));
        Ok(())
    }

    fn stroke_sample<'py>(&self, py: Python<'py>, origin: [f32; 3], dir: [f32; 3], pressure: f32) -> PyResult<(bool, Bound<'py, PyBytes>, bool)> {
        let mut guard = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let session = &mut *guard;
        let (id, stroke) = session.stroke.as_mut().ok_or_else(|| PyValueError::new_err("no stroke in progress"))?;
        let object = session.scene.get_mut(*id).ok_or_else(|| PyKeyError::new_err("the stroke's object is gone"))?;
        let hit = stroke.sample_ray(object, Vec3::from_array(origin), Vec3::from_array(dir), pressure);
        let changed = stroke.drain_changed();
        if stroke.take_topology_dirty() {
            object.revision += 1;
            return Ok((hit, PyBytes::new(py, &[]), true));
        }
        Ok((hit, PyBytes::new(py, &update_bytes(object, &changed)), false))
    }

    fn stroke_end(&self) -> PyResult<String> {
        let mut guard = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let session = &mut *guard;
        let Some((id, stroke)) = session.stroke.take() else { return Ok(json!({"ok": true, "result": null}).to_string()) };
        let kind = stroke.settings.kind;
        let dabs = stroke.dabs;
        let Some(object) = session.scene.get_mut(id) else { return Ok(json!({"ok": false, "error": {"code": "not_found", "message": "the stroke's object is gone"}}).to_string()) };
        let result = stroke.finish(object);
        let revision = object.revision;
        let topology = matches!(result, fsc_sculpt::StrokeEdit::Topology { .. });
        let name = BrushKind::NAMES[kind as usize];
        let label = format!("{}{} Stroke", name[..1].to_ascii_uppercase(), name[1..].replace('_', " "));
        let changed = fsc_commands::push_stroke(session, id, label, result);
        Ok(json!({"ok": true, "result": {"id": id, "dabs": dabs, "vertices_changed": changed, "revision": revision, "topology": topology}}).to_string())
    }
}

fn update_bytes(object: &fsc_core::Object, changed: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + changed.len() * 32);
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

fn reply(result: Result<Value, fsc_commands::CommandError>) -> String {
    match result {
        Ok(result) => json!({"ok": true, "result": result}),
        Err(err) => json!({"ok": false, "error": err}),
    }
    .to_string()
}

#[pyfunction]
fn catalogue() -> String {
    fsc_commands::catalogue().to_string()
}

#[pyfunction]
fn version() -> &'static str {
    fsc_core::engine_version()
}

#[pymodule]
fn fsc_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Engine>()?;
    m.add_function(wrap_pyfunction!(catalogue, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
