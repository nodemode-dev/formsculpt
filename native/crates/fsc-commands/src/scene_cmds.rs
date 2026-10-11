use fsc_core::{Object, Primitive};
use serde_json::{json, Value};

use crate::history::Edit;
use crate::{CommandError, Session};

pub(crate) fn summary(o: &Object) -> Value {
    let b = o.mesh.bounds();
    json!({
        "id": o.id,
        "name": o.name,
        "visible": o.visible,
        "folder": o.folder,
        "vertices": o.mesh.vertex_count(),
        "triangles": o.mesh.triangle_count(),
        "masked_vertices": o.mask.iter().filter(|&&m| m > 0.5).count(),
        "groups": crate::group_cmds::group_count(&o.mesh),
        "hidden_faces": if o.mesh.has_hidden() { o.mesh.hidden.iter().filter(|&&h| h).count() } else { 0 },
        "revision": o.revision,
        "level": o.levels().0,
        "levels": o.levels().1,
        "layers": o.layers.list.iter().map(|l| json!({"name": l.name, "strength": l.strength})).collect::<Vec<_>>(),
        "active_layer": o.layers.active,
        "paint_layers": o.paint_layers.list.iter().filter(|_| o.paint_layers.valid(o.mesh.vertex_count())).map(|l| json!({"name": l.name, "blend": l.blend.name(), "opacity": l.opacity, "visible": l.visible})).collect::<Vec<_>>(),
        "active_paint_layer": o.paint_layers.active.filter(|_| o.paint_layers.valid(o.mesh.vertex_count())),
        "bounds": {"min": b.min.to_array(), "max": b.max.to_array()},
    })
}

pub(crate) fn get_scene(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    Ok(json!({
        "objects": s.scene.objects.iter().map(summary).collect::<Vec<_>>(),
        "active": s.scene.active,
    }))
}

pub(crate) fn new_scene(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    let removed = s.scene.objects.len();
    if removed > 0 {
        let active = s.scene.active;
        let parked = std::mem::take(&mut s.scene.objects);
        s.scene.active = None;
        s.history.push(Edit::Clear { parked, active });
    }
    Ok(json!({"removed": removed}))
}

pub(crate) fn id_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"id": {"type": "integer", "minimum": 1, "description": "An object id from get_scene."}},
        "required": ["id"],
        "additionalProperties": false
    })
}

pub(crate) fn add_primitive_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": {"type": "string", "enum": Primitive::KINDS},
            "name": {"type": "string", "maxLength": 120},
            "size": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000, "default": 1, "description": "Radius for sphere, uv_sphere, cylinder, cone and the torus ring; edge length for cube and plane."},
            "detail": {"type": "integer", "minimum": 0, "maximum": 2048, "description": "Subdivisions for sphere (0 to 9, default 5), divisions for cube and plane (default 16), segments for the round kinds (default 64)."},
            "height": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000, "default": 2, "description": "Cylinder and cone height."},
            "thickness": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000, "default": 0.3, "description": "Torus tube radius."}
        },
        "required": ["kind"],
        "additionalProperties": false
    })
}

fn num(args: &Value, key: &str, default: f64, lo: f64, hi: f64) -> Result<f32, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default as f32),
        Some(v) => {
            let x = v.as_f64().ok_or_else(|| CommandError::new("bad_arguments", format!("{key} must be a number")))?;
            if x <= lo || x > hi {
                return Err(CommandError::new("bad_arguments", format!("{key} must be above {lo} and at most {hi}")));
            }
            Ok(x as f32)
        }
    }
}

fn int(args: &Value, key: &str, default: u32, hi: u32) -> Result<u32, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let x = v.as_u64().ok_or_else(|| CommandError::new("bad_arguments", format!("{key} must be a whole number")))?;
            if x > hi as u64 {
                return Err(CommandError::new("bad_arguments", format!("{key} must be at most {hi}")));
            }
            Ok(x as u32)
        }
    }
}

pub(crate) fn add_primitive(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let kind = args.get("kind").and_then(Value::as_str).ok_or_else(|| CommandError::new("bad_arguments", "kind is required"))?;
    let size = num(args, "size", 1.0, 0.0, 1000.0)?;
    let height = num(args, "height", 2.0, 0.0, 1000.0)?;
    let thickness = num(args, "thickness", 0.3, 0.0, 1000.0)?;
    let primitive = match kind {
        "sphere" => Primitive::Sphere { radius: size, subdivisions: int(args, "detail", 5, 9)? },
        "uv_sphere" => {
            let d = int(args, "detail", 64, 2048)?.max(3);
            Primitive::UvSphere { radius: size, segments: d, rings: (d / 2).max(2) }
        }
        "cube" => Primitive::Cube { size, divisions: int(args, "detail", 16, 1024)?.max(1) },
        "cylinder" => {
            let d = int(args, "detail", 64, 2048)?.max(3);
            Primitive::Cylinder { radius: size, height, segments: d, rings: (d / 4).max(1) }
        }
        "cone" => Primitive::Cone { radius: size, height, segments: int(args, "detail", 64, 2048)?.max(3) },
        "torus" => {
            let d = int(args, "detail", 64, 2048)?.max(3);
            Primitive::Torus { major: size, minor: thickness, segments: d, sides: (d / 2).max(3) }
        }
        "plane" => Primitive::Plane { size, divisions: int(args, "detail", 16, 2048)?.max(1) },
        other => return Err(CommandError::new("bad_arguments", format!("unknown kind {other}; use one of {}", Primitive::KINDS.join(", ")))),
    };
    let mesh = primitive.build();
    let base = args.get("name").and_then(Value::as_str).filter(|n| !n.trim().is_empty()).map(str::to_string).unwrap_or_else(|| {
        let mut c = kind.replace('_', " ");
        if let Some(f) = c.get_mut(0..1) {
            f.make_ascii_uppercase();
        }
        c
    });
    let name = s.scene.unique_name(&base);
    let id = s.scene.add(name, mesh);
    let index = s.scene.index_of(id).expect("just added");
    s.history.push(Edit::Add { id, index, parked: None });
    Ok(json!({"object": summary(s.scene.get(id).expect("just added")), "active": id}))
}

fn object_id(args: &Value) -> Result<u32, CommandError> {
    args.get("id")
        .and_then(Value::as_u64)
        .map(|v| v as u32)
        .ok_or_else(|| CommandError::new("bad_arguments", "id is required; get_scene lists object ids"))
}

pub(crate) fn select_object(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = object_id(args)?;
    if s.scene.get(id).is_none() {
        return Err(CommandError::new("not_found", format!("no object {id}; get_scene lists object ids")));
    }
    s.scene.active = Some(id);
    Ok(json!({"active": id}))
}

pub(crate) fn delete_object(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = object_id(args)?;
    let index = s.scene.index_of(id).ok_or_else(|| CommandError::new("not_found", format!("no object {id}; get_scene lists object ids")))?;
    let removed = s.scene.remove(id).expect("found above");
    let reply = json!({"removed": removed.id, "name": removed.name, "active": s.scene.active});
    s.history.push(Edit::Delete { id, index, parked: Some(removed) });
    Ok(reply)
}
