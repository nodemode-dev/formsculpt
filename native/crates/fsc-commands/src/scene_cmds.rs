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
        "instance_of": o.instance.as_ref().map(|i| i.source),
        "primitive": o.primitive.as_ref().filter(|(a, r)| *r == o.revision && o.multires.is_none() && o.layers.list.is_empty() && !a.contains("\"build\":")).and_then(|(a, _)| serde_json::from_str::<Value>(a).ok()),
        // A tube or lathe that edit_build can still remake ("tube" or "lathe"), without its whole curve.
        "editable_build": o.primitive.as_ref().filter(|(_, r)| *r == o.revision && o.multires.is_none() && o.layers.list.is_empty()).and_then(|(a, _)| serde_json::from_str::<Value>(a).ok()).and_then(|v| v.get("build").cloned()),
        "vertices": o.mesh.vertex_count(),
        "triangles": o.mesh.triangle_count(),
        "masked_vertices": o.mask.iter().filter(|&&m| m > 0.5).count(),
        "groups": crate::group_cmds::group_count(&o.mesh),
        "uvs": o.mesh.uvs.len() == o.mesh.indices.len() && !o.mesh.indices.is_empty(),
        "texture_size": o.live_texture().map(|t| t.size),
        "selection_sets": o.selection_sets.len(),
        "hidden_faces": if o.mesh.has_hidden() { o.mesh.hidden.iter().filter(|&&h| h).count() } else { 0 },
        "revision": o.revision,
        "level": o.levels().0,
        "levels": o.levels().1,
        "layers": o.layers.list.iter().map(|l| json!({"name": l.name, "strength": l.strength, "shape_factor": l.shape_factor, "paint_factor": l.paint_factor, "mask_factor": l.mask_factor, "has_paint": l.paint.is_some(), "has_mask": l.mask.is_some()})).collect::<Vec<_>>(),
        "active_layer": o.layers.active,
        "paint_layers": o.paint_layers.list.iter().filter(|_| o.paint_layers.valid(o.mesh.vertex_count())).map(|l| json!({"name": l.name, "blend": l.blend.name(), "channel_blend": l.channel_blend.iter().zip(fsc_core::paint_layers::CHANNEL_NAMES).filter_map(|(b, n)| b.map(|b| (n, b.name()))).collect::<std::collections::BTreeMap<_, _>>(), "opacity": l.opacity, "visible": l.visible})).collect::<Vec<_>>(),
        "active_paint_layer": o.paint_layers.active.filter(|_| o.paint_layers.valid(o.mesh.vertex_count())),
        "bounds": {"min": b.min.to_array(), "max": b.max.to_array()},
        "material": {"subsurface": o.material.subsurface, "subsurface_color": o.material.subsurface_color, "subsurface_radius": o.material.subsurface_radius, "transmission": o.material.transmission, "ior": o.material.ior, "blend": o.material.blend.name(), "opacity": o.material.opacity},
    })
}

