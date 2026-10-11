use std::sync::Mutex;

use fsc_commands::Session;
use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde_json::{json, Value};

type MeshBuffers<'py> = (Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>, u64);

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

    fn mesh<'py>(&self, py: Python<'py>, id: u32) -> PyResult<MeshBuffers<'py>> {
        let session = self.session.lock().map_err(|_| PyValueError::new_err("engine session poisoned"))?;
        let object = session.scene.get(id).ok_or_else(|| PyKeyError::new_err(format!("no object {id}")))?;
        Ok((
            PyBytes::new(py, &object.mesh.positions_bytes()),
            PyBytes::new(py, &object.mesh.normals_bytes()),
            PyBytes::new(py, &object.mesh.indices_bytes()),
            object.revision,
        ))
    }
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
