use base64::Engine as _;
use serde_json::{json, Value};

use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

const ARRAY_LIMIT: usize = 200_000;

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn get_uvs_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "format": {"type": "string", "enum": ["base64", "array"], "default": "base64", "description": "base64 gives little-endian f32 pairs u, v per triangle corner; array gives a list of [u, v] and is refused above 200000 corners."}
        },
        "additionalProperties": false
    })
}

pub(crate) fn get_uvs(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let array = match args.get("format") {
        None | Some(Value::Null) => false,
        Some(v) => match v.as_str() {
            Some("base64") => false,
            Some("array") => true,
            _ => return Err(bad("format is base64 or array")),
        },
    };
    let object = s.scene.get(id).expect("checked");
    let mesh = &object.mesh;
    if mesh.uvs.len() != mesh.indices.len() || mesh.indices.is_empty() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)"));
    }
    let corners = mesh.indices.len();
    let uvs = if array {
        if corners > ARRAY_LIMIT {
            return Err(bad(format!("{corners} corners is too many for format array (limit {ARRAY_LIMIT}); use base64")));
        }
        json!(mesh.uvs.iter().map(|uv| [uv[0], uv[1]]).collect::<Vec<_>>())
    } else {
        let mut bytes = Vec::with_capacity(corners * 8);
        for uv in &mesh.uvs {
            bytes.extend_from_slice(&uv[0].to_le_bytes());
            bytes.extend_from_slice(&uv[1].to_le_bytes());
        }
        json!(base64::engine::general_purpose::STANDARD.encode(&bytes))
    };
    Ok(json!({"object": id, "revision": object.revision, "corners": corners, "uvs": uvs}))
}
