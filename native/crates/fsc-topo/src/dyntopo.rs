use std::collections::{HashMap, HashSet};

use fsc_core::Mesh;
use glam::Vec3;

use crate::edges::{key, Edge};

pub fn refine_sphere(mesh: &mut Mesh, mask: &mut Vec<f32>, center: Vec3, radius: f32, max_edge: f32, passes: u32) -> usize {
    let max2 = max_edge * max_edge;
    let reach = radius + max_edge;
    let mut total = 0;
    for _ in 0..passes {
        let near: Vec<bool> = mesh.positions.iter().map(|p| Vec3::from_array(*p).distance_squared(center) <= reach * reach).collect();
        let mut wanted: HashMap<Edge, f32> = HashMap::new();
        for t in 0..mesh.triangle_count() {
            let tri = mesh.triangle(t);
            if !tri.iter().any(|&v| near[v as usize]) {
                continue;
            }
            for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                let (pa, pb) = (mesh.position(a), mesh.position(b));
                let l2 = pa.distance_squared(pb);
                if l2 > max2 && ((pa + pb) * 0.5).distance(center) <= radius {
                    wanted.insert(key(a, b), l2);
                }
            }
        }
        if wanted.is_empty() {
            break;
        }
        let mut faces_of: HashMap<Edge, Vec<u32>> = HashMap::with_capacity(wanted.len());
        for t in 0..mesh.triangle_count() {
            let tri = mesh.triangle(t);
            for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                let e = key(a, b);
                if wanted.contains_key(&e) {
                    faces_of.entry(e).or_default().push(t as u32);
                }
            }
        }
        let mut order: Vec<(Edge, f32)> = wanted.into_iter().collect();
        order.sort_by(|x, y| y.1.total_cmp(&x.1));
        let mut touched: HashSet<u32> = HashSet::new();
        let mut splits = 0;
        for ((a, b), _) in order {
            let Some(faces) = faces_of.get(&(a, b)) else { continue };
            if faces.len() > 2 || faces.iter().any(|f| touched.contains(f)) {
                continue;
            }
            let m = mesh.positions.len() as u32;
            let (pa, pb) = (mesh.position(a), mesh.position(b));
            let (na, nb) = (Vec3::from_array(mesh.normals[a as usize]), Vec3::from_array(mesh.normals[b as usize]));
            let n = (na + nb).normalize_or(Vec3::Y);
            let e = pb - pa;
            let bulge = -0.125 * (e.dot(na) - e.dot(nb));
            let pm = (pa + pb) * 0.5 + n * bulge.clamp(-0.25 * e.length(), 0.25 * e.length());
            mesh.positions.push(pm.to_array());
            mesh.normals.push(n.to_array());
            mask.push(0.5 * (mask[a as usize] + mask[b as usize]));
            for &f in faces {
                let base = 3 * f as usize;
                let tri = [mesh.indices[base], mesh.indices[base + 1], mesh.indices[base + 2]];
                let k = (0..3).find(|&k| key(tri[k], tri[(k + 1) % 3]) == (a, b)).expect("edge in face");
                let (u, v, w) = (tri[k], tri[(k + 1) % 3], tri[(k + 2) % 3]);
                mesh.indices[base] = u;
                mesh.indices[base + 1] = m;
                mesh.indices[base + 2] = w;
                mesh.indices.extend_from_slice(&[m, v, w]);
                touched.insert(f);
            }
            splits += 1;
        }
        total += splits;
        if splits == 0 {
            break;
        }
    }
    total
}
