//! A rig from the skeleton a base mesh was built from, posed joint by joint with automatic weights, and groom tools
//! that comb, cut and curl grown fibers by reshaping their guide curves.

use fsc_core::time::Instant;
use fsc_core::Mesh;
use glam::{Quat, Vec3};
use serde_json::{json, Value};

use crate::history::Edit;
use crate::object_cmds::set_positions;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn vec3(v: &Value) -> Option<Vec3> {
    let a = v.as_array()?;
    if a.len() < 3 {
        return None;
    }
    Some(Vec3::new(a[0].as_f64()? as f32, a[1].as_f64()? as f32, a[2].as_f64()? as f32))
}

fn arg_vec3(args: &Value, key: &str) -> Result<Option<Vec3>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => vec3(v).map(Some).ok_or_else(|| bad(format!("{key} must be [x, y, z]"))),
    }
}

/// A fingerprint of a mesh's positions (FNV-1a over their bits), to tell which recorded rig or groom state an undone
/// mesh belongs to.
pub(crate) fn fingerprint(mesh: &Mesh) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in &mesh.positions {
        for x in p {
            h ^= x.to_bits() as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

/// What an object was built from (skeleton_mesh or grow_fibers), as of its current mesh: the latest record, or an
/// earlier one when undo brought back the mesh it belongs to. Returns the record and the record list it came from.
fn made(s: &Session, id: u32, kind: &str) -> Result<(Value, Vec<Value>), CommandError> {
    let object = s.scene.get(id).expect("checked");
    let top: Value = object.primitive.as_ref().and_then(|(a, _)| serde_json::from_str(a).ok()).unwrap_or(Value::Null);
    if top.get("build").and_then(Value::as_str) != Some(kind) {
        let what = if kind == "skeleton" { "skeleton_mesh" } else { "grow_fibers" };
        return Err(CommandError::new("no_rig", format!("this object was not made by {what} in this session, so it has no {}", if kind == "skeleton" { "skeleton to pose" } else { "fibers to groom" })));
    }
    let mut records: Vec<Value> = top.get("past").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut latest = top.clone();
    if let Some(m) = latest.as_object_mut() {
        m.remove("past");
    }
    records.push(latest.clone());
    let now = fingerprint(&object.mesh);
    let chosen = records.iter().rev().find(|r| r.get("fp").and_then(Value::as_str) == Some(now.as_str())).cloned().unwrap_or(latest);
    Ok((chosen, records))
}

/// Keeps a new record (tagged with the mesh it made) on top of the earlier ones, the one it started from tagged with
/// the mesh before; the last eight are kept.
fn store(s: &mut Session, id: u32, mut record: Value, mut records: Vec<Value>, started_from: &Value, before: String) {
    if let Some(last) = records.iter_mut().rev().find(|r| *r == started_from) {
        last["fp"] = json!(before);
    }
    let Some(o) = s.scene.get_mut(id) else { return };
    record["fp"] = json!(fingerprint(&o.mesh));
    let keep = records.len().saturating_sub(8);
    record["past"] = Value::Array(records.split_off(keep));
    let r = o.primitive.as_ref().map(|(_, r)| *r).unwrap_or(0);
    o.primitive = Some((record.to_string(), r));
}

fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-20)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

pub(crate) fn pose_joint_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "An object made by skeleton_mesh (sculpted or not); the active object when omitted."},
            "joint": {"type": "integer", "minimum": 0, "description": "The joint (sphere index) to turn; everything beyond it in the skeleton follows."},
            "point": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "Choose the joint nearest this point instead (a click)."},
            "axis": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "default": [0, 0, 1], "description": "The hinge axis."},
            "angle": {"type": "number", "minimum": -360, "maximum": 360, "description": "Degrees to turn."},
            "softness": {"type": "number", "minimum": 0.05, "maximum": 4, "default": 0.6, "description": "How widely the bend blends across the joint, as a fraction of the joint's radius."}
        },
        "required": ["angle"],
        "additionalProperties": false
    })
}

