use std::collections::HashMap;
use std::f32::consts::TAU;

use fsc_core::Mesh;
use glam::Vec3;

use crate::edges::edge_faces;

fn find(uf: &mut [u32], mut c: u32) -> u32 {
    while uf[c as usize] != c {
        uf[c as usize] = uf[uf[c as usize] as usize];
        c = uf[c as usize];
    }
    c
}

fn union(uf: &mut [u32], a: u32, b: u32) {
    let (ra, rb) = (find(uf, a), find(uf, b));
    if ra != rb {
        uf[ra as usize] = rb;
    }
}

fn corner(mesh: &Mesh, f: u32, v: u32) -> u32 {
    let t = mesh.triangle(f as usize);
    3 * f + t.iter().position(|&w| w == v).unwrap_or(0) as u32
}

fn pairs_around(mesh: &Mesh, a: u32, b: u32, faces: &[u32]) -> Option<Vec<(u32, u32)>> {
    let (pa, pb) = (mesh.position(a), mesh.position(b));
    let axis = (pb - pa).normalize_or_zero();
    let mut items: Vec<(f32, u32, Vec3)> = Vec::with_capacity(faces.len());
    let mut u = Vec3::ZERO;
    for &f in faces {
        let t = mesh.triangle(f as usize);
        let c = t.into_iter().find(|&w| w != a && w != b)?;
        let w = mesh.position(c) - pa;
        let w = (w - axis * w.dot(axis)).normalize_or_zero();
        if u == Vec3::ZERO {
            u = w;
        }
        let v = axis.cross(u);
        let theta = w.dot(v).atan2(w.dot(u)).rem_euclid(TAU);
        let n = (mesh.position(t[1]) - mesh.position(t[0])).cross(mesh.position(t[2]) - mesh.position(t[0]));
        items.push((theta, f, n));
    }
    items.sort_by(|x, y| x.0.total_cmp(&y.0));
    let v = axis.cross(u);
    let count = items.len();
    let mut pairs = Vec::with_capacity(count / 2);
    let mut taken = vec![false; count];
    for i in 0..count {
        let j = (i + 1) % count;
        let (t0, t1) = (items[i].0, if j == 0 { items[j].0 + TAU } else { items[j].0 });
        let mid = 0.5 * (t0 + t1);
        let gap = u * mid.cos() + v * mid.sin();
        if items[i].2.dot(gap) < 0.0 && !taken[i] && !taken[j] {
            taken[i] = true;
            taken[j] = true;
            pairs.push((items[i].1, items[j].1));
        }
    }
    taken.iter().all(|&t| t).then_some(pairs)
}

pub fn split_nonmanifold(mesh: &Mesh) -> (Mesh, usize) {
    let edges = edge_faces(mesh);
    if edges.values().all(|f| f.len() <= 2) {
        return (mesh.clone(), 0);
    }
    let mut uf: Vec<u32> = (0..mesh.indices.len() as u32).collect();
    for (&(a, b), faces) in &edges {
        let pairs = match faces.len() {
            2 => vec![(faces[0], faces[1])],
            n if n > 2 && n % 2 == 0 => match pairs_around(mesh, a, b, faces) {
                Some(p) => p,
                None => continue,
            },
            _ => continue,
        };
        for (f, g) in pairs {
            union(&mut uf, corner(mesh, f, a), corner(mesh, g, a));
            union(&mut uf, corner(mesh, f, b), corner(mesh, g, b));
        }
    }
    let mut positions = mesh.positions.clone();
    let mut indices = mesh.indices.clone();
    let mut first: HashMap<u32, u32> = HashMap::new();
    let mut copy: HashMap<u32, u32> = HashMap::new();
    let mut split = 0;
    for c in 0..mesh.indices.len() as u32 {
        let v = mesh.indices[c as usize];
        let root = find(&mut uf, c);
        let owner = *first.entry(v).or_insert(root);
        if owner == root {
            continue;
        }
        let nv = *copy.entry(root).or_insert_with(|| {
            positions.push(mesh.positions[v as usize]);
            split += 1;
            positions.len() as u32 - 1
        });
        indices[c as usize] = nv;
    }
    (Mesh::new(positions, indices), split)
}
