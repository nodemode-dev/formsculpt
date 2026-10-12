use fsc_core::time::Instant;

use fsc_core::Mesh;
use fsc_topo::Ball;
use glam::Vec3;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn number(args: &Value, key: &str, default: f64, lo: f64, hi: f64) -> Result<f64, CommandError> {
    let v = args.get(key).and_then(Value::as_f64).unwrap_or(default);
    if !(lo..=hi).contains(&v) {
        return Err(bad(format!("{key} must be {lo} to {hi}")));
    }
    Ok(v)
}

fn floats(v: &Value, what: &str, lens: &[usize]) -> Result<Vec<f32>, CommandError> {
    let a = v.as_array().ok_or_else(|| bad(format!("each {what} must be an array of numbers")))?;
    if !lens.contains(&a.len()) {
        return Err(bad(format!("each {what} needs {} numbers", lens.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(" or "))));
    }
    a.iter()
        .map(|x| x.as_f64().filter(|f| f.is_finite()).map(|f| f as f32).ok_or_else(|| bad(format!("{what} values must be finite numbers"))))
        .collect()
}

fn list<'a>(args: &'a Value, key: &str) -> Result<&'a Vec<Value>, CommandError> {
    args.get(key).and_then(Value::as_array).ok_or_else(|| bad(format!("{key} is required")))
}

fn add(s: &mut Session, args: &Value, base: &str, mesh: Mesh, started: Instant) -> Value {
    let base = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).unwrap_or(base);
    let name = s.scene.unique_name(base);
    let id = s.scene.add(name, mesh);
    let index = s.scene.index_of(id).expect("just added");
    s.history.push(Edit::Add { id, index, parked: None });
    json!({"object": summary(s.scene.get(id).expect("just added")), "active": id, "ms": started.elapsed().as_millis()})
}

const NAME: &str = "Name for the new object; a default when omitted.";

pub(crate) fn skeleton_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "spheres": {"type": "array", "minItems": 1, "maxItems": 512, "items": {"type": "array", "items": {"type": "number"}, "minItems": 4, "maxItems": 4}, "description": "Joints as [x, y, z, radius]."},
            "links": {"type": "array", "maxItems": 1024, "items": {"type": "array", "items": {"type": "integer", "minimum": 0}, "minItems": 2, "maxItems": 2}, "description": "Pairs of sphere indices joined by a tapered limb."},
            "blend": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.3, "description": "How softly limbs merge where they meet, as a fraction of the mean radius."},
            "resolution": {"type": "integer", "minimum": 32, "maximum": 256, "default": 96, "description": "Voxel cells along the longest side."},
            "name": {"type": "string", "description": NAME}
        },
        "required": ["spheres"],
        "additionalProperties": false
    })
}

pub(crate) fn skeleton(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let mesh = skeleton_mesh(args)?;
    let reply = add(s, args, "Skeleton", mesh, started);
    Ok(remember(s, reply, "skeleton", args))
}

fn skeleton_mesh(args: &Value) -> Result<Mesh, CommandError> {
    let mut balls = Vec::new();
    for v in list(args, "spheres")? {
        let f = floats(v, "sphere", &[4])?;
        if f[3] <= 0.0 {
            return Err(bad("sphere radii must be above 0"));
        }
        balls.push(Ball { center: Vec3::new(f[0], f[1], f[2]), radius: f[3] });
    }
    let mut links = Vec::new();
    if let Some(ls) = args.get("links").and_then(Value::as_array) {
        for l in ls {
            let pair = l.as_array().filter(|p| p.len() == 2).ok_or_else(|| bad("each link is [i, j]"))?;
            let (a, b) = (pair[0].as_u64().unwrap_or(u64::MAX) as usize, pair[1].as_u64().unwrap_or(u64::MAX) as usize);
            if a >= balls.len() || b >= balls.len() || a == b {
                return Err(bad(format!("link [{a}, {b}] must join two different spheres of the {}", balls.len())));
            }
            links.push((a, b));
        }
    }
    let blend = number(args, "blend", 0.3, 0.0, 1.0)? as f32;
    let resolution = number(args, "resolution", 96.0, 32.0, 256.0)? as u32;
    fsc_topo::skeleton_mesh(&balls, &links, blend, resolution).map_err(|e| CommandError::new("build_failed", e))
}