pub(crate) fn material_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "subsurface": {"type": "number", "minimum": 0, "maximum": 1, "description": "How much light enters and comes back out tinted: skin 0.3 to 0.6, wax and jade 0.8."},
            "subsurface_color": {"type": "array", "items": {"type": "number", "minimum": 0, "maximum": 1}, "minItems": 3, "maxItems": 3, "description": "The tint light picks up inside, linear RGB (skin is a warm red)."},
            "subsurface_radius": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000, "description": "How far light travels inside, in world units; thin parts such as ears glow when lit from behind."},
            "transmission": {"type": "number", "minimum": 0, "maximum": 1, "description": "How much light passes through, refracted: 1 for clear glass or water."},
            "ior": {"type": "number", "minimum": 1, "maximum": 2.5, "description": "Index of refraction: water 1.33, glass 1.5, diamond 2.4."},
            "blend": {"type": "string", "enum": fsc_core::MaterialBlend::NAMES, "description": "How the surface covers what is behind it: opaque; dithered (see-through by a fine pattern, a share 1 - opacity left out, no sorting needed, keeps shadows); additive (adds its light, hides nothing, casts no shadow: glows, holograms, beams). For glass or water use transmission instead."},
            "opacity": {"type": "number", "minimum": 0, "maximum": 1, "description": "Dithered: the share of the surface drawn. Additive: how much light it adds. Opaque ignores it."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Sets an object's subsurface and transmission, keeping what is not given, as one undo step.
pub(crate) fn set_material(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = crate::sculpt_cmds::target(s, args)?;
    let object = s.scene.get_mut(id).expect("checked");
    let before = object.material;
    let mut m = before;
    let num = |key: &str, lo: f64, hi: f64| -> Result<Option<f32>, CommandError> {
        match args.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v.as_f64().filter(|x| (lo..=hi).contains(x)).map(|x| Some(x as f32)).ok_or_else(|| CommandError::new("bad_arguments", format!("{key} must be from {lo} to {hi}"))),
        }
    };
    if let Some(v) = num("subsurface", 0.0, 1.0)? {
        m.subsurface = v;
    }
    if let Some(v) = num("subsurface_radius", 1e-6, 1000.0)? {
        m.subsurface_radius = v;
    }
    if let Some(v) = num("transmission", 0.0, 1.0)? {
        m.transmission = v;
    }
    if let Some(v) = num("ior", 1.0, 2.5)? {
        m.ior = v;
    }
    if let Some(v) = num("opacity", 0.0, 1.0)? {
        m.opacity = v;
    }
    match args.get("blend") {
        None | Some(Value::Null) => {}
        Some(v) => {
            let name = v.as_str().unwrap_or("");
            m.blend = fsc_core::MaterialBlend::parse(name).ok_or_else(|| CommandError::new("bad_arguments", format!("blend must be one of {}, not {name}", fsc_core::MaterialBlend::NAMES.join(", "))))?;
        }
    }
    if let Some(c) = args.get("subsurface_color") {
        let f: Vec<f32> = c.as_array().map(|a| a.iter().filter_map(Value::as_f64).map(|x| x as f32).collect()).unwrap_or_default();
        if f.len() != 3 || f.iter().any(|x| !(0.0..=1.0).contains(x)) {
            return Err(CommandError::new("bad_arguments", "subsurface_color is [r, g, b] from 0 to 1"));
        }
        m.subsurface_color = [f[0], f[1], f[2]];
    }
    if m == before {
        return Ok(json!({"object": summary(object), "changed": false}));
    }
    object.material = m;
    s.history.push(Edit::Material { object: id, material: before });
    Ok(json!({"object": summary(s.scene.get(id).expect("checked")), "changed": true}))
}

pub(crate) fn get_scene(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    Ok(json!({
        "objects": s.scene.objects.iter().map(|o| {
            let mut v = summary(o);
            v["parent"] = json!(s.scene.parent_of(o.id));
            v
        }).collect::<Vec<_>>(),
        "groups": crate::hierarchy_cmds::groups_report(s),
        "active": s.scene.active,
        // The saved views' names, in order (list_views has the cameras).
        "views": s.scene.views.iter().map(|v| v.name.clone()).collect::<Vec<_>>(),
    }))
}

pub(crate) fn new_scene(s: &mut Session, _: &Value) -> Result<Value, CommandError> {
    let removed = s.scene.objects.len();
    if removed > 0 || !s.scene.groups.is_empty() {
        let active = s.scene.active;
        let parked = std::mem::take(&mut s.scene.objects);
        let groups = std::mem::take(&mut s.scene.groups);
        s.scene.active = None;
        s.history.push(Edit::Clear { parked, active, groups });
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
            "kind": {"type": "string", "enum": Primitive::KINDS.iter().chain(["head"].iter()).collect::<Vec<_>>()},
            "name": {"type": "string", "maxLength": 120},
            "size": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000, "default": 1, "description": "Radius for sphere, uv_sphere, icosahedron, quad_sphere, cylinder, cone and the torus ring; edge length for cube and plane."},
            "detail": {"type": "integer", "minimum": 0, "maximum": 2048, "description": "Subdivisions for sphere (0 to 9, default 5) and icosahedron (0 to 9, default 0: the 20 faces), divisions per side for cube, plane and quad_sphere (default 16) and head (default 40), segments for the round kinds (default 64)."},
            "project": {"type": "boolean", "default": false, "description": "Icosahedron: push every point out onto the sphere (round, as sphere is) instead of keeping the faces flat."},
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
    let (mesh, kind) = primitive_mesh(args)?;
    let base = args.get("name").and_then(Value::as_str).filter(|n| !n.trim().is_empty()).map(str::to_string).unwrap_or_else(|| {
        if kind == "uv_sphere" {
            return "UV sphere".to_string();
        }
        let mut c = kind.replace('_', " ");
        if let Some(f) = c.get_mut(0..1) {
            f.make_ascii_uppercase();
        }
        c
    });
    let name = s.scene.unique_name(&base);
    // The head comes with a layer that turns its masculine form into a feminine one (strength 0 shows the first).
    let (mesh, feminine) = if kind == "head" {
        let (m, delta) = fsc_core::head::head(num(args, "size", 1.0, 0.0, 1000.0)?, int(args, "detail", 40, 256)?.max(4));
        (m, Some(delta))
    } else {
        (mesh, None)
    };
    let id = s.scene.add(name, mesh);
    let index = s.scene.index_of(id).expect("just added");
    s.history.push(Edit::Add { id, index, parked: None });
    let object = s.scene.get_mut(id).expect("just added");
    if let Some(delta) = feminine {
        object.layers.list.push(fsc_core::layers::Layer { name: "Feminine".into(), strength: 0.0, delta, ..Default::default() });
        object.layers.active = None;
    }
    // Open to edit with edit_primitive until anything else changes it.
    object.primitive = Some((args.to_string(), object.revision));
    Ok(json!({"object": summary(s.scene.get(id).expect("just added")), "active": id}))
}

