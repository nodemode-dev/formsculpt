use fsc_core::Mesh;
use serde_json::{json, Value};

use crate::bake_cmds::packed_mesh;
use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn need_uvs(mesh: &Mesh) -> Result<(), CommandError> {
    if mesh.uvs.len() != mesh.indices.len() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)"));
    }
    Ok(())
}

struct Island {
    faces: usize,
    uv_area: f64,
    world_area: f64,
    centroid: [f32; 2],
}

impl Island {
    fn density(&self, texture: f64) -> f64 {
        if self.world_area > 0.0 && self.uv_area > 0.0 {
            texture * (self.uv_area / self.world_area).sqrt()
        } else {
            0.0
        }
    }
}

fn measure(mesh: &Mesh) -> (Vec<u32>, Vec<Island>) {
    let island = fsc_paint::islands(mesh);
    let count = island.iter().copied().max().map_or(0, |m| m as usize + 1);
    let mut out: Vec<Island> = (0..count).map(|_| Island { faces: 0, uv_area: 0.0, world_area: 0.0, centroid: [0.0; 2] }).collect();
    let mut weighted = vec![[0.0f64; 2]; count];
    for (t, &owner) in island.iter().enumerate() {
        let q = [0, 1, 2].map(|k| mesh.uvs[3 * t + k]);
        let p = [0, 1, 2].map(|k| glam::Vec3::from_array(mesh.positions[mesh.indices[3 * t + k] as usize]));
        let uv = f64::from(((q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[2][0] - q[0][0]) * (q[1][1] - q[0][1])).abs()) * 0.5;
        let world = f64::from((p[1] - p[0]).cross(p[2] - p[0]).length()) * 0.5;
        let i = owner as usize;
        out[i].faces += 1;
        out[i].uv_area += uv;
        out[i].world_area += world;
        for c in q {
            weighted[i][0] += f64::from(c[0]) * uv / 3.0;
            weighted[i][1] += f64::from(c[1]) * uv / 3.0;
        }
    }
    for (i, o) in out.iter_mut().enumerate() {
        if o.uv_area > 0.0 {
            o.centroid = [(weighted[i][0] / o.uv_area) as f32, (weighted[i][1] / o.uv_area) as f32];
        }
    }
    (island, out)
}

struct Stats {
    mean: f64,
    min: f64,
    max: f64,
}

fn stats(islands: &[Island], texture: f64) -> Stats {
    let (mut sum, mut weight, mut min, mut max) = (0.0, 0.0, f64::INFINITY, 0.0f64);
    for o in islands {
        let d = o.density(texture);
        if d > 0.0 {
            sum += d * o.world_area;
            weight += o.world_area;
            min = min.min(d);
            max = max.max(d);
        }
    }
    if weight == 0.0 {
        return Stats { mean: 0.0, min: 0.0, max: 0.0 };
    }
    Stats { mean: sum / weight, min, max }
}

fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn stats_json(st: &Stats) -> Value {
    let spread = if st.min > 0.0 { st.max / st.min } else { 0.0 };
    json!({"mean": round(st.mean), "min": round(st.min), "max": round(st.max), "spread": round(spread)})
}

fn texture_arg(args: &Value) -> Result<f64, CommandError> {
    match args.get("texture") {
        None | Some(Value::Null) => Ok(2048.0),
        Some(v) => match v.as_u64() {
            Some(n) if (64..=16384).contains(&n) => Ok(n as f64),
            _ => Err(bad("texture is a whole number of pixels, 64 to 16384")),
        },
    }
}

pub(crate) fn density_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "texture": {"type": "integer", "minimum": 64, "maximum": 16384, "default": 2048, "description": "Texture width and height in pixels that the density is measured for."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Texel density per UV island, in pixels per world unit, with the spread between the sharpest and the softest.
pub(crate) fn uv_density(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let texture = texture_arg(args)?;
    let mesh = &s.scene.get(id).expect("checked").mesh;
    need_uvs(mesh)?;
    let (_, islands) = measure(mesh);
    let st = stats(&islands, texture);
    let listed: Vec<Value> = islands
        .iter()
        .enumerate()
        .map(|(i, o)| json!({"index": i, "faces": o.faces, "density": round(o.density(texture)), "uv_area": o.uv_area, "world_area": round(o.world_area)}))
        .collect();
    let spread = if st.min > 0.0 { st.max / st.min } else { 0.0 };
    Ok(json!({"islands": listed, "mean": round(st.mean), "min": round(st.min), "max": round(st.max), "spread": round(spread), "texture": texture as u64}))
}

pub(crate) fn equalize_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "target": {"type": "number", "exclusiveMinimum": 0, "description": "Density to give every island, in pixels per world unit for a 2048 pixel texture; the area-weighted mean of the islands when omitted."},
            "pack": {"type": "boolean", "default": true, "description": "Pack the islands into the unit square afterwards so they do not overlap. Packing keeps the islands' sizes relative to each other, so it can change all densities by the same factor."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Scales each UV island about its centre so every island has the same texel density. One undo step.
pub(crate) fn uv_equalize(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let pack = match args.get("pack") {
        None | Some(Value::Null) => true,
        Some(v) => v.as_bool().ok_or_else(|| bad("pack is true or false"))?,
    };
    let wanted = match args.get("target") {
        None | Some(Value::Null) => None,
        Some(v) => match v.as_f64() {
            Some(t) if t > 0.0 && t.is_finite() => Some(t),
            _ => return Err(bad("target is a density above 0")),
        },
    };
    let texture = 2048.0;
    let object = s.scene.get_mut(id).expect("checked");
    need_uvs(&object.mesh)?;
    let (island, islands) = measure(&object.mesh);
    let before = stats(&islands, texture);
    let goal = wanted.unwrap_or(before.mean);
    if goal <= 0.0 || goal.is_nan() {
        return Err(CommandError::new("no_area", "no island has both UV area and surface area, so there is no density to equalize"));
    }
    let factors: Vec<f32> = islands.iter().map(|o| {
        let d = o.density(texture);
        if d > 0.0 { (goal / d) as f32 } else { 1.0 }
    }).collect();
    let mut mesh = object.mesh.clone();
    for c in 0..mesh.uvs.len() {
        let i = island[c / 3] as usize;
        let (k, m) = (factors[i], islands[i].centroid);
        let p = mesh.uvs[c];
        mesh.uvs[c] = [m[0] + (p[0] - m[0]) * k, m[1] + (p[1] - m[1]) * k];
    }
    if pack {
        mesh = packed_mesh(&mesh).0;
    }
    let after = stats(&measure(&mesh).1, texture);
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    let parked = object.texture.take();
    object.clear_maps();
    let shown = summary(object);
    s.history.push(Edit::Topology { label: "Equalize UV density".into(), object: id, mesh: old_mesh, mask: old_mask }.with_texture(id, parked));
    Ok(json!({"object": shown, "before": stats_json(&before), "after": stats_json(&after)}))
}
