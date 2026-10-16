use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

const MIN_WEIGHT: f64 = 1e-3;

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn uv_relax_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "iterations": {"type": "integer", "minimum": 1, "maximum": 500, "default": 30, "description": "Relaxation passes."},
            "strength": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.5, "description": "How far each point moves toward the average of its neighbours on each pass."},
            "keep_border": {"type": "boolean", "default": true, "description": "Keep the points on an island's border (seams and open edges) where they are."},
            "pinned": {"type": "array", "items": {"type": "integer", "minimum": 0}, "description": "Vertex indices whose UVs stay put."},
            "island": {"type": "integer", "minimum": 0, "description": "A vertex index: relax only the island or islands that vertex belongs to."},
            "mode": {"type": "string", "enum": ["smooth", "conformal"], "default": "conformal", "description": "smooth is a plain average over the mesh connectivity; conformal weights neighbours by the cotangents of the 3D triangles, which brings the UV angles toward the 3D shape."}
        },
        "required": [],
        "additionalProperties": false
    })
}

fn index_of(v: &Value, what: &str, limit: usize) -> Result<usize, CommandError> {
    match v.as_u64() {
        Some(i) if (i as usize) < limit => Ok(i as usize),
        _ => Err(bad(format!("{what} is a vertex index from 0 to {}", limit.saturating_sub(1)))),
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f64; 3] {
    [f64::from(a[0] - b[0]), f64::from(a[1] - b[1]), f64::from(a[2] - b[2])]
}

fn cotangent(p: [f32; 3], q: [f32; 3], apex: [f32; 3]) -> f64 {
    let (a, b) = (sub(q, apex), sub(p, apex));
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let area = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    if area < 1e-18 {
        0.0
    } else {
        dot / area
    }
}

/// Relaxes UVs over the mesh connectivity, with pins and island borders held. One undo step.
pub(crate) fn uv_relax(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let iterations = match args.get("iterations") {
        None | Some(Value::Null) => 30,
        Some(v) => match v.as_u64() {
            Some(n) if (1..=500).contains(&n) => n as usize,
            _ => return Err(bad("iterations is a whole number from 1 to 500")),
        },
    };
    let strength = match args.get("strength") {
        None | Some(Value::Null) => 0.5,
        Some(v) => match v.as_f64() {
            Some(x) if (0.0..=1.0).contains(&x) => x,
            _ => return Err(bad("strength is a number from 0 to 1")),
        },
    };
    let keep_border = match args.get("keep_border") {
        None | Some(Value::Null) => true,
        Some(v) => v.as_bool().ok_or_else(|| bad("keep_border is true or false"))?,
    };
    let conformal = match args.get("mode") {
        None | Some(Value::Null) => true,
        Some(v) => match v.as_str() {
            Some("conformal") => true,
            Some("smooth") => false,
            _ => return Err(bad("mode is smooth or conformal")),
        },
    };
    let object = s.scene.get_mut(id).expect("checked");
    if object.mesh.uvs.len() != object.mesh.indices.len() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)"));
    }
    let verts = object.mesh.positions.len();
    let mut pinned: HashSet<u32> = HashSet::new();
    match args.get("pinned") {
        None | Some(Value::Null) => {}
        Some(Value::Array(list)) => {
            for v in list {
                pinned.insert(index_of(v, "pinned", verts)? as u32);
            }
        }
        Some(_) => return Err(bad("pinned is an array of vertex indices")),
    }
    let only = match args.get("island") {
        None | Some(Value::Null) => None,
        Some(v) => Some(index_of(v, "island", verts)? as u32),
    };

    let mesh = &object.mesh;
    let island = fsc_paint::islands(mesh);
    let mut chosen: Option<HashSet<u32>> = None;
    if let Some(v) = only {
        let set: HashSet<u32> = mesh.indices.iter().enumerate().filter(|(_, &w)| w == v).map(|(c, _)| island[c / 3]).collect();
        if set.is_empty() {
            return Err(bad("that vertex is not used by any face, so it is in no island"));
        }
        chosen = Some(set);
    }

    let mut key: HashMap<(u32, u32, u32, u32), usize> = HashMap::new();
    let mut node_vertex: Vec<u32> = Vec::new();
    let mut node_island: Vec<u32> = Vec::new();
    let mut node_uv: Vec<[f32; 2]> = Vec::new();
    let mut of_corner = Vec::with_capacity(mesh.indices.len());
    for (c, &v) in mesh.indices.iter().enumerate() {
        let w = mesh.uvs[c];
        let isl = island[c / 3];
        let n = *key.entry((v, isl, w[0].to_bits(), w[1].to_bits())).or_insert_with(|| {
            node_vertex.push(v);
            node_island.push(isl);
            node_uv.push(w);
            node_vertex.len() - 1
        });
        of_corner.push(n);
    }
    let count = node_uv.len();

    let mut links: Vec<(usize, usize, f64)> = Vec::new();
    let mut seen: Vec<(usize, usize)> = Vec::new();
    for t in 0..mesh.triangle_count() {
        let corner = [3 * t, 3 * t + 1, 3 * t + 2];
        let p = corner.map(|c| mesh.positions[mesh.indices[c] as usize]);
        for k in 0..3 {
            let (i, j, o) = (k, (k + 1) % 3, (k + 2) % 3);
            let (a, b) = (of_corner[corner[i]], of_corner[corner[j]]);
            if a == b {
                continue;
            }
            let w = 0.5 * cotangent(p[i], p[j], p[o]);
            links.push((a, b, w));
            links.push((b, a, w));
            seen.push((a.min(b), a.max(b)));
        }
    }
    seen.sort_unstable();
    let mut border = vec![false; count];
    let mut run = 0;
    while run < seen.len() {
        let mut end = run;
        while end < seen.len() && seen[end] == seen[run] {
            end += 1;
        }
        if end - run == 1 {
            border[seen[run].0] = true;
            border[seen[run].1] = true;
        }
        run = end;
    }
    links.sort_unstable_by_key(|l| (l.0, l.1));
    let mut start: Vec<usize> = Vec::with_capacity(count + 1);
    let mut merged: Vec<(usize, f64)> = Vec::with_capacity(links.len());
    let mut at = 0;
    for n in 0..count {
        start.push(merged.len());
        while at < links.len() && links[at].0 == n {
            let other = links[at].1;
            let mut sum = 0.0;
            while at < links.len() && links[at].0 == n && links[at].1 == other {
                sum += links[at].2;
                at += 1;
            }
            merged.push((other, if conformal { sum.max(MIN_WEIGHT) } else { 1.0 }));
        }
    }
    start.push(merged.len());

    let free: Vec<bool> = (0..count)
        .map(|n| {
            !pinned.contains(&node_vertex[n])
                && !(keep_border && border[n])
                && chosen.as_ref().is_none_or(|set| set.contains(&node_island[n]))
                && start[n + 1] > start[n]
        })
        .collect();

    let (before, _) = fsc_paint::stretch(mesh, &mesh.uvs);
    let mut cur = node_uv.clone();
    let mut next = cur.clone();
    let k = strength as f32;
    for _ in 0..iterations {
        for n in 0..count {
            if !free[n] {
                continue;
            }
            let (mut sw, mut su, mut sv) = (0.0f64, 0.0f64, 0.0f64);
            for &(m, w) in &merged[start[n]..start[n + 1]] {
                sw += w;
                su += w * f64::from(cur[m][0]);
                sv += w * f64::from(cur[m][1]);
            }
            let goal = [(su / sw) as f32, (sv / sw) as f32];
            next[n] = [cur[n][0] + k * (goal[0] - cur[n][0]), cur[n][1] + k * (goal[1] - cur[n][1])];
        }
        std::mem::swap(&mut cur, &mut next);
    }

    let mut uvs = mesh.uvs.clone();
    let mut moved_vertices: HashSet<u32> = HashSet::new();
    for (c, &n) in of_corner.iter().enumerate() {
        if cur[n] != node_uv[n] {
            uvs[c] = cur[n];
            moved_vertices.insert(node_vertex[n]);
        }
    }
    let (after, _) = fsc_paint::stretch(mesh, &uvs);
    let mut edited = mesh.clone();
    edited.uvs = uvs;
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(edited, mask);
    object.clear_maps();
    let result = summary(object);
    s.history.push(Edit::Topology { label: "Relax UVs".into(), object: id, mesh: old_mesh, mask: old_mask });
    let round = |v: f32| (f64::from(v) * 1000.0).round() / 1000.0;
    Ok(json!({"object": result, "moved": moved_vertices.len(), "distortion_before": round(before), "distortion_after": round(after)}))
}
