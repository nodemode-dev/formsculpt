use std::collections::HashMap;

use fsc_core::Mesh;

pub type Edge = (u32, u32);

pub fn key(a: u32, b: u32) -> Edge {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

pub fn edge_faces(mesh: &Mesh) -> HashMap<Edge, Vec<u32>> {
    let mut map: HashMap<Edge, Vec<u32>> = HashMap::with_capacity(mesh.indices.len());
    for t in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(t);
        for (u, v) in [(a, b), (b, c), (c, a)] {
            map.entry(key(u, v)).or_default().push(t as u32);
        }
    }
    map
}

pub fn boundary_edges(mesh: &Mesh) -> Vec<(u32, u32)> {
    let mut directed: HashMap<Edge, (u32, u32, u32)> = HashMap::new();
    for t in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(t);
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let e = directed.entry(key(u, v)).or_insert((u, v, 0));
            e.2 += 1;
        }
    }
    directed.into_values().filter(|(_, _, n)| *n == 1).map(|(u, v, _)| (v, u)).collect()
}