/// The sphere of a remembered skeleton nearest a world point.
fn nearest_sphere(spheres: &[Value], p: Vec3) -> Option<usize> {
    spheres
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            let a = v.as_array()?;
            let c = Vec3::new(a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32, a.get(2)?.as_f64()? as f32);
            Some((i, c.distance(p) - a.get(3)?.as_f64()? as f32))
        })
        .min_by(|x, y| x.1.total_cmp(&y.1))
        .map(|(i, _)| i)
}

/// Skeleton editing: add a sphere linked to the nearest one, or drag the sphere nearest a point, then remake.
fn edit_skeleton(merged: &mut Value, args: &Value) -> Result<Value, CommandError> {
    let mut changes = json!({});
    if let Some(add) = args.get("add") {
        let p = floats(add.get("point").unwrap_or(&Value::Null), "add point", &[3])?;
        let p = Vec3::new(p[0], p[1], p[2]);
        let spheres = merged["spheres"].as_array().cloned().unwrap_or_default();
        let near = nearest_sphere(&spheres, p).ok_or_else(|| bad("the skeleton has no spheres"))?;
        let near_r = spheres[near][3].as_f64().unwrap_or(0.1) as f32;
        let r = add.get("radius").and_then(Value::as_f64).map(|r| r as f32).unwrap_or(near_r * 0.8);
        if r <= 0.0 {
            return Err(bad("add radius must be above 0"));
        }
        let index = spheres.len();
        if index >= 512 {
            return Err(bad("a skeleton holds at most 512 spheres"));
        }
        merged["spheres"].as_array_mut().expect("listed").push(json!([p.x, p.y, p.z, r]));
        if !merged["links"].is_array() {
            merged["links"] = json!([]);
        }
        merged["links"].as_array_mut().expect("listed").push(json!([near, index]));
        changes["added"] = json!(index);
        changes["linked_to"] = json!(near);
    }
    if let Some(drag) = args.get("drag") {
        let from = floats(drag.get("from").unwrap_or(&Value::Null), "drag from", &[3])?;
        let to = floats(drag.get("to").unwrap_or(&Value::Null), "drag to", &[3])?;
        let spheres = merged["spheres"].as_array().cloned().unwrap_or_default();
        let i = nearest_sphere(&spheres, Vec3::new(from[0], from[1], from[2])).ok_or_else(|| bad("the skeleton has no spheres"))?;
        let r = spheres[i][3].clone();
        merged["spheres"][i] = json!([to[0], to[1], to[2], r]);
        changes["moved"] = json!(i);
    }
    Ok(changes)
}

pub(crate) fn shadow_box_schema() -> Value {
    let outline = |what: &str| json!({"type": "array", "minItems": 3, "maxItems": 2048, "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}, "description": what});
    json!({
        "type": "object",
        "properties": {
            "front": outline("The outline seen from the front, as [x, y] points in world units (a closed polygon)."),
            "side": outline("The outline seen from the side (the right view), as [z, y] points."),
            "top": outline("The outline seen from above, as [x, z] points."),
            "blend": {"type": "number", "minimum": 0, "maximum": 0.5, "default": 0.05, "description": "Rounds the edges where the outlines meet, as a fraction of the solid's size."},
            "resolution": {"type": "integer", "minimum": 32, "maximum": 256, "default": 128, "description": "Voxel cells along the longest side."},
            "name": {"type": "string", "description": NAME}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Shadow box: a base mesh from two or three outlines, the solid that casts those silhouettes.
pub(crate) fn shadow_box(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let outline = |key: &str| -> Result<Option<Vec<glam::Vec2>>, CommandError> {
        match args.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Array(a)) => a.iter().map(|v| floats(v, key, &[2]).map(|f| glam::Vec2::new(f[0], f[1]))).collect::<Result<Vec<_>, _>>().map(Some),
            Some(_) => Err(bad(format!("{key} is a list of [a, b] points"))),
        }
    };
    let (front, side, top) = (outline("front")?, outline("side")?, outline("top")?);
    let blend = number(args, "blend", 0.05, 0.0, 0.5)? as f32;
    let resolution = number(args, "resolution", 128.0, 32.0, 256.0)? as u32;
    let mesh = fsc_topo::silhouette_mesh(front.as_deref(), side.as_deref(), top.as_deref(), blend, resolution).map_err(|e| CommandError::new("build_failed", e))?;
    Ok(add(s, args, "Shadow Box", mesh, started))
}

pub(crate) fn text_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "text": {"type": "string", "minLength": 1, "maxLength": 160, "description": "The words; a newline starts another line. Letters are drawn as capitals; the font has A-Z, 0-9, space and . , : - + ! ? ' /."},
            "size": {"type": "number", "exclusiveMinimum": 0, "maximum": 100, "default": 1, "description": "Capital height in world units."},
            "weight": {"type": "number", "minimum": 0.04, "maximum": 0.4, "default": 0.14, "description": "Stroke width as a fraction of the size."},
            "depth": {"type": "number", "exclusiveMinimum": 0, "maximum": 100, "description": "How deep the letters are along z; a quarter of the size when omitted."},
            "resolution": {"type": "integer", "minimum": 48, "maximum": 512, "description": "Voxel cells along the longest side; chosen so each stroke is about six cells wide when omitted."},
            "name": {"type": "string", "description": NAME}
        },
        "required": ["text"],
        "additionalProperties": false
    })
}