/// Pose a skeleton-built object by turning one joint: the bones beyond it and the mesh around them follow, weighted
/// automatically by distance to each bone so the bend blends across the joint. Sculpted detail rides along.
pub(crate) fn pose_joint(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let (start, records) = made(s, id, "skeleton")?;
    let before = fingerprint(&s.scene.get(id).expect("checked").mesh);
    let mut rig = start.clone();
    let spheres: Vec<(Vec3, f32)> = rig["spheres"].as_array().map(|a| a.iter().filter_map(|v| Some((vec3(v)?, v.get(3)?.as_f64()? as f32))).collect()).unwrap_or_default();
    let links: Vec<(usize, usize)> = rig["links"].as_array().map(|a| a.iter().filter_map(|l| Some((l.get(0)?.as_u64()? as usize, l.get(1)?.as_u64()? as usize))).collect()).unwrap_or_default();
    if links.is_empty() {
        return Err(CommandError::new("no_rig", "the skeleton has no bones (links) to pose"));
    }
    let joint = match (args.get("joint").and_then(Value::as_u64), arg_vec3(args, "point")?) {
        (Some(j), _) => j as usize,
        (None, Some(p)) => spheres.iter().enumerate().min_by(|a, b| (a.1 .0.distance(p) - a.1 .1).total_cmp(&(b.1 .0.distance(p) - b.1 .1))).map(|(i, _)| i).unwrap_or(0),
        (None, None) => return Err(bad("give joint or point")),
    };
    if joint >= spheres.len() {
        return Err(bad(format!("joint {joint} is past the {} joints", spheres.len())));
    }
    let axis = arg_vec3(args, "axis")?.unwrap_or(Vec3::Z).try_normalize().ok_or_else(|| bad("axis must not be zero"))?;
    let angle = args.get("angle").and_then(Value::as_f64).filter(|a| (-360.0..=360.0).contains(a)).ok_or_else(|| bad("angle is required, -360 to 360"))? as f32;
    let softness = args.get("softness").and_then(Value::as_f64).unwrap_or(0.6).clamp(0.05, 4.0) as f32;
    // Parents from a walk outward from joint 0, so "beyond a joint" means its subtree.
    let n = spheres.len();
    let mut parent = vec![usize::MAX; n];
    let mut seen = vec![false; n];
    let mut queue = std::collections::VecDeque::from([0usize]);
    seen[0] = true;
    while let Some(j) = queue.pop_front() {
        for &(a, b) in &links {
            let other = if a == j { b } else if b == j { a } else { continue };
            if other < n && !seen[other] {
                seen[other] = true;
                parent[other] = j;
                queue.push_back(other);
            }
        }
    }
    let mut moving = vec![false; n];
    for (j, m) in moving.iter_mut().enumerate() {
        let mut k = j;
        while k != usize::MAX {
            if parent[k] == joint {
                *m = true;
                break;
            }
            k = parent[k];
        }
    }
    if !moving.iter().any(|&m| m) {
        return Err(CommandError::new("no_rig", format!("joint {joint} is an end; nothing lies beyond it to turn")));
    }
    // Bones as parent to child; a bone moves when its child is beyond the joint.
    let bones: Vec<(usize, usize)> = (0..n).filter(|&c| parent[c] != usize::MAX).map(|c| (parent[c], c)).collect();
    let pivot = spheres[joint].0;
    let sigma = (spheres[joint].1 * softness).max(1e-4);
    let turn = Quat::from_axis_angle(axis, angle.to_radians());
    let object = s.scene.get(id).expect("checked");
    let started = Instant::now();
    let mut positions = object.mesh.positions.clone();
    let mut moved = 0usize;
    for p in positions.iter_mut() {
        let v = Vec3::from(*p);
        let d: Vec<f32> = bones.iter().map(|&(a, b)| segment_distance(v, spheres[a].0, spheres[b].0) - spheres[a].1.min(spheres[b].1) * 0.5).collect();
        let nearest = d.iter().copied().fold(f32::INFINITY, f32::min);
        let (mut on, mut all) = (0.0f32, 0.0f32);
        for (k, &(_, c)) in bones.iter().enumerate() {
            let w = (-((d[k] - nearest) / sigma).powi(2)).exp();
            all += w;
            if moving[c] {
                on += w;
            }
        }
        let w = if all > 0.0 { on / all } else { 0.0 };
        if w > 1e-4 {
            let q = Quat::IDENTITY.slerp(turn, w);
            *p = (pivot + q * (v - pivot)).to_array();
            moved += 1;
        }
    }
    // The skeleton itself turns too, so the next pose starts from here.
    let list = rig["spheres"].as_array_mut().expect("listed");
    for (j, m) in moving.iter().enumerate() {
        if *m {
            let c = pivot + turn * (spheres[j].0 - pivot);
            list[j][0] = json!(c.x);
            list[j][1] = json!(c.y);
            list[j][2] = json!(c.z);
        }
    }
    let mut reply = set_positions(s, id, "Pose Joint", positions);
    store(s, id, rig, records, &start, before);
    reply["joint"] = json!(joint);
    reply["joints_moved"] = json!(moving.iter().filter(|m| **m).count());
    reply["vertices_moved"] = json!(moved);
    reply["ms"] = json!(started.elapsed().as_millis());
    Ok(reply)
}