pub(crate) fn edit_primitive_schema() -> Value {
    let mut schema = add_primitive_schema();
    let props = schema["properties"].as_object_mut().expect("object");
    props.remove("kind");
    props.remove("name");
    props.insert("id".into(), json!({"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."}));
    schema["required"] = json!([]);
    schema
}

/// Rebuilds a primitive with new settings while nothing else has changed it since it was made or last edited.
pub(crate) fn edit_primitive(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = crate::sculpt_cmds::target(s, args)?;
    let object = s.scene.get(id).expect("checked");
    let Some((made, revision)) = object.primitive.clone() else {
        return Err(CommandError::new("not_editable", "this object was not made by add_primitive in this session"));
    };
    if revision != object.revision || object.multires.is_some() || !object.layers.list.is_empty() {
        return Err(CommandError::new("not_editable", "the primitive has been changed since it was made, so its shape is fixed"));
    }
    let mut merged: Value = serde_json::from_str(&made).unwrap_or_else(|_| json!({}));
    for key in ["size", "detail", "height", "thickness", "project"] {
        if let Some(v) = args.get(key) {
            merged[key] = v.clone();
        }
    }
    let (mesh, _) = primitive_mesh(&merged)?;
    let object = s.scene.get_mut(id).expect("checked");
    let mask = vec![0.0; mesh.vertex_count()];
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    object.revision += 1;
    object.primitive = Some((merged.to_string(), object.revision));
    s.history.push(Edit::Topology { label: "Edit Primitive".into(), object: id, mesh: old_mesh, mask: old_mask });
    Ok(json!({"object": summary(s.scene.get(id).expect("checked"))}))
}

fn primitive_mesh(args: &Value) -> Result<(fsc_core::Mesh, String), CommandError> {
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
        "icosahedron" => Primitive::Icosahedron { radius: size, subdivisions: int(args, "detail", 0, 9)?, project: args.get("project").and_then(Value::as_bool).unwrap_or(false) },
        "quad_sphere" => Primitive::QuadSphere { radius: size, divisions: int(args, "detail", 16, 1024)?.max(1) },
        // Built with its layer by add_primitive; here only so the kind is known (a placeholder mesh is replaced).
        "head" => Primitive::Icosahedron { radius: size, subdivisions: 0, project: true },
        other => return Err(CommandError::new("bad_arguments", format!("unknown kind {other}; use one of {}", Primitive::KINDS.join(", ")))),
    };
    Ok((primitive.build(), kind.to_string()))
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

pub(crate) fn delete_object_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "An object id from get_scene."},
            "children": {"type": "boolean", "default": false, "description": "Also delete every object and group below it in the hierarchy (Delete+); otherwise they move up to its parent."}
        },
        "required": ["id"],
        "additionalProperties": false
    })
}

pub(crate) fn delete_object(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = object_id(args)?;
    let name = s.scene.get(id).map(|o| o.name.clone()).ok_or_else(|| CommandError::new("not_found", format!("no object {id}; get_scene lists object ids")))?;
    let mut reply = crate::hierarchy_cmds::remove_node(s, id, args.get("children").and_then(Value::as_bool).unwrap_or(false))?;
    reply["name"] = json!(name);
    Ok(reply)
}