/// 3D text: words in a rounded stroke font as one watertight solid facing the front.
pub(crate) fn text(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let words = args.get("text").and_then(Value::as_str).filter(|t| !t.trim().is_empty()).ok_or_else(|| bad("text is required"))?;
    if words.chars().count() > 160 {
        return Err(bad("text is at most 160 characters"));
    }
    let size = number(args, "size", 1.0, 1e-4, 100.0)? as f32;
    let weight = number(args, "weight", 0.14, 0.04, 0.4)? as f32;
    let depth = number(args, "depth", (size * 0.25) as f64, 1e-5, 100.0)? as f32;
    let resolution = args.get("resolution").and_then(Value::as_u64).map(|r| r.clamp(48, 512) as u32);
    let mesh = fsc_topo::text_mesh(words, size, weight, depth, resolution).map_err(|e| CommandError::new("build_failed", e))?;
    let base: String = words.lines().next().unwrap_or("Text").chars().take(24).collect();
    let mut named = args.clone();
    if named.get("name").is_none() {
        named["name"] = json!(base.trim());
    }
    Ok(add(s, &named, "Text", mesh, started))
}

pub(crate) fn extrude_shape_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "outlines": {"type": "array", "minItems": 1, "maxItems": 64, "items": {"type": "array", "minItems": 3, "maxItems": 2048, "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}}, "description": "Closed outlines as lists of [x, y] points in world units, seen from the front; an outline inside another cuts a hole (a ring, a letter O, a window)."},
            "depth": {"type": "number", "exclusiveMinimum": 0, "maximum": 100, "description": "Thickness along z; a fifth of the shape's width when omitted."},
            "round": {"type": "number", "minimum": 0, "maximum": 50, "default": 0, "description": "Rounds the edges by this many world units (at most half the depth)."},
            "resolution": {"type": "integer", "minimum": 48, "maximum": 512, "description": "Voxel cells along the longest side; 160 to 512 by the shape's detail when omitted."},
            "name": {"type": "string", "description": NAME}
        },
        "required": ["outlines"],
        "additionalProperties": false
    })
}

/// Vector shape: closed outlines extruded into a watertight solid.
pub(crate) fn extrude_shape(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let mut outlines = Vec::new();
    for o in list(args, "outlines")? {
        let pts = o.as_array().ok_or_else(|| bad("each outline is a list of [x, y] points"))?;
        outlines.push(pts.iter().map(|v| floats(v, "outlines", &[2]).map(|f| glam::Vec2::new(f[0], f[1]))).collect::<Result<Vec<_>, _>>()?);
    }
    let (mut lo, mut hi) = (glam::Vec2::splat(f32::INFINITY), glam::Vec2::splat(f32::NEG_INFINITY));
    for p in outlines.iter().flatten() {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let width = (hi - lo).max_element().max(1e-4);
    let depth = number(args, "depth", (width * 0.2) as f64, 1e-5, 100.0)? as f32;
    let round = number(args, "round", 0.0, 0.0, 50.0)? as f32;
    let resolution = args.get("resolution").and_then(Value::as_u64).map(|r| r.clamp(48, 512) as u32);
    let mesh = fsc_topo::extrude_shape(&outlines, depth, round, resolution).map_err(|e| CommandError::new("build_failed", e))?;
    Ok(add(s, args, "Shape", mesh, started))
}

pub(crate) fn tube_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "points": {"type": "array", "minItems": 2, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 4}, "description": "The curve as [x, y, z] or [x, y, z, radius] points; a point's radius overrides radius."},
            "radius": {"type": "number", "exclusiveMinimum": 0, "default": 0.1, "description": "Tube radius where a point gives none."},
            "segments": {"type": "integer", "minimum": 3, "maximum": 128, "default": 16, "description": "Sides around the tube."},
            "smooth": {"type": "integer", "minimum": 1, "maximum": 32, "default": 4, "description": "Rings per curve span (a smooth spline through the points)."},
            "closed": {"type": "boolean", "default": false, "description": "Join the last point back to the first (a ring) instead of capping the ends."},
            "name": {"type": "string", "description": NAME}
        },
        "required": ["points"],
        "additionalProperties": false
    })
}