pub(crate) fn groom_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "A fibers object from grow_fibers; the active object when omitted."},
            "mode": {"type": "string", "enum": ["comb", "cut", "curl", "lengthen"], "description": "comb bends fibers along the stroke, cut trims them where the stroke passes, curl twists them into spirals, lengthen grows them."},
            "points": {"type": "array", "minItems": 1, "maxItems": 4096, "items": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}, "description": "The stroke as world points over the fibers (comb follows its direction)."},
            "radius": {"type": "number", "exclusiveMinimum": 0, "description": "Reach of the stroke in world units; a third of a fiber's length when omitted."},
            "strength": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.6}
        },
        "required": ["mode", "points"],
        "additionalProperties": false
    })
}

/// Tubes for every guide curve, in one mesh, painted as given.
pub(crate) fn fiber_mesh(curves: &[Vec<(Vec3, f32)>], color: Option<[f32; 3]>) -> Mesh {
    let (mut positions, mut indices) = (Vec::new(), Vec::new());
    for pts in curves {
        if let Ok(tube) = fsc_topo::tube(pts, 4, 0, false) {
            let base = positions.len() as u32;
            positions.extend_from_slice(&tube.positions);
            indices.extend(tube.indices.iter().map(|i| i + base));
        }
    }
    let mut out = Mesh::new(positions, indices);
    if let Some(c) = color {
        let mut paint = fsc_core::DEFAULT_PAINT;
        paint[..3].copy_from_slice(&c);
        out.paint = vec![paint; out.vertex_count()];
    }
    out
}

pub(crate) fn fiber_record(curves: &[Vec<(Vec3, f32)>], color: Option<[f32; 3]>) -> Value {
    let round = |x: f32| (x * 1e5).round() / 1e5;
    json!({
        "build": "fibers",
        "color": color,
        "curves": curves.iter().map(|c| c.iter().map(|(p, r)| [round(p.x), round(p.y), round(p.z), round(*r)]).collect::<Vec<_>>()).collect::<Vec<_>>()
    })
}

