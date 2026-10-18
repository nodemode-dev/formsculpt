//! Import and export options: a scale, the other file's up axis, triangulation on the way out and joining the parts
//! on the way in. formsculptor works y up; a file from a z-up app (Blender is one) is turned on the
//! way in and out, the same turn Blender's glTF exporter makes (x, y, z there is x, z, -y here).

use fsc_core::Mesh;
use fsc_io::Format;
use serde_json::{json, Value};

use crate::CommandError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exchange {
    /// Multiplies every position (after the axis turn on the way in, before it on the way out).
    pub scale: f32,
    /// The other file is z up.
    pub z_up: bool,
    /// Export: write triangles even where the mesh has quads.
    pub triangulate: bool,
    /// Import: join every part of the file into one object.
    pub merge: bool,
}

impl Default for Exchange {
    fn default() -> Self {
        Exchange { scale: 1.0, z_up: false, triangulate: false, merge: false }
    }
}

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

/// The schema properties every import or export command shares; `export` adds triangulate, `import` adds merge.
pub(crate) fn properties(export: bool) -> Value {
    let mut p = json!({
        "scale": {"type": "number", "exclusiveMinimum": 0, "maximum": 100000, "default": 1, "description": "Multiply the size by this (100 turns metres into centimetres)."},
        "up_axis": {"type": "string", "enum": ["y", "z"], "default": "y", "description": if export { "The up axis to write: z for apps that are z up, such as Blender. Applies to OBJ, STL, PLY and 3MF; GLB and glTF are y up by their specification, and FBX and USD declare y up for their readers." } else { "The file's up axis: z stands a file from a z-up app, such as Blender, upright. Not used for GLB and glTF, which are y up by their specification." }}
    });
    if export {
        p["triangulate"] = json!({"type": "boolean", "default": false, "description": "Write triangles where the mesh has quads (OBJ keeps quads otherwise)."});
    } else {
        p["merge"] = json!({"type": "boolean", "default": false, "description": "Join every part of the file into one object."});
    }
    p
}

/// The options from a command's arguments (scale, up_axis, triangulate, merge), refused when out of range.
pub fn parse(args: &Value) -> Result<Exchange, CommandError> {
    let mut ex = Exchange::default();
    if let Some(v) = args.get("scale").filter(|v| !v.is_null()) {
        let s = v.as_f64().ok_or_else(|| bad("scale must be a number"))?;
        if !(s > 0.0 && s <= 100_000.0) {
            return Err(bad("scale must be above 0 and at most 100000"));
        }
        ex.scale = s as f32;
    }
    match args.get("up_axis").and_then(Value::as_str) {
        None | Some("y") => {}
        Some("z") => ex.z_up = true,
        Some(other) => return Err(bad(format!("up_axis is y or z, not {other}"))),
    }
    ex.triangulate = args.get("triangulate").and_then(Value::as_bool).unwrap_or(false);
    ex.merge = args.get("merge").and_then(Value::as_bool).unwrap_or(false);
    Ok(ex)
}

/// Whether the up axis is turned for this format: only the ones that carry no axis of their own.
pub fn turns(format: Format) -> bool {
    matches!(format, Format::Obj | Format::Stl | Format::Ply | Format::ThreeMf)
}

/// formsculptor's axes to the file's: scale, then (x, y, z) to (x, -z, y) for a z-up file.
pub fn outward(p: [f32; 3], scale: f32, z_up: bool) -> [f32; 3] {
    let [x, y, z] = [p[0] * scale, p[1] * scale, p[2] * scale];
    if z_up {
        [x, -z, y]
    } else {
        [x, y, z]
    }
}

/// The file's axes to formsculptor's: (x, y, z) to (x, z, -y) for a z-up file, then scale.
pub fn inward(p: [f32; 3], scale: f32, z_up: bool) -> [f32; 3] {
    let [x, y, z] = if z_up { [p[0], p[2], -p[1]] } else { p };
    [x * scale, y * scale, z * scale]
}

/// A copy of mesh for writing, or None when nothing would change.
pub fn outward_mesh(mesh: &Mesh, scale: f32, z_up: bool, triangulate: bool) -> Option<Mesh> {
    let triangles = triangulate && mesh.quads;
    if scale == 1.0 && !z_up && !triangles {
        return None;
    }
    let mut m = mesh.clone();
    if scale != 1.0 || z_up {
        for p in &mut m.positions {
            *p = outward(*p, scale, z_up);
        }
        m.recompute_normals();
    }
    if triangles {
        m.quads = false;
    }
    Some(m)
}

/// Turns and scales an imported mesh in place.
pub fn inward_mesh(mesh: &mut Mesh, scale: f32, z_up: bool) {
    if scale == 1.0 && !z_up {
        return;
    }
    for p in &mut mesh.positions {
        *p = inward(*p, scale, z_up);
    }
    mesh.recompute_normals();
}

/// Whether an import turns this format: every one but glTF, which its specification fixes as y up.
pub fn turns_in(format: Format) -> bool {
    !matches!(format, Format::Glb | Format::Gltf)
}

/// Reports what was applied, for the command's reply: the scale and the up axis actually used.
pub fn applied(ex: &Exchange, turned: bool) -> Value {
    json!({"scale": ex.scale, "up_axis": if ex.z_up && turned { "z" } else { "y" }})
}
