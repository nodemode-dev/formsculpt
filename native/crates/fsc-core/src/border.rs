use std::collections::{BTreeMap, HashMap};

use glam::Vec3;

use crate::mesh::Mesh;

#[derive(Debug, Clone)]
pub struct BorderLoop {
    pub vertices: Vec<u32>,
    pub closed: bool,
    pub length: f32,
}

pub fn faces_in_group(mesh: &Mesh, group: u32) -> Vec<bool> {
    if !mesh.has_groups() {
        return vec![false; mesh.triangle_count()];
    }
    mesh.groups.iter().map(|g| *g == group).collect()
}

pub fn faces_by_mask(mesh: &Mesh, mask: &[f32], threshold: f32) -> Vec<bool> {
    (0..mesh.triangle_count())
        .map(|t| {
            let [a, b, c] = mesh.triangle(t);
            let m = |v: u32| mask.get(v as usize).copied().unwrap_or(0.0);
            (m(a) + m(b) + m(c)) / 3.0 >= threshold
        })
        .collect()
}

fn welded(mesh: &Mesh) -> Vec<u32> {
    let mut seen: HashMap<[i64; 3], u32> = HashMap::new();
    mesh.positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let key = [(p[0] * 1e5).round() as i64, (p[1] * 1e5).round() as i64, (p[2] * 1e5).round() as i64];
            *seen.entry(key).or_insert(i as u32)
        })
        .collect()
}

fn walk(start: u32, adj: &BTreeMap<u32, Vec<(u32, usize)>>, used: &mut [bool]) -> Vec<u32> {
    let mut chain = vec![start];
    let mut here = start;
    while let Some(&(next, edge)) = adj[&here].iter().find(|(_, e)| !used[*e]) {
        used[edge] = true;
        chain.push(next);
        here = next;
        if here == start {
            break;
        }
    }
    chain
}

pub fn border_loops(mesh: &Mesh, faces: &[bool]) -> Vec<BorderLoop> {
    let weld = welded(mesh);
    let mut count: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for (t, inside) in faces.iter().enumerate().take(mesh.triangle_count()) {
        if !inside {
            continue;
        }
        let tri = mesh.triangle(t).map(|v| weld[v as usize]);
        if tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2] {
            continue;
        }
        for k in 0..3 {
            let (a, b) = (tri[k], tri[(k + 1) % 3]);
            *count.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    let edges: Vec<(u32, u32)> = count.into_iter().filter(|(_, n)| *n == 1).map(|(e, _)| e).collect();
    let mut adj: BTreeMap<u32, Vec<(u32, usize)>> = BTreeMap::new();
    for (i, (a, b)) in edges.iter().enumerate() {
        adj.entry(*a).or_default().push((*b, i));
        adj.entry(*b).or_default().push((*a, i));
    }
    let mut used = vec![false; edges.len()];
    let mut chains: Vec<(Vec<u32>, bool)> = Vec::new();
    let ends: Vec<u32> = adj.iter().filter(|(_, e)| e.len() != 2).map(|(v, _)| *v).collect();
    for v in ends {
        while adj[&v].iter().any(|(_, e)| !used[*e]) {
            chains.push((walk(v, &adj, &mut used), false));
        }
    }
    for i in 0..edges.len() {
        if !used[i] {
            let mut chain = walk(edges[i].0, &adj, &mut used);
            let closed = chain.len() > 2 && chain.first() == chain.last();
            if closed {
                chain.pop();
            }
            chains.push((chain, closed));
        }
    }
    let mut out: Vec<BorderLoop> = chains
        .into_iter()
        .map(|(vertices, closed)| {
            let mut length: f32 = vertices.windows(2).map(|w| mesh.position(w[0]).distance(mesh.position(w[1]))).sum();
            if closed {
                length += mesh.position(vertices[vertices.len() - 1]).distance(mesh.position(vertices[0]));
            }
            BorderLoop { vertices, closed, length }
        })
        .collect();
    out.sort_by(|a, b| b.length.total_cmp(&a.length).then(a.vertices[0].cmp(&b.vertices[0])));
    out
}

pub fn resample_border(mesh: &Mesh, border: &BorderLoop, count: usize, offset: f32) -> Vec<Vec3> {
    let n = border.vertices.len();
    let seg_count = if border.closed { n } else { n - 1 };
    let at = |k: usize| border.vertices[k % n];
    let mut cum = vec![0.0f32];
    for k in 0..seg_count {
        cum.push(cum[k] + mesh.position(at(k)).distance(mesh.position(at(k + 1))));
    }
    let total = cum[seg_count];
    let steps = if border.closed { count } else { count - 1 };
    let normal = |v: u32| mesh.normals.get(v as usize).map(|p| Vec3::from_array(*p)).unwrap_or(Vec3::ZERO);
    let mut seg = 0;
    (0..count)
        .map(|s| {
            let want = total * s as f32 / steps as f32;
            while seg + 1 < seg_count && cum[seg + 1] < want {
                seg += 1;
            }
            let span = (cum[seg + 1] - cum[seg]).max(1e-12);
            let t = ((want - cum[seg]) / span).clamp(0.0, 1.0);
            let (a, b) = (at(seg), at(seg + 1));
            let p = mesh.position(a).lerp(mesh.position(b), t);
            if offset == 0.0 {
                return p;
            }
            let up = normal(a).lerp(normal(b), t).try_normalize().unwrap_or(Vec3::ZERO);
            p + up * offset
        })
        .collect()
}
