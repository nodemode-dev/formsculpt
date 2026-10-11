use fsc_core::{Mesh, Primitive};
use glam::{Mat3, Quat, Vec3};
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
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
                return Err(bad(format!("{key} values must be numbers")));
            }
            Ok(Some(Vec3::new(v[0], v[1], v[2])))
        }
        Some(_) => Err(bad(format!("{key} is [x, y, z]"))),
    }
}

/// A copy of `mesh` turned by `m` about `pivot`, then moved by `by`. Paint and face groups ride along.
fn moved(mesh: &Mesh, pivot: Vec3, m: Mat3, by: Vec3) -> Mesh {
    let positions = mesh.positions.iter().map(|&p| (m * (Vec3::from(p) - pivot) + pivot + by).to_array()).collect();
    let mut out = Mesh::new(positions, mesh.indices.clone());
    out.paint = mesh.paint.clone();
    if mesh.has_groups() {
        out.groups = mesh.groups.clone();
    }
    out
}

/// Adds an object; with `record` it is its own undo step (an array records one scene step for all its copies).
fn add_object(s: &mut Session, name: &str, mesh: Mesh, record: bool) -> u32 {
    let name = s.scene.unique_name(name);
    let id = s.scene.add(name, mesh);
    if record {
        let index = s.scene.index_of(id).expect("just added");
        s.history.push(Edit::Add { id, index, parked: None });
    }
    id
}

pub(crate) fn array_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object to repeat; the active object when omitted."},
            "count": {"type": "integer", "minimum": 2, "maximum": 200, "default": 4, "description": "Copies in all, the original included."},
            "mode": {"type": "string", "enum": ["linear", "radial"], "default": "linear"},
            "offset": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Linear: the step from one copy to the next, in world units; one bounds width along x when omitted."},
            "axis": {"type": "string", "enum": ["x", "y", "z"], "default": "y", "description": "Radial: the axis the copies turn around (through center)."},
            "center": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Radial: the point the axis passes through; the world origin when omitted."},
            "angle": {"type": "number", "minimum": -360, "maximum": 360, "default": 360, "description": "Radial: the total turn in degrees; 360 spaces the copies evenly around the full circle."},
            "merge": {"type": "boolean", "default": false, "description": "Join the copies with the original into one object instead of adding separate objects."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn array_object(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let count = args.get("count").and_then(Value::as_u64).unwrap_or(4);
    if !(2..=200).contains(&count) {
        return Err(bad("count must be 2 to 200"));
    }
    let merge = args.get("merge").and_then(Value::as_bool).unwrap_or(false);
    let o = s.scene.get(id).expect("checked");
    if merge && o.multires.is_some() {
        return Err(CommandError::new("has_levels", "merging copies would change the vertex count under the subdivision levels; delete the levels first or leave merge off"));
    }
    let b = o.mesh.bounds();
    let mut copies = Vec::new();
    match args.get("mode").and_then(Value::as_str).unwrap_or("linear") {
        "linear" => {
            let step = vec3(args, "offset")?.unwrap_or(Vec3::new(b.extent().x * 1.2, 0.0, 0.0));
            for k in 1..count {
                copies.push(moved(&o.mesh, Vec3::ZERO, Mat3::IDENTITY, step * k as f32));
            }
        }
        "radial" => {
            let axis = match args.get("axis").and_then(Value::as_str).unwrap_or("y") {
                "x" => Vec3::X,
                "y" => Vec3::Y,
                "z" => Vec3::Z,
                other => return Err(bad(format!("axis must be x, y or z, not {other}"))),
            };
            let center = vec3(args, "center")?.unwrap_or(Vec3::ZERO);
            let total = args.get("angle").and_then(Value::as_f64).unwrap_or(360.0) as f32;
            let full = (total.abs() - 360.0).abs() < 1e-3;
            let step = total.to_radians() / if full { count as f32 } else { (count - 1) as f32 };
            for k in 1..count {
                copies.push(moved(&o.mesh, center, Mat3::from_axis_angle(axis, step * k as f32), Vec3::ZERO));
            }
        }
        other => return Err(bad(format!("unknown mode {other}; use linear or radial"))),
    }
    let name = o.name.clone();
    if merge {
        let mut parts: Vec<&Mesh> = vec![&o.mesh];
        parts.extend(copies.iter());
        let mut mesh = fsc_topo::merge(&parts);
        if o.mesh.has_paint() {
            mesh.paint = parts.iter().flat_map(|m| if m.has_paint() { m.paint.clone() } else { vec![fsc_core::DEFAULT_PAINT; m.vertex_count()] }).collect();
        }
        let reply = crate::topo_cmds::apply(s, id, "Array", mesh, Vec::new())?;
        return Ok(json!({"object": reply["object"], "copies": count}));
    }
    s.history.push(Edit::Scene { label: "Array".into(), objects: s.scene.objects.clone(), active: s.scene.active });
    let ids: Vec<u32> = copies.into_iter().enumerate().map(|(k, m)| add_object(s, &format!("{name} {}", k + 2), m, false)).collect();
    s.scene.active = Some(id);
    Ok(json!({"original": id, "added": ids, "copies": count}))
}

pub(crate) fn insert_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Where to place it, usually a point on a surface (pick gives one)."},
            "normal": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "The surface normal there; the new mesh stands along it. Up when omitted."},
            "kind": {"type": "string", "enum": ["sphere", "cube", "cylinder", "cone", "torus"], "default": "sphere", "description": "A primitive to insert, unless source_id is given."},
            "source_id": {"type": "integer", "minimum": 1, "description": "Insert a copy of this object instead (a horn, a bolt, an eye)."},
            "size": {"type": "number", "exclusiveMinimum": 0, "maximum": 10000, "default": 0.2, "description": "The inserted mesh's largest dimension, in world units."},
            "sink": {"type": "number", "minimum": -1, "maximum": 1, "default": 0.25, "description": "How far it sinks into the surface, as a fraction of its height (0 sits on top, 0.5 half buried)."}
        },
        "required": ["point"],
        "additionalProperties": false
    })
}

