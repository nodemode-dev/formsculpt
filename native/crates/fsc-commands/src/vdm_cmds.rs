use std::sync::Arc;

use fsc_sculpt::{Vdm, VdmPlacement};
use glam::Vec3;
use serde_json::{json, Value};

use crate::object_cmds::set_positions;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn vec3(args: &Value, key: &str) -> Result<Option<Vec3>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() == 3 => {
            let v: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN) as f32).collect();
            if v.iter().any(|x| !x.is_finite()) {
                return Err(bad(format!("{key} must be three numbers")));
            }
            Ok(Some(Vec3::new(v[0], v[1], v[2])))
        }
        Some(_) => Err(bad(format!("{key} must be [x, y, z]"))),
    }
}

fn num(args: &Value, key: &str, default: f32, lo: f32, hi: f32) -> Result<f32, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let x = v.as_f64().ok_or_else(|| bad(format!("{key} must be a number")))? as f32;
            if !(lo..=hi).contains(&x) {
                return Err(bad(format!("{key} must be from {lo} to {hi}")));
            }
            Ok(x)
        }
    }
}

fn names(s: &Session) -> Vec<String> {
    let mut all: Vec<String> = Vdm::BUILTIN.iter().map(|n| n.to_string()).collect();
    let mut own: Vec<String> = s.vdms.keys().cloned().collect();
    own.sort_unstable();
    all.extend(own);
    all
}

pub(crate) fn stamp_schema() -> Value {
    json!({"type": "object", "properties": {
        "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
        "vdm": {"type": "string", "description": "A built-in map (horn, hook, mushroom) or one made with vdm_capture."},
        "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The stamp's centre on the surface (pick gives one)."},
        "normal": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The surface normal there (pick gives it); the stamp rises along it."},
        "radius": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000, "description": "Half the stamp's width in world units; a tenth of the object's largest side when omitted."},
        "angle": {"type": "number", "minimum": -360, "maximum": 360, "default": 0, "description": "Turns the stamp about the normal, in degrees (which way a horn leans)."},
        "strength": {"type": "number", "minimum": -3, "maximum": 3, "default": 1, "description": "Scales every offset; negative presses the shape in."}
    }, "required": ["vdm", "point", "normal"], "additionalProperties": false})
}

/// Stamps a vector displacement map onto the surface as one undo step.
pub(crate) fn vdm_stamp(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let name = args.get("vdm").and_then(Value::as_str).ok_or_else(|| bad("vdm is required"))?;
    let map = match s.vdms.get(name) {
        Some(m) => m.clone(),
        None => Arc::new(Vdm::builtin(name).ok_or_else(|| CommandError::new("not_found", format!("no map {name}; use one of {}", names(s).join(", "))))?),
    };
    let point = vec3(args, "point")?.ok_or_else(|| bad("point is required; pick gives a point on the surface"))?;
    let normal = vec3(args, "normal")?.ok_or_else(|| bad("normal is required; pick gives the normal with the point"))?;
    if normal.length_squared() < 1e-12 {
        return Err(bad("normal must not be zero"));
    }
    let object = s.scene.get(id).expect("checked");
    let b = object.mesh.bounds();
    let radius = num(args, "radius", (b.max - b.min).max_element().max(1e-3) * 0.1, 1e-6, 1000.0)?;
    let angle = num(args, "angle", 0.0, -360.0, 360.0)?.to_radians();
    let strength = num(args, "strength", 1.0, -3.0, 3.0)?;
    let at = VdmPlacement { center: point, normal, radius, angle, strength };
    let (positions, moved) = fsc_sculpt::vdm_stamp(&object.mesh.positions, &object.mask, &map, at);
    if moved == 0 {
        return Err(CommandError::new("nothing_moved", "no vertex lies under the stamp; check point and radius, or the mask"));
    }
    let mut reply = set_positions(s, id, "VDM Stamp", positions);
    reply["vdm"] = json!(name);
    reply["radius"] = json!(radius);
    Ok(reply)
}

pub(crate) fn capture_schema() -> Value {
    json!({"type": "object", "properties": {
        "id": {"type": "integer", "minimum": 1, "description": "A plane made with add_primitive (kind plane) and then sculpted; the active object when omitted."},
        "name": {"type": "string", "minLength": 1, "maxLength": 64, "description": "The name vdm_stamp uses for it; capturing the same name again replaces it."}
    }, "required": ["name"], "additionalProperties": false})
}

/// Captures the sculpting on a plane as a named vector displacement map.
pub(crate) fn vdm_capture(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let name = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad("name is required"))?;
    if Vdm::BUILTIN.contains(&name) {
        return Err(bad(format!("{name} is a built-in map; choose another name")));
    }
    let object = s.scene.get(id).expect("checked");
    let made: Value = object.primitive.as_ref().and_then(|(a, _)| serde_json::from_str(a).ok()).unwrap_or(Value::Null);
    if made.get("kind").and_then(Value::as_str) != Some("plane") {
        return Err(CommandError::new("not_a_plane", "capture from a plane made with add_primitive (kind plane, detail 64 or more) and sculpted in this session"));
    }
    let size = made.get("size").and_then(Value::as_f64).unwrap_or(1.0) as f32;
    let divisions = made.get("detail").and_then(Value::as_u64).unwrap_or(16) as usize;
    if divisions > 1024 {
        return Err(bad("the plane is too fine to capture; use detail 1024 or less"));
    }
    let map = Vdm::capture(&object.mesh.positions, divisions, size).map_err(|e| CommandError::new("not_a_plane", e))?;
    let reach = map.reach();
    if reach < 1e-4 {
        return Err(CommandError::new("nothing_sculpted", "the plane is still flat; sculpt it before capturing"));
    }
    s.vdms.insert(name.to_string(), Arc::new(map));
    Ok(json!({"name": name, "resolution": divisions + 1, "reach": reach, "maps": names(s)}))
}
