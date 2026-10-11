use std::collections::HashMap;

use fsc_core::{Mesh, Stencil, Topology};

use crate::edges::key;
use crate::subdivide::loop_subdivide;

pub struct Reconstructed {
    pub lower: Mesh,
    pub stencil: Stencil,
}

fn ring(mesh: &Mesh, topo: &Topology, v: u32) -> (Vec<u32>, bool) {
    let mut next: HashMap<u32, u32> = HashMap::new();
    for &f in topo.faces_of(v) {
        let t = mesh.triangle(f as usize);
        let k = t.iter().position(|&w| w == v).unwrap_or(0);
        next.insert(t[(k + 1) % 3], t[(k + 2) % 3]);
    }
    let start = next.keys().copied().find(|x| !next.values().any(|y| y == x));
    let closed = start.is_none();
    let Some(&first) = start.as_ref().or_else(|| next.keys().min()) else { return (Vec::new(), true) };
    let mut out = vec![first];
    let mut cur = first;
    while let Some(&n) = next.get(&cur) {
        if n == first || out.len() > next.len() {
            break;
        }
        out.push(n);
        cur = n;
    }
    (out, closed)
}

const UNKNOWN: u8 = 0;
const EVEN: u8 = 1;
const ODD: u8 = 2;

fn label_from(mesh: &Mesh, topo: &Topology, seed: u32, label: &mut [u8], ends: &mut [(u32, u32)], touched: &mut Vec<u32>) -> bool {
    let mut queue = vec![seed];
    label[seed as usize] = EVEN;
    touched.push(seed);
    while let Some(e) = queue.pop() {
        for &o in topo.neighbors_of(e) {
            match label[o as usize] {
                EVEN => return false,
                UNKNOWN => {
                    label[o as usize] = ODD;
                    touched.push(o);
                }
                _ => {}
            }
            let (r, closed) = ring(mesh, topo, o);
            let Some(i) = r.iter().position(|&w| w == e) else { return false };
            let other = match (closed, r.len()) {
                (true, 6) => r[(i + 3) % 6],
                (false, 4) if i == 0 => r[3],
                (false, 4) if i == 3 => r[0],
                _ => return false,
            };
            match label[other as usize] {
                ODD => return false,
                UNKNOWN => {
                    label[other as usize] = EVEN;
                    touched.push(other);
                    queue.push(other);
                }
                _ => {}
            }
            ends[o as usize] = (e.min(other), e.max(other));
        }
    }
    true
}

pub fn reconstruct_lower(mesh: &Mesh) -> Result<Reconstructed, String> {
    let not_loop = || "the mesh does not have the regular one-to-four structure of a subdivided mesh, so no lower level can be rebuilt".to_string();
    if mesh.triangle_count() % 4 != 0 || mesh.triangle_count() < 16 {
        return Err(not_loop());
    }
    let topo = Topology::build(mesh);
    let n = mesh.vertex_count();
    let mut label = vec![UNKNOWN; n];
    let mut ends = vec![(u32::MAX, u32::MAX); n];
    for v in 0..n as u32 {
        if label[v as usize] != UNKNOWN || topo.faces_of(v).is_empty() {
            continue;
        }
        let mut seeds = vec![v];
        seeds.extend_from_slice(topo.neighbors_of(v));
        let mut ok = false;
        for s in seeds {
            let mut touched = Vec::new();
            if label_from(mesh, &topo, s, &mut label, &mut ends, &mut touched) {
                ok = true;
                break;
            }
            for t in touched {
                label[t as usize] = UNKNOWN;
            }
        }
        if !ok {
            return Err(not_loop());
        }
    }
    let mut low_of = vec![u32::MAX; n];
    let mut evens = Vec::new();
    for v in 0..n {
        if label[v] == EVEN {
            low_of[v] = evens.len() as u32;
            evens.push(v as u32);
        }
    }
    let mut indices = Vec::with_capacity(mesh.indices.len() / 4);
    for t in 0..mesh.triangle_count() {
        let tri = mesh.triangle(t);
        let even_count = tri.iter().filter(|&&w| label[w as usize] == EVEN).count();
        if even_count > 1 {
            return Err(not_loop());
        }
        if even_count == 1 {
            continue;
        }
        let common = |x: u32, y: u32| -> Option<u32> {
            let (a, b) = ends[x as usize];
            let (c, d) = ends[y as usize];
            [a, b].into_iter().find(|&w| w == c || w == d)
        };
        let corners = [common(tri[2], tri[0]), common(tri[0], tri[1]), common(tri[1], tri[2])];
        for c in corners {
            let c = c.ok_or_else(not_loop)?;
            indices.push(low_of[c as usize]);
        }
    }
    if indices.len() * 4 != mesh.indices.len() {
        return Err(not_loop());
    }
    let start: Vec<[f32; 3]> = evens.iter().map(|&v| mesh.positions[v as usize]).collect();
    let mut lower = Mesh::new(start, indices);
    let out = loop_subdivide(&lower, true);
    if out.mesh.vertex_count() != n {
        return Err(not_loop());
    }
    for _ in 0..12 {
        let prev = lower.positions.clone();
        for (lv, &hv) in evens.iter().enumerate() {
            let (a, b) = (out.stencil.offsets[lv] as usize, out.stencil.offsets[lv + 1] as usize);
            let mut rest = [0.0f32; 3];
            let mut own = 1.0;
            for k in a..b {
                let (s, w) = (out.stencil.sources[k] as usize, out.stencil.weights[k]);
                if s == lv {
                    own = w;
                } else {
                    for c in 0..3 {
                        rest[c] += w * prev[s][c];
                    }
                }
            }
            let h = mesh.positions[hv as usize];
            lower.positions[lv] = [(h[0] - rest[0]) / own, (h[1] - rest[1]) / own, (h[2] - rest[2]) / own];
        }
    }
    lower.recompute_normals();
    let mut odd_of: HashMap<(u32, u32), u32> = HashMap::with_capacity(n - evens.len());
    for v in 0..n {
        if label[v] == ODD {
            let (a, b) = ends[v];
            odd_of.insert(key(low_of[a as usize], low_of[b as usize]), v as u32);
        }
    }
    let mut row_of = vec![u32::MAX; n];
    for (j, &(a, b)) in out.parents.iter().enumerate() {
        let high = if a == b { evens[a as usize] } else { *odd_of.get(&key(a, b)).ok_or_else(not_loop)? };
        row_of[high as usize] = j as u32;
    }
    let mut stencil = Stencil::default();
    let mut row = Vec::new();
    for &j in &row_of {
        if j == u32::MAX {
            return Err(not_loop());
        }
        let (a, b) = (out.stencil.offsets[j as usize] as usize, out.stencil.offsets[j as usize + 1] as usize);
        row.clear();
        row.extend((a..b).map(|k| (out.stencil.sources[k], out.stencil.weights[k])));
        stencil.push(&row);
    }
    Ok(Reconstructed { lower, stencil })
}
