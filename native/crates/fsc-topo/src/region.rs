use std::collections::HashMap;

use fsc_core::{Bvh, Mesh};
use glam::Vec3;

pub fn region_faces(mesh: &Mesh, mask: &[f32]) -> Vec<bool> {
    (0..mesh.triangle_count()).map(|t| mesh.triangle(t).iter().all(|&v| mask[v as usize] >= 0.5)).collect()
}

fn boundary(mesh: &Mesh, inside: &[bool]) -> Vec<(u32, u32)> {
    let mut count: HashMap<(u32, u32), u32> = HashMap::new();
    for t in (0..mesh.triangle_count()).filter(|&t| inside[t]) {
        let tri = mesh.triangle(t);
        for k in 0..3 {
            let (a, b) = (tri[k], tri[(k + 1) % 3]);
            *count.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    let mut out = Vec::new();
    for t in (0..mesh.triangle_count()).filter(|&t| inside[t]) {
        let tri = mesh.triangle(t);
        for k in 0..3 {
            let (a, b) = (tri[k], tri[(k + 1) % 3]);
            if count[&(a.min(b), a.max(b))] == 1 {
                out.push((a, b));
            }
        }
    }
    out
}

pub fn extract_shell(mesh: &Mesh, mask: &[f32], thickness: f32) -> Option<Mesh> {
    let inside = region_faces(mesh, mask);
    if !inside.iter().any(|&x| x) {
        return None;
    }
    let mut local: HashMap<u32, u32> = HashMap::new();
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut base: Vec<u32> = Vec::new();
    for t in (0..mesh.triangle_count()).filter(|&t| inside[t]) {
        for v in mesh.triangle(t) {
            local.entry(v).or_insert_with(|| {
                base.push(v);
                positions.push(mesh.positions[v as usize]);
                positions.len() as u32 - 1
            });
        }
    }
    let n = base.len() as u32;
    for &v in &base {
        let p = mesh.position(v) + Vec3::from_array(mesh.normals[v as usize]) * thickness;
        positions.push(p.to_array());
    }
    let mut indices = Vec::new();
    for t in (0..mesh.triangle_count()).filter(|&t| inside[t]) {
        let [a, b, c] = mesh.triangle(t).map(|v| local[&v]);
        indices.extend_from_slice(&[a + n, b + n, c + n]);
        indices.extend_from_slice(&[a, c, b]);
    }
    for (u, v) in boundary(mesh, &inside) {
        let (ub, vb) = (local[&u], local[&v]);
        let (ut, vt) = (ub + n, vb + n);
        indices.extend_from_slice(&[vt, ut, ub, vt, ub, vb]);
    }
    let mut out = Mesh::new(positions, indices);
    if mesh.has_paint() {
        out.paint = base.iter().chain(base.iter()).map(|&v| mesh.paint[v as usize]).collect();
    }
    Some(out)
}

pub fn extrude_region(mesh: &Mesh, mask: &[f32], distance: f32, inset: f32) -> Option<(Mesh, Vec<f32>)> {
    let inside = region_faces(mesh, mask);
    if !inside.iter().any(|&x| x) {
        return None;
    }
    let edges = boundary(mesh, &inside);
    let on_edge: std::collections::HashSet<u32> = edges.iter().flat_map(|&(a, b)| [a, b]).collect();
    let mut positions = mesh.positions.clone();
    let mut new_mask = mask.to_vec();
    let mut paint = mesh.paint.clone();
    let mut copy: HashMap<u32, u32> = HashMap::new();
    let mut neighbor_sum: HashMap<u32, (Vec3, f32)> = HashMap::new();
    for t in (0..mesh.triangle_count()).filter(|&t| inside[t]) {
        let tri = mesh.triangle(t);
        let centroid = (mesh.position(tri[0]) + mesh.position(tri[1]) + mesh.position(tri[2])) / 3.0;
        for v in tri {
            let e = neighbor_sum.entry(v).or_insert((Vec3::ZERO, 0.0));
            e.0 += centroid;
            e.1 += 1.0;
        }
    }
    let mut ordered: Vec<u32> = neighbor_sum.keys().copied().collect();
    ordered.sort_unstable();
    for v in ordered {
        let p = mesh.position(v);
        let n = Vec3::from_array(mesh.normals[v as usize]);
        let mut q = p + n * distance;
        if inset > 0.0 && on_edge.contains(&v) {
            let (sum, count) = neighbor_sum[&v];
            let toward = sum / count - p;
            q += (toward - n * toward.dot(n)) * inset.clamp(0.0, 1.0) * 2.0;
        }
        copy.insert(v, positions.len() as u32);
        positions.push(q.to_array());
        new_mask.push(mask[v as usize]);
        if mesh.has_paint() {
            paint.push(mesh.paint[v as usize]);
        }
    }
    let mut indices = Vec::with_capacity(mesh.indices.len() + edges.len() * 6);
    for (t, &is_in) in inside.iter().enumerate() {
        let tri = mesh.triangle(t);
        if is_in {
            indices.extend(tri.iter().map(|v| copy[v]));
        } else {
            indices.extend_from_slice(&tri);
        }
    }
    for (u, v) in edges {
        let (u2, v2) = (copy[&u], copy[&v]);
        indices.extend_from_slice(&[u, v, v2, u, v2, u2]);
    }
    let mut out = Mesh::new(positions, indices);
    out.paint = paint;
    crate::dyntopo::compact(&mut out, &mut new_mask);
    out.recompute_normals();
    Some((out, new_mask))
}

pub fn project_onto(dst: &Mesh, mask: &[f32], src: &Mesh, src_bvh: &Bvh, max_dist: f32, strength: f32) -> (Vec<[f32; 3]>, usize) {
    let mut moved = 0;
    let out = dst
        .positions
        .iter()
        .enumerate()
        .map(|(v, p)| {
            let p = Vec3::from_array(*p);
            let n = Vec3::from_array(dst.normals[v]);
            let w = strength * (1.0 - mask.get(v).copied().unwrap_or(0.0));
            if w <= 0.0 {
                return p.to_array();
            }
            let mut best: Option<f32> = None;
            for dir in [n, -n] {
                if let Some(h) = src_bvh.raycast(src, p - dir * 1e-5, dir) {
                    let d = h.t - 1e-5;
                    if d <= max_dist && best.is_none_or(|b| d < b.abs()) {
                        best = Some(if dir == n { d } else { -d });
                    }
                }
            }
            match best {
                Some(d) => {
                    moved += 1;
                    (p + n * d * w).to_array()
                }
                None => p.to_array(),
            }
        })
        .collect();
    (out, moved)
}