pub(crate) fn insert_mesh(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let point = vec3(args, "point")?.ok_or_else(|| bad("point is required"))?;
    let normal = vec3(args, "normal")?.unwrap_or(Vec3::Y).normalize_or(Vec3::Y);
    let size = args.get("size").and_then(Value::as_f64).unwrap_or(0.2) as f32;
    let sink = args.get("sink").and_then(Value::as_f64).unwrap_or(0.25) as f32;
    if !(size > 0.0 && size.is_finite()) || !(-1.0..=1.0).contains(&sink) {
        return Err(bad("size must be above 0 and sink -1 to 1"));
    }
    let (source, name) = match args.get("source_id").and_then(Value::as_u64) {
        Some(sid) => {
            let o = s.scene.get(sid as u32).ok_or_else(|| CommandError::new("not_found", format!("no object {sid}")))?;
            (o.mesh.clone(), format!("{} insert", o.name))
        }
        None => {
            let kind = args.get("kind").and_then(Value::as_str).unwrap_or("sphere");
            let p = match kind {
                "sphere" => Primitive::Sphere { radius: 0.5, subdivisions: 4 },
                "cube" => Primitive::Cube { size: 1.0, divisions: 8 },
                "cylinder" => Primitive::Cylinder { radius: 0.5, height: 1.0, segments: 48, rings: 12 },
                "cone" => Primitive::Cone { radius: 0.5, height: 1.0, segments: 48 },
                "torus" => Primitive::Torus { major: 0.35, minor: 0.15, segments: 48, sides: 24 },
                other => return Err(bad(format!("unknown kind {other}"))),
            };
            let mut c = kind.to_string();
            if let Some(f) = c.get_mut(0..1) {
                f.make_ascii_uppercase();
            }
            (p.build(), c)
        }
    };
    // Scale to `size`, stand it on its local y along the normal, and sink it by a fraction of its height.
    let b = source.bounds();
    let scale = size / b.extent().max_element().max(1e-9);
    let rot = Quat::from_rotation_arc(Vec3::Y, normal);
    let base = Vec3::new(b.center().x, b.min.y, b.center().z);
    let height = b.extent().y * scale;
    let place = point - normal * (height * sink);
    let positions = source.positions.iter().map(|&p| (rot * ((Vec3::from(p) - base) * scale) + place).to_array()).collect();
    let mut mesh = Mesh::new(positions, source.indices.clone());
    mesh.paint = source.paint.clone();
    if source.has_groups() {
        mesh.groups = source.groups.clone();
    }
    let id = add_object(s, &name, mesh, true);
    Ok(json!({"object": summary(s.scene.get(id).expect("added")), "active": id}))
}
