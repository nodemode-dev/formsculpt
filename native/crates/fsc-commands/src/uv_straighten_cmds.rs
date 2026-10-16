use std::collections::HashSet;

use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::uv_relax_cmds::uv_relax;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn uv_straighten_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "path": {"type": "array", "items": {"type": "integer", "minimum": 0}, "minItems": 2, "description": "Mesh vertex indices forming a chain of mesh edges inside one UV island, in order."},
            "mode": {"type": "string", "enum": ["line", "axis"], "default": "line", "description": "line puts the path on the straight UV segment from its first to its last point; axis snaps that segment to horizontal or vertical, keeps the first point fixed and scales the length to the island's mean UV to 3D ratio."},
            "relax": {"type": "integer", "minimum": 0, "maximum": 500, "default": 20, "description": "Conformal relax passes afterwards, with the path and the island border held, so the island interior follows the straightened path."}
        },
        "required": ["path"],
        "additionalProperties": false
    })
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f64 {
    let d = [f64::from(a[0] - b[0]), f64::from(a[1] - b[1]), f64::from(a[2] - b[2])];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

fn round3(v: f32) -> f64 {
    (f64::from(v) * 1000.0).round() / 1000.0
}

/// Places a chain of mesh vertices in an island on a straight UV line, then relaxes the interior. One undo step.
pub(crate) fn uv_straighten(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let axis = match args.get("mode") {
        None | Some(Value::Null) => false,
        Some(v) => match v.as_str() {
            Some("line") => false,
            Some("axis") => true,
            _ => return Err(bad("mode is line or axis")),
        },
    };
    let relax = match args.get("relax") {
        None | Some(Value::Null) => 20,
        Some(v) => match v.as_u64() {
            Some(n) if n <= 500 => n as usize,
            _ => return Err(bad("relax is a whole number from 0 to 500")),
        },
    };
    let object = s.scene.get_mut(id).expect("checked");
    if object.mesh.uvs.len() != object.mesh.indices.len() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)"));
    }
    let verts = object.mesh.positions.len();
    let path: Vec<u32> = match args.get("path") {
        Some(Value::Array(list)) if list.len() >= 2 => {
            let mut out = Vec::with_capacity(list.len());
            for v in list {
                match v.as_u64() {
                    Some(i) if (i as usize) < verts => out.push(i as u32),
                    _ => return Err(bad(format!("path holds vertex indices from 0 to {}", verts.saturating_sub(1)))),
                }
            }
            out
        }
        _ => return Err(bad("path is an array of two or more vertex indices")),
    };
    if path.iter().collect::<HashSet<_>>().len() != path.len() {
        return Err(bad("path must not visit a vertex twice"));
    }

    let mesh = &object.mesh;
    let island = fsc_paint::islands(mesh);
    let mut common: Option<HashSet<u32>> = None;
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let here: HashSet<u32> = (0..mesh.triangle_count())
            .filter(|&t| {
                let tri = [mesh.indices[3 * t], mesh.indices[3 * t + 1], mesh.indices[3 * t + 2]];
                tri.contains(&a) && tri.contains(&b)
            })
            .map(|t| island[t])
            .collect();
        if here.is_empty() {
            return Err(bad(format!("vertices {a} and {b} are not joined by a mesh edge")));
        }
        common = Some(match common {
            None => here,
            Some(prev) => prev.intersection(&here).copied().collect(),
        });
    }
    let chosen = match common.and_then(|set| set.into_iter().min()) {
        Some(i) => i,
        None => return Err(bad("the path crosses a seam: its points are not all in one UV island")),
    };

    let corner_of = |v: u32| mesh.indices.iter().enumerate().find(|(c, &w)| w == v && island[c / 3] == chosen).map(|(c, _)| c);
    let first = corner_of(path[0]).ok_or_else(|| bad("the first path vertex is not in the island"))?;
    let last = corner_of(path[path.len() - 1]).ok_or_else(|| bad("the last path vertex is not in the island"))?;
    for &v in &path {
        if corner_of(v).is_none() {
            return Err(bad("the path points are not all in one UV island"));
        }
    }
    let (p0, p1) = (mesh.uvs[first], mesh.uvs[last]);

    let mut along = vec![0.0f64];
    for pair in path.windows(2) {
        let step = dist(mesh.positions[pair[0] as usize], mesh.positions[pair[1] as usize]);
        along.push(along[along.len() - 1] + step);
    }
    let total = along[along.len() - 1];
    if total < 1e-12 {
        return Err(bad("the path has no length in 3D"));
    }

    let end = if axis {
        let (mut uv_area, mut area) = (0.0f64, 0.0f64);
        for (t, _) in island.iter().enumerate().filter(|(_, &i)| i == chosen) {
            let c = [3 * t, 3 * t + 1, 3 * t + 2];
            let q = c.map(|k| mesh.uvs[k]);
            uv_area += 0.5 * f64::from((q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[2][0] - q[0][0]) * (q[1][1] - q[0][1])).abs();
            let p = c.map(|k| mesh.positions[mesh.indices[k] as usize]);
            let (u, w) = ([p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]], [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]);
            let x = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
            area += 0.5 * f64::from(x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
        }
        if area < 1e-18 || uv_area < 1e-18 {
            return Err(bad("the island has no area to measure a UV to 3D ratio from"));
        }
        let length = total * (uv_area / area).sqrt();
        let (dx, dy) = (f64::from(p1[0] - p0[0]), f64::from(p1[1] - p0[1]));
        let dir = if dx.abs() >= dy.abs() { [if dx < 0.0 { -1.0 } else { 1.0 }, 0.0] } else { [0.0, if dy < 0.0 { -1.0 } else { 1.0 }] };
        [f64::from(p0[0]) + length * dir[0], f64::from(p0[1]) + length * dir[1]]
    } else {
        [f64::from(p1[0]), f64::from(p1[1])]
    };

    let (before, _) = fsc_paint::stretch(mesh, &mesh.uvs);
    let mut uvs = mesh.uvs.clone();
    for (k, &v) in path.iter().enumerate() {
        let f = along[k] / total;
        let goal = [
            (f64::from(p0[0]) + f * (end[0] - f64::from(p0[0]))) as f32,
            (f64::from(p0[1]) + f * (end[1] - f64::from(p0[1]))) as f32,
        ];
        for (c, &w) in mesh.indices.iter().enumerate() {
            if w == v && island[c / 3] == chosen {
                uvs[c] = goal;
            }
        }
    }

    let mut hold: Vec<u32> = path.clone();
    hold.extend(mesh.indices.iter().enumerate().filter(|(c, _)| island[c / 3] != chosen).map(|(_, &v)| v));
    hold.sort_unstable();
    hold.dedup();

    let original = mesh.clone();
    let mask = object.mask.clone();
    let mut edited = original.clone();
    edited.uvs = uvs;
    object.replace_mesh(edited, mask.clone());
    object.maps = None;

    if relax > 0 {
        let call = json!({"id": id, "iterations": relax, "mode": "conformal", "pinned": hold});
        if let Err(e) = uv_relax(s, &call) {
            let object = s.scene.get_mut(id).expect("checked");
            object.replace_mesh(original, mask);
            return Err(e);
        }
        let finished = s.scene.get_mut(id).expect("checked").mesh.clone();
        let _ = s.history.undo(&mut s.scene);
        let object = s.scene.get_mut(id).expect("checked");
        object.replace_mesh(finished, mask.clone());
        object.maps = None;
    }

    let object = s.scene.get_mut(id).expect("checked");
    let (after, _) = fsc_paint::stretch(&object.mesh, &object.mesh.uvs);
    let result = summary(object);
    s.history.push(Edit::Topology { label: "Straighten UVs".into(), object: id, mesh: original, mask });
    Ok(json!({"object": result, "straightened": path.len(), "distortion_before": round3(before), "distortion_after": round3(after)}))
}