pub(crate) fn tube(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let mesh = tube_mesh(args)?;
    let reply = add(s, args, "Tube", mesh, started);
    Ok(remember(s, reply, "tube", args))
}

/// Keeps what a tube or lathe was made from, so edit_build can remake it until anything else changes it.
fn remember(s: &mut Session, reply: Value, kind: &str, args: &Value) -> Value {
    let Some(id) = reply["object"]["id"].as_u64() else { return reply };
    let mut made = args.clone();
    if let Some(m) = made.as_object_mut() {
        m.remove("name");
        m.insert("build".into(), json!(kind));
    }
    if let Some(o) = s.scene.get_mut(id as u32) {
        o.primitive = Some((made.to_string(), o.revision));
    }
    let mut reply = reply;
    if let Some(o) = s.scene.get(id as u32) {
        reply["object"] = summary(o);
    }
    reply
}

fn tube_mesh(args: &Value) -> Result<Mesh, CommandError> {
    let radius = number(args, "radius", 0.1, 1e-6, 1e6)? as f32;
    let mut points = Vec::new();
    for v in list(args, "points")? {
        let f = floats(v, "point", &[3, 4])?;
        let r = f.get(3).copied().unwrap_or(radius);
        if r <= 0.0 {
            return Err(bad("point radii must be above 0"));
        }
        points.push((Vec3::new(f[0], f[1], f[2]), r));
    }
    let segments = number(args, "segments", 16.0, 3.0, 128.0)? as usize;
    let smooth = number(args, "smooth", 4.0, 1.0, 32.0)? as usize;
    let closed = args.get("closed").and_then(Value::as_bool).unwrap_or(false);
    fsc_topo::tube(&points, segments, smooth, closed).map_err(|e| CommandError::new("build_failed", e))
}

pub(crate) fn lathe_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "profile": {"type": "array", "minItems": 2, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}, "description": "The outline as [radius, y] points from bottom to top, spun around the y axis. A radius of 0 closes to a point; open ends are capped flat."},
            "segments": {"type": "integer", "minimum": 3, "maximum": 256, "default": 48, "description": "Steps around the axis."},
            "name": {"type": "string", "description": NAME}
        },
        "required": ["profile"],
        "additionalProperties": false
    })
}

pub(crate) fn lathe(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let started = Instant::now();
    let mesh = lathe_mesh(args)?;
    let reply = add(s, args, "Lathe", mesh, started);
    Ok(remember(s, reply, "lathe", args))
}

fn lathe_mesh(args: &Value) -> Result<Mesh, CommandError> {
    let mut profile = Vec::new();
    for v in list(args, "profile")? {
        let f = floats(v, "profile point", &[2])?;
        if f[0] < 0.0 {
            return Err(bad("profile radii must be 0 or more"));
        }
        profile.push([f[0], f[1]]);
    }
    let segments = number(args, "segments", 48.0, 3.0, 256.0)? as usize;
    fsc_topo::lathe(&profile, segments).map_err(|e| CommandError::new("build_failed", e))
}

