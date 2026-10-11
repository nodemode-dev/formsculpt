use std::collections::HashMap;
use std::f32::consts::TAU;

use fsc_core::Mesh;
use glam::Vec3;

use crate::edges::{edge_faces, key, Edge};

pub struct Subdivided {
    pub mesh: Mesh,
    pub parents: Vec<(u32, u32)>,
}

pub fn loop_subdivide(mesh: &Mesh, smooth: bool) -> Subdivided {
    let n = mesh.vertex_count();
    let faces_of_edge = edge_faces(mesh);
    let mut neighbors: Vec<Vec<u32>> = vec![Vec::new(); n];
    let mut boundary_nb: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (&(a, b), fs) in &faces_of_edge {
        neighbors[a as usize].push(b);
        neighbors[b as usize].push(a);
        if fs.len() == 1 {
            boundary_nb[a as usize].push(b);
            boundary_nb[b as usize].push(a);
        }
    }
    let p = |i: u32| mesh.position(i);
    let mut positions: Vec<[f32; 3]> = (0..n as u32)
        .map(|v| {
            if !smooth {
                return mesh.positions[v as usize];
            }
            let pv = p(v);
            let bn = &boundary_nb[v as usize];
            if bn.len() == 2 {
                return (pv * 0.75 + (p(bn[0]) + p(bn[1])) * 0.125).to_array();
            }
            if !bn.is_empty() {
                return mesh.positions[v as usize];
            }
            let nb = &neighbors[v as usize];
            let k = nb.len() as f32;
            if k < 3.0 {
                return mesh.positions[v as usize];
            }
            let beta = (0.625 - (0.375 + 0.25 * (TAU / k).cos()).powi(2)) / k;
            let sum: Vec3 = nb.iter().map(|&w| p(w)).sum();
            (pv * (1.0 - k * beta) + sum * beta).to_array()
        })
        .collect();
    let mut parents: Vec<(u32, u32)> = (0..n as u32).map(|v| (v, v)).collect();
    let mut mid: HashMap<Edge, u32> = HashMap::with_capacity(faces_of_edge.len());
    for (&(a, b), fs) in &faces_of_edge {
        let pos = if smooth && fs.len() == 2 {
            let opp = |t: u32| -> u32 {
                let [x, y, z] = mesh.triangle(t as usize);
                [x, y, z].into_iter().find(|&w| w != a && w != b).unwrap_or(a)
            };
            (p(a) + p(b)) * 0.375 + (p(opp(fs[0])) + p(opp(fs[1]))) * 0.125
        } else {
            (p(a) + p(b)) * 0.5
        };
        mid.insert((a, b), positions.len() as u32);
        positions.push(pos.to_array());
        parents.push((a, b));
    }
    let mut indices = Vec::with_capacity(mesh.indices.len() * 4);
    for t in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(t);
        let (ab, bc, ca) = (mid[&key(a, b)], mid[&key(b, c)], mid[&key(c, a)]);
        indices.extend_from_slice(&[a, ab, ca, b, bc, ab, c, ca, bc, ab, bc, ca]);
    }
    Subdivided { mesh: Mesh::new(positions, indices), parents }
}
