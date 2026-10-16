use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde_json::{json, Value};

use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn no_uvs() -> CommandError {
    CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)")
}

pub(crate) fn uv_pick_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "at": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "A UV point [u, v] (0 to 1, v up) to pick near."},
            "radius": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.02, "description": "Largest UV distance from at to the picked corner."}
        },
        "required": ["at"],
        "additionalProperties": false
    })
}

pub(crate) fn uv_path_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "from": {"type": "integer", "minimum": 0, "description": "Mesh vertex index where the path starts."},
            "to": {"type": "integer", "minimum": 0, "description": "Mesh vertex index where the path ends; not the same as from."},
            "island": {"type": "integer", "minimum": 0, "description": "An island's number as get_uv_layout numbers it. When given, only mesh edges with a triangle in that island are walked, so the path stays inside the island."},
            "closed": {"type": "boolean", "default": false, "description": "Accepted and ignored: from and to must differ, and a loop is made by repeating the first vertex at the end of the path given to uv_cut."}
        },
        "required": ["from", "to"],
        "additionalProperties": false
    })
}

fn at_point(args: &Value) -> Result<[f64; 2], CommandError> {
    match args.get("at") {
        Some(Value::Array(a)) if a.len() == 2 => match (a[0].as_f64(), a[1].as_f64()) {
            (Some(u), Some(v)) => Ok([u, v]),
            _ => Err(bad("at is two numbers")),
        },
        _ => Err(bad("at is a UV point [u, v]")),
    }
}

/// Finds the triangle corner whose UV is nearest a point, within a radius.
pub(crate) fn uv_pick(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let at = at_point(args)?;
    let radius = match args.get("radius") {
        None | Some(Value::Null) => 0.02,
        Some(v) => match v.as_f64() {
            Some(r) if (0.0..=1.0).contains(&r) => r,
            _ => return Err(bad("radius is a number from 0 to 1")),
        },
    };
    let mesh = &s.scene.get(id).expect("checked").mesh;
    if mesh.uvs.len() != mesh.indices.len() {
        return Err(no_uvs());
    }
    let mut best: Option<(usize, f64)> = None;
    for (c, uv) in mesh.uvs.iter().enumerate() {
        let d = ((f64::from(uv[0]) - at[0]).powi(2) + (f64::from(uv[1]) - at[1]).powi(2)).sqrt();
        if d <= radius && best.is_none_or(|(_, b)| d < b) {
            best = Some((c, d));
        }
    }
    match best {
        None => Ok(json!({"vertex": null})),
        Some((c, d)) => {
            let island = fsc_paint::islands(mesh);
            let uv = mesh.uvs[c];
            Ok(json!({"vertex": mesh.indices[c], "uv": [f64::from(uv[0]), f64::from(uv[1])], "island": island[c / 3], "distance": d}))
        }
    }
}

fn index(args: &Value, key: &str, verts: usize) -> Result<u32, CommandError> {
    match args.get(key).and_then(Value::as_u64) {
        Some(i) if (i as usize) < verts => Ok(i as u32),
        _ => Err(bad(format!("{key} is a vertex index from 0 to {}", verts.saturating_sub(1)))),
    }
}

fn edge_len(p: &[[f32; 3]], a: u32, b: u32) -> f64 {
    let (a, b) = (p[a as usize], p[b as usize]);
    let d = [f64::from(a[0] - b[0]), f64::from(a[1] - b[1]), f64::from(a[2] - b[2])];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// Shortest path between two vertices along mesh edges by 3D length, optionally inside one UV island.
pub(crate) fn uv_path(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    if let Some(c) = args.get("closed") {
        if !c.is_null() && !c.is_boolean() {
            return Err(bad("closed is true or false"));
        }
    }
    let mesh = &s.scene.get(id).expect("checked").mesh;
    if mesh.uvs.len() != mesh.indices.len() {
        return Err(no_uvs());
    }
    let verts = mesh.positions.len();
    let from = index(args, "from", verts)?;
    let to = index(args, "to", verts)?;
    if from == to {
        return Err(bad("from and to must be different vertices"));
    }
    let island = fsc_paint::islands(mesh);
    let count = island.iter().copied().max().map_or(0, |m| m as usize + 1);
    let only = match args.get("island") {
        None | Some(Value::Null) => None,
        Some(v) => match v.as_u64() {
            Some(i) if (i as usize) < count => Some(i as u32),
            _ => return Err(bad(format!("island is a number from 0 to {}", count.saturating_sub(1)))),
        },
    };

    let mut adj: Vec<Vec<(u32, f64)>> = vec![Vec::new(); verts];
    for (t, &owner) in island.iter().enumerate() {
        if only.is_some_and(|i| owner != i) {
            continue;
        }
        for k in 0..3 {
            let (a, b) = (mesh.indices[3 * t + k], mesh.indices[3 * t + (k + 1) % 3]);
            let w = edge_len(&mesh.positions, a, b);
            adj[a as usize].push((b, w));
            adj[b as usize].push((a, w));
        }
    }

    let mut dist = vec![f64::INFINITY; verts];
    let mut prev = vec![u32::MAX; verts];
    let mut heap = BinaryHeap::new();
    dist[from as usize] = 0.0;
    heap.push(Reverse((0u64, from)));
    while let Some(Reverse((key, v))) = heap.pop() {
        let d = f64::from_bits(key);
        if d > dist[v as usize] {
            continue;
        }
        if v == to {
            break;
        }
        for &(n, w) in &adj[v as usize] {
            let nd = d + w;
            if nd < dist[n as usize] {
                dist[n as usize] = nd;
                prev[n as usize] = v;
                heap.push(Reverse((nd.to_bits(), n)));
            }
        }
    }
    if !dist[to as usize].is_finite() {
        let msg = match only {
            Some(i) => format!("no chain of mesh edges joins vertices {from} and {to} inside island {i}"),
            None => format!("no chain of mesh edges joins vertices {from} and {to}"),
        };
        return Err(CommandError::new("no_path", msg));
    }
    let mut path = vec![to];
    while *path.last().expect("not empty") != from {
        path.push(prev[*path.last().expect("not empty") as usize]);
    }
    path.reverse();
    Ok(json!({"path": path, "length": dist[to as usize], "edges": path.len() - 1}))
}