pub(crate) fn edit_build_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "A tube, lathe or skeleton; the active object when omitted."},
            "add": {"type": "object", "properties": {"point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}, "radius": {"type": "number", "exclusiveMinimum": 0, "description": "Four fifths of the nearest sphere's when omitted."}}, "required": ["point"], "additionalProperties": false, "description": "Skeleton: add a sphere at point, linked to the nearest sphere (ZSphere style: click beside a joint to grow a limb)."},
            "drag": {"type": "object", "properties": {"from": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}, "to": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}}, "required": ["from", "to"], "additionalProperties": false, "description": "Skeleton: move the sphere nearest from to to."},
            "spheres": {"type": "array", "minItems": 1, "maxItems": 512, "items": {"type": "array", "items": {"type": "number"}, "minItems": 4, "maxItems": 4}, "description": "Skeleton: new joints as [x, y, z, radius]."},
            "links": {"type": "array", "maxItems": 1024, "items": {"type": "array", "items": {"type": "integer", "minimum": 0}, "minItems": 2, "maxItems": 2}, "description": "Skeleton: new links."},
            "blend": {"type": "number", "minimum": 0, "maximum": 1, "description": "Skeleton: how softly limbs merge."},
            "resolution": {"type": "integer", "minimum": 32, "maximum": 256, "description": "Skeleton: voxel cells along the longest side."},
            "points": {"type": "array", "minItems": 2, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 4}, "description": "Tube: a new curve, as [x, y, z] or [x, y, z, radius]."},
            "move": {"type": "array", "maxItems": 4096, "items": {"type": "object", "properties": {"index": {"type": "integer", "minimum": 0}, "to": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 4}}, "required": ["index", "to"], "additionalProperties": false}, "description": "Tube: move single curve points by index (to [x, y, z] or [x, y, z, radius]); lathe: move profile points (to [radius, y]); skeleton: replace spheres (to [x, y, z, radius])."},
            "radius": {"type": "number", "exclusiveMinimum": 0, "maximum": 1000000, "description": "Tube: the radius for points without their own."},
            "segments": {"type": "integer", "minimum": 3, "maximum": 256, "description": "Steps around the tube or the lathe's axis."},
            "smooth": {"type": "integer", "minimum": 1, "maximum": 32, "description": "Tube: curve steps between points."},
            "closed": {"type": "boolean", "description": "Tube: close the curve into a ring."},
            "profile": {"type": "array", "minItems": 2, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}, "description": "Lathe: a new profile, as [radius, y]."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Remakes a tube or lathe from changed arguments while nothing else has changed it since it was made or last edited.
pub(crate) fn edit_build(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = crate::sculpt_cmds::target(s, args)?;
    let object = s.scene.get(id).expect("checked");
    let made: Value = object.primitive.as_ref().and_then(|(a, _)| serde_json::from_str(a).ok()).unwrap_or(Value::Null);
    let Some(kind) = made.get("build").and_then(Value::as_str).map(str::to_string) else {
        return Err(CommandError::new("not_editable", "this object was not made by curve_tube, lathe or skeleton_mesh in this session"));
    };
    if object.primitive.as_ref().map(|(_, r)| *r) != Some(object.revision) || object.multires.is_some() || !object.layers.list.is_empty() {
        return Err(CommandError::new("not_editable", format!("the {kind} has been changed since it was made, so its shape is fixed")));
    }
    let mut merged = made.clone();
    for key in ["points", "radius", "segments", "smooth", "closed", "profile", "spheres", "links", "blend", "resolution"] {
        if let Some(v) = args.get(key) {
            merged[key] = v.clone();
        }
    }
    let mut changes = json!({});
    if kind == "skeleton" {
        changes = edit_skeleton(&mut merged, args)?;
    } else if args.get("add").is_some() || args.get("drag").is_some() {
        return Err(bad("add and drag edit a skeleton"));
    }
    if let Some(moves) = args.get("move").and_then(Value::as_array) {
        let list_key = match kind.as_str() {
            "tube" => "points",
            "skeleton" => "spheres",
            _ => "profile",
        };
        let len = merged[list_key].as_array().map(Vec::len).unwrap_or(0);
        for m in moves {
            let i = m["index"].as_u64().unwrap_or(u64::MAX) as usize;
            if i >= len {
                return Err(bad(format!("move index {i} is past the {len} {list_key}")));
            }
            merged[list_key][i] = m["to"].clone();
        }
    }
    let (mesh, label) = match kind.as_str() {
        "tube" => (tube_mesh(&merged)?, "Tube"),
        "skeleton" => (skeleton_mesh(&merged)?, "Skeleton"),
        _ => (lathe_mesh(&merged)?, "Lathe"),
    };
    let object = s.scene.get_mut(id).expect("checked");
    let mask = vec![0.0; mesh.vertex_count()];
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    object.revision += 1;
    object.primitive = Some((merged.to_string(), object.revision));
    s.history.push(Edit::Topology { label: format!("Edit {label}"), object: id, mesh: old_mesh, mask: old_mask });
    let mut reply = json!({"object": summary(s.scene.get(id).expect("checked"))});
    if kind == "skeleton" {
        reply["spheres"] = merged["spheres"].clone();
        reply["links"] = merged["links"].clone();
        reply["changes"] = changes;
    }
    Ok(reply)
}
