use std::collections::HashMap;
use std::f32::consts::TAU;

use fsc_core::{Mesh, Stencil};

use crate::edges::{edge_faces, key, Edge};

pub struct Subdivided {
    pub mesh: Mesh,
    pub parents: Vec<(u32, u32)>,
    pub stencil: Stencil,
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
    let mut stencil = Stencil::default();
    let mut row: Vec<(u32, f32)> = Vec::with_capacity(16);
    for v in 0..n as u32 {
        row.clear();
        let bn = &boundary_nb[v as usize];
        let nb = &neighbors[v as usize];
        let k = nb.len() as f32;
        if smooth && bn.len() == 2 {
            row.extend_from_slice(&[(v, 0.75), (bn[0], 0.125), (bn[1], 0.125)]);
        } else if smooth && bn.is_empty() && k >= 3.0 {
            let beta = (0.625 - (0.375 + 0.25 * (TAU / k).cos()).powi(2)) / k;
            row.push((v, 1.0 - k * beta));
            row.extend(nb.iter().map(|&w| (w, beta)));
        } else {
            row.push((v, 1.0));
        }
        stencil.push(&row);
    }
    let mut parents: Vec<(u32, u32)> = (0..n as u32).map(|v| (v, v)).collect();
    let mut mid: HashMap<Edge, u32> = HashMap::with_capacity(faces_of_edge.len());
    for (&(a, b), fs) in &faces_of_edge {
        row.clear();
        if smooth && fs.len() == 2 {
            let opp = |t: u32| -> u32 {
                let [x, y, z] = mesh.triangle(t as usize);
                [x, y, z].into_iter().find(|&w| w != a && w != b).unwrap_or(a)
            };
            row.extend_from_slice(&[(a, 0.375), (b, 0.375), (opp(fs[0]), 0.125), (opp(fs[1]), 0.125)]);
        } else {
            row.extend_from_slice(&[(a, 0.5), (b, 0.5)]);
        }
        mid.insert((a, b), parents.len() as u32);
        stencil.push(&row);
        parents.push((a, b));
    }
    let positions = stencil.apply3(&mesh.positions);
    let mut indices = Vec::with_capacity(mesh.indices.len() * 4);
    for t in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(t);
        let (ab, bc, ca) = (mid[&key(a, b)], mid[&key(b, c)], mid[&key(c, a)]);
        indices.extend_from_slice(&[a, ab, ca, b, bc, ab, c, ca, bc, ab, bc, ca]);
    }
    let mut out = Mesh::new(positions, indices);
    // Each parent triangle became four consecutive children, so face groups and hidden flags carry over exactly.
    if mesh.has_groups() {
        out.groups = mesh.groups.iter().flat_map(|&g| [g; 4]).collect();
    }
    if mesh.has_hidden() {
        out.hidden = mesh.hidden.iter().flat_map(|&h| [h; 4]).collect();
    }
    Subdivided { mesh: out, parents, stencil }
}
