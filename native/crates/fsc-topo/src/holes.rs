use std::collections::HashMap;

use fsc_core::Mesh;
use glam::Vec3;

use crate::edges::boundary_edges;

pub fn fill_holes(mesh: &Mesh) -> (Mesh, usize) {
    let edges = boundary_edges(mesh);
    if edges.is_empty() {
        return (mesh.clone(), 0);
    }
    let mut next: HashMap<u32, u32> = HashMap::with_capacity(edges.len());
    for &(a, b) in &edges {
        next.insert(a, b);
    }
    let mut positions = mesh.positions.clone();
    let mut indices = mesh.indices.clone();
    let mut filled = 0;
    let mut visited: HashMap<u32, bool> = HashMap::new();
    for &(start, _) in &edges {
        if visited.contains_key(&start) {
            continue;
        }
        let mut cycle = vec![start];
        visited.insert(start, true);
        let mut cur = start;
        let mut closed = false;
        for _ in 0..edges.len() {
            let Some(&n) = next.get(&cur) else { break };
            if n == start {
                closed = true;
                break;
            }
            if visited.contains_key(&n) {
                break;
            }
            visited.insert(n, true);
            cycle.push(n);
            cur = n;
        }
        if !closed || cycle.len() < 3 {
            continue;
        }
        let center: Vec3 = cycle.iter().map(|&v| mesh.position(v)).sum::<Vec3>() / cycle.len() as f32;
        let c = positions.len() as u32;
        positions.push(center.to_array());
        for k in 0..cycle.len() {
            indices.extend_from_slice(&[cycle[k], cycle[(k + 1) % cycle.len()], c]);
        }
        filled += 1;
    }
    (Mesh::new(positions, indices), filled)
}
