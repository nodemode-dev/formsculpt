use std::sync::Mutex;

use fsc_commands::Session;
use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde_json::{json, Value};

type MapFiles<'py> = (String, Vec<(String, Bound<'py, PyBytes>)>);
type SmoothBuffers<'py> = (Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>, u64, Bound<'py, PyBytes>);
type MeshBuffers<'py> =(Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>, u64, Bound<'py, PyBytes>);

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
            let mut files = vec![
                ("color.png".to_string(), PyBytes::new(py, &m.color_png)),
                ("roughness_metal.png".to_string(), PyBytes::new(py, &m.mr_png)),
                ("normal.png".to_string(), PyBytes::new(py, &m.normal_png)),
                ("displacement.exr".to_string(), PyBytes::new(py, &m.displacement_exr)),
            ];
            if !m.emissive_png.is_empty() {
                files.push(("emissive.png".to_string(), PyBytes::new(py, &m.emissive_png)));
            }
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

    /// Dynamic subdivision: the object's mesh Loop-subdivided `levels` times (1 to 3) for display only, the sculpted mesh
    /// unchanged (fsc_topo::smooth_preview). Returns positions, normals, indices, revision and paint (empty if none).
    fn mesh_smooth<'py>(&self, py: Python<'py>, id: u32, levels: u32) -> PyResult<SmoothBuffers<'py>> {
        let session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let object = session.scene.get(id).ok_or_else(|| PyKeyError::new_err(format!("no object {id}")))?;
        let mesh = fsc_topo::smooth_preview(&object.mesh, levels);
        Ok((PyBytes::new(py, &mesh.positions_bytes()), PyBytes::new(py, &mesh.normals_bytes()), PyBytes::new(py, &mesh.indices_bytes()), object.revision, PyBytes::new(py, &mesh.paint_bytes())))
    }

    // Live strokes go through fsc_commands::live, which the browser build (fsc-wasm) shares.
    fn stroke_begin(&self, id: u32, settings: &str) -> PyResult<()> {
        let parsed: Value = serde_json::from_str(settings).map_err(|e| PyValueError::new_err(format!("settings are not JSON: {e}")))?;
        let mut session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        fsc_commands::live::stroke_begin(&mut session, id, &parsed).map_err(|e| if e.code == "not_found" { PyKeyError::new_err(e.message) } else { PyValueError::new_err(e.message) })
    }

    #[pyo3(signature = (origin, dir, pressure, tilt=0.0))]
    fn stroke_sample<'py>(&self, py: Python<'py>, origin: [f32; 3], dir: [f32; 3], pressure: f32, tilt: f32) -> PyResult<(bool, Bound<'py, PyBytes>, bool)> {
        let mut session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let (hit, update, topology) = fsc_commands::live::stroke_sample(&mut session, origin, dir, pressure, tilt).map_err(|e| PyValueError::new_err(e.message))?;
        Ok((hit, PyBytes::new(py, &update), topology))
    }

    fn stroke_end(&self) -> PyResult<String> {
        let mut session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        Ok(fsc_commands::live::stroke_end(&mut session).to_string())
    }
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