/// Groom grown fibers: comb, cut, curl or lengthen them where a stroke passes, by reshaping each fiber's guide curve
/// (segment lengths kept except when cutting or lengthening) and remaking the tubes. One undo step.
pub(crate) fn groom_fibers(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let (rec, records) = made(s, id, "fibers")?;
    let before = fingerprint(&s.scene.get(id).expect("checked").mesh);
    let started = Instant::now();
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("");
    if !["comb", "cut", "curl", "lengthen"].contains(&mode) {
        return Err(bad("mode must be comb, cut, curl or lengthen"));
    }
    let stroke: Vec<Vec3> = args.get("points").and_then(Value::as_array).map(|a| a.iter().filter_map(vec3).collect()).unwrap_or_default();
    if stroke.is_empty() {
        return Err(bad("points is required"));
    }
    let strength = args.get("strength").and_then(Value::as_f64).unwrap_or(0.6).clamp(0.0, 1.0) as f32;
    let color: Option<[f32; 3]> = rec.get("color").and_then(|c| serde_json::from_value(c.clone()).ok());
    let mut curves: Vec<Vec<(Vec3, f32)>> = rec["curves"].as_array().map(|a| a.iter().map(|c| c.as_array().map(|pts| pts.iter().filter_map(|p| Some((vec3(p)?, p.get(3)?.as_f64()? as f32))).collect()).unwrap_or_default()).collect()).unwrap_or_default();
    let mean_len = curves.iter().map(|c| c.windows(2).map(|w| w[0].0.distance(w[1].0)).sum::<f32>()).sum::<f32>() / curves.len().max(1) as f32;
    let radius = args.get("radius").and_then(Value::as_f64).map(|r| r as f32).unwrap_or(mean_len / 3.0).max(1e-5);
    // Distance to the stroke and its direction there.
    let near = |p: Vec3| -> (f32, Vec3) {
        if stroke.len() == 1 {
            return (p.distance(stroke[0]), Vec3::ZERO);
        }
        let mut best = (f32::INFINITY, Vec3::ZERO);
        for w in stroke.windows(2) {
            let d = segment_distance(p, w[0], w[1]);
            if d < best.0 {
                best = (d, (w[1] - w[0]).normalize_or_zero());
            }
        }
        best
    };
    let falloff = |d: f32| {
        let x = (1.0 - d / radius).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    let mut touched = 0usize;
    for c in curves.iter_mut() {
        if c.len() < 2 {
            continue;
        }
        let mut changed = false;
        let lengths: Vec<f32> = c.windows(2).map(|w| w[0].0.distance(w[1].0)).collect();
        let root_dir = (c[1].0 - c[0].0).normalize_or(Vec3::Y);
        match mode {
            "cut" => {
                // The first point (past the root) the stroke reaches ends the fiber there.
                if let Some(k) = (1..c.len()).find(|&k| near(c[k].0).0 < radius * (1.0 - strength * 0.5)) {
                    let keep = k.max(2);
                    changed = keep < c.len();
                    c.truncate(keep);
                }
            }
            _ => {
                let mut dirs: Vec<Vec3> = c.windows(2).map(|w| (w[1].0 - w[0].0).normalize_or(root_dir)).collect();
                let mut grow = 1.0f32;
                for k in 0..dirs.len() {
                    let (d, along) = near(c[k + 1].0);
                    let w = falloff(d) * strength;
                    if w <= 0.0 || (mode == "comb" && along == Vec3::ZERO) {
                        continue;
                    }
                    changed = true;
                    let along_k = (k + 1) as f32 / dirs.len() as f32;
                    dirs[k] = match mode {
                        "comb" if along != Vec3::ZERO => dirs[k].lerp(along, w).normalize_or(dirs[k]),
                        "curl" => Quat::from_axis_angle(root_dir, w * along_k * std::f32::consts::PI * 1.5) * dirs[k],
                        _ => dirs[k],
                    };
                    if mode == "lengthen" {
                        grow = grow.max(1.0 + w);
                    }
                }
                if changed {
                    let mut at = c[0].0;
                    for k in 0..dirs.len() {
                        at += dirs[k] * lengths[k] * grow;
                        c[k + 1].0 = at;
                    }
                }
            }
        }
        if changed {
            touched += 1;
        }
    }
    if touched == 0 {
        return Err(CommandError::new("nothing_groomed", "the stroke reached no fibers; draw over them or give a larger radius"));
    }
    let mesh = fiber_mesh(&curves, color);
    let record = fiber_record(&curves, color);
    let object = s.scene.get_mut(id).expect("checked");
    let mask = vec![0.0; mesh.vertex_count()];
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    let after = summary(object);
    s.history.push(Edit::Topology { label: format!("Groom ({mode})"), object: id, mesh: old_mesh, mask: old_mask });
    store(s, id, record, records, &rec, before);
    Ok(json!({"object": after, "fibers_groomed": touched, "ms": started.elapsed().as_millis()}))
}
