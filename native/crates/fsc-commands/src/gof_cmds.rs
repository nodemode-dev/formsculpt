//! GoF, formsculptor's own hand-off to other apps: one folder every app can watch, a file per object in the format
//! that app reads best, and a manifest (gof.json) that says what each file is. The file is a bridge, so when the other
//! app saves over it bridge_sync brings the edit back into the same object.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn io(msg: impl Into<String>) -> CommandError {
    CommandError::new("io_failed", msg)
}

/// Each target and the file it gets: GLB for Blender, Unreal and FormSurface (all read glTF), FBX for Unity (read
/// without a package), Substance's package of OBJ meshes and maps, and OBJ for any other app.
const TARGETS: &[(&str, &str)] = &[("blender", "glb"), ("unreal", "glb"), ("unity", "fbx"), ("formsurface", "glb"), ("substance", "package"), ("any", "obj")];

pub(crate) fn gof_send_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "target": {"type": "string", "enum": TARGETS.iter().map(|(t, _)| *t).collect::<Vec<_>>(), "description": "The app it is for, which picks the file: blender, unreal and formsurface get GLB, unity FBX, substance a Substance 3D Painter package (OBJ meshes, baked maps, a README; needs UVs), any OBJ."},
            "folder": {"type": "string", "minLength": 1, "description": "The GoF folder; FORMSCULPTOR_GOF when set, otherwise Formsculptor/GoF in the home folder."}
        },
        "required": ["target"],
        "additionalProperties": false
    })
}

/// The GoF folder: the argument, then FORMSCULPTOR_GOF, then Formsculptor/GoF in the home folder.
fn folder(args: &Value) -> Result<PathBuf, CommandError> {
    if let Some(f) = args.get("folder").and_then(Value::as_str).filter(|f| !f.trim().is_empty()) {
        return Ok(PathBuf::from(f));
    }
    if let Some(f) = std::env::var_os("FORMSCULPTOR_GOF").filter(|f| !f.is_empty()) {
        return Ok(PathBuf::from(f));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).filter(|h| !h.is_empty()).ok_or_else(|| bad("no home folder: give folder or set FORMSCULPTOR_GOF"))?;
    Ok(Path::new(&home).join("Formsculptor").join("GoF"))
}

/// Writes the object for the target into the GoF folder, as a bridge for files, and records it in gof.json.
pub(crate) fn gof_send(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let wanted = args.get("target").and_then(Value::as_str).ok_or_else(|| bad("target is required"))?;
    let &(app, kind) = TARGETS.iter().find(|(t, _)| *t == wanted).ok_or_else(|| bad(format!("unknown target {wanted}")))?;
    let dir = folder(args)?;
    std::fs::create_dir_all(&dir).map_err(|e| io(format!("could not make {}: {e}", dir.display())))?;
    let object = s.scene.get(id).expect("checked");
    // FormSurface skips a mesh without UVs (its loader needs them to paint), so a GoF send for it needs them too.
    if app == "formsurface" && (object.mesh.uvs.is_empty() || object.mesh.uvs.len() != object.mesh.indices.len()) {
        return Err(CommandError::new("no_uvs", "FormSurface paints on UVs and skips a mesh without them; run uv_unwrap first"));
    }
    let name = object.name.clone();
    // The same stem the Substance package and substance_sync use, so maps exported back under it are found.
    let stem: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let file = if kind == "package" { format!("{stem}_substance") } else { format!("{stem}.{kind}") };
    let path = dir.join(&file);
    let path_text = path.to_string_lossy().to_string();
    // Units: a scene unit is a metre, as glTF reads one, unless the scene has a real size (mm_per_unit). GLB and OBJ
    // are written in metres (glTF's unit); FBX in centimetres, the unit its UnitScaleFactor 1 declares, which Unity
    // converts to its metre and Unreal uses as its own. The bridge reads an edit back at the inverse.
    let metres = s.scene.mm_per_unit.map(|mm| mm / 1000.0).unwrap_or(1.0);
    let (scale, units) = match kind {
        "fbx" => (metres * 100.0, "cm"),
        "package" => (1.0, "scene"),
        _ => (metres, "m"),
    };
    let written = if kind == "package" {
        crate::sim_cmds::substance_send(s, &json!({"id": id, "folder": path_text}))?
    } else {
        json!({"bytes": crate::file_cmds::bridge_write(s, id, &path_text, scale, false)?})
    };
    let object = s.scene.get(id).expect("checked");
    let sent = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let entry = json!({
        "id": id, "name": name, "file": file, "format": kind, "target": app, "revision": object.revision,
        "vertices": object.mesh.vertex_count(), "triangles": object.mesh.indices.len() / 3, "sent": sent, "returns": kind != "package",
        "units": units, "scale": scale
    });
    // gof.json: the objects in the folder, newest last, one entry per file.
    let manifest_path = dir.join("gof.json");
    let mut objects: Vec<Value> = std::fs::read(&manifest_path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|m| m.get("objects").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    objects.retain(|o| o.get("file").and_then(Value::as_str) != Some(file.as_str()));
    objects.push(entry.clone());
    let manifest = json!({
        "gof": 1, "app": "formsculptor", "version": env!("CARGO_PKG_VERSION"), "up_axis": "y",
        "mm_per_unit": s.scene.mm_per_unit, "objects": objects
    });
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| io(e.to_string()))?;
    std::fs::write(&manifest_path, text).map_err(|e| io(format!("could not write {}: {e}", manifest_path.display())))?;
    Ok(json!({"id": id, "target": app, "folder": dir.to_string_lossy(), "path": path_text, "manifest": manifest_path.to_string_lossy(), "entry": entry, "written": written}))
}
