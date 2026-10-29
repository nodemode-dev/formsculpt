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

/// The solids `extract_patch` can build from a masked patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchMode {
    Balloon,
    Extrude,
    Splat,
    Project,
}

fn patch_slab(mesh: &Mesh, inside: &[bool], local: &HashMap<u32, u32>, base: &[u32], back: &[Vec3], front: &[Vec3], flip: bool) -> Mesh {
    let n = base.len() as u32;
    let positions: Vec<[f32; 3]> = back.iter().chain(front.iter()).map(|p| p.to_array()).collect();
    let mut indices = Vec::new();
    for t in (0..mesh.triangle_count()).filter(|&t| inside[t]) {
        let [a, b, c] = mesh.triangle(t).map(|v| local[&v]);
        indices.extend_from_slice(&[a + n, b + n, c + n]);
        indices.extend_from_slice(&[a, c, b]);
    }
    for (u, v) in boundary(mesh, inside) {
        let (ub, vb) = (local[&u], local[&v]);
        let (ut, vt) = (ub + n, vb + n);
        indices.extend_from_slice(&[vt, ut, ub, vt, ub, vb]);
    }
    if flip {
        for tri in indices.as_chunks_mut::<3>().0 {
            tri.swap(1, 2);
        }
    }
    let mut out = Mesh::new(positions, indices);
    if mesh.has_paint() {
        out.paint = base.iter().chain(base.iter()).map(|&v| mesh.paint[v as usize]).collect();
    }
    out
}

/// Builds a closed solid from the masked patch: a balloon dome, an extrusion along the average normal, a splat with a flat back, or a flat projected badge.
pub fn extract_patch(mesh: &Mesh, mask: &[f32], mode: PatchMode, thickness: f32, direction: Option<Vec3>) -> Result<Mesh, &'static str> {
    let inside = region_faces(mesh, mask);
    if !inside.iter().any(|&x| x) {
        return Err("nothing is masked");
    }
    let mut local: HashMap<u32, u32> = HashMap::new();
    let mut base: Vec<u32> = Vec::new();
    let mut area_normal = Vec3::ZERO;
    for t in (0..mesh.triangle_count()).filter(|&t| inside[t]) {
        let tri = mesh.triangle(t);
        for v in tri {
            local.entry(v).or_insert_with(|| {
                base.push(v);
                base.len() as u32 - 1
            });
        }
        let [a, b, c] = tri.map(|v| mesh.position(v));
        area_normal += (b - a).cross(c - a);
    }
    let average = area_normal.try_normalize().unwrap_or(Vec3::Y);
    let surface: Vec<Vec3> = base.iter().map(|&v| mesh.position(v)).collect();
    let normals: Vec<Vec3> = base.iter().map(|&v| Vec3::from_array(mesh.normals[v as usize])).collect();
    let flip = thickness < 0.0;
    match mode {
        PatchMode::Extrude => {
            let front: Vec<Vec3> = surface.iter().map(|&p| p + average * thickness).collect();
            Ok(patch_slab(mesh, &inside, &local, &base, &surface, &front, flip))
        }
        PatchMode::Splat => {
            let floor = surface.iter().map(|p| p.dot(average)).fold(f32::INFINITY, f32::min);
            let back: Vec<Vec3> = surface.iter().map(|&p| p + average * (floor - p.dot(average))).collect();
            let front: Vec<Vec3> = surface.iter().zip(&normals).map(|(&p, &n)| p + n * thickness).collect();
            Ok(patch_slab(mesh, &inside, &local, &base, &back, &front, flip))
        }
        PatchMode::Project => {
            let dir = direction.and_then(Vec3::try_normalize).unwrap_or(average);
            let centre = surface.iter().sum::<Vec3>() / surface.len() as f32;
            let back: Vec<Vec3> = surface.iter().map(|&p| p - dir * (p - centre).dot(dir)).collect();
            let front: Vec<Vec3> = back.iter().map(|&p| p + dir * thickness).collect();
            Ok(patch_slab(mesh, &inside, &local, &base, &back, &front, (thickness * dir.dot(average)) < 0.0))
        }
        PatchMode::Balloon => {
            let edges = boundary(mesh, &inside);
            if edges.is_empty() {
                return Err("the masked part has no border; leave some of the surface unmasked");
            }
            let count = base.len();
            let mut on_border = vec![false; count];
            for &(a, b) in &edges {
                on_border[local[&a] as usize] = true;
                on_border[local[&b] as usize] = true;
            }
            let mut adjacent: Vec<Vec<(usize, f32)>> = vec![Vec::new(); count];
            for t in (0..mesh.triangle_count()).filter(|&t| inside[t]) {
                let tri = mesh.triangle(t).map(|v| local[&v] as usize);
                for k in 0..3 {
                    let (a, b) = (tri[k], tri[(k + 1) % 3]);
                    let len = surface[a].distance(surface[b]);
                    adjacent[a].push((b, len));
                    adjacent[b].push((a, len));
                }
            }
            let mut dist = vec![f32::INFINITY; count];
            let mut heap = std::collections::BinaryHeap::new();
            for (i, &border) in on_border.iter().enumerate() {
                if border {
                    dist[i] = 0.0;
                    heap.push(std::cmp::Reverse((0u32, i)));
                }
            }
            while let Some(std::cmp::Reverse((bits, i))) = heap.pop() {
                if f32::from_bits(bits) > dist[i] {
                    continue;
                }
                for &(j, len) in &adjacent[i] {
                    let d = dist[i] + len;
                    if d < dist[j] {
                        dist[j] = d;
                        heap.push(std::cmp::Reverse((d.to_bits(), j)));
                    }
                }
            }
            let reach = dist.iter().copied().filter(|d| d.is_finite()).fold(0.0f32, f32::max);
            if reach <= 0.0 {
                return Err("the masked patch has no interior vertices; mask a larger area");
            }
            let mut front_of: Vec<u32> = (0..count as u32).collect();
            let mut positions: Vec<[f32; 3]> = surface.iter().map(|p| p.to_array()).collect();
            let mut paint_from: Vec<u32> = base.clone();
            for i in 0..count {
                if on_border[i] {
                    continue;
                }
                let d = if dist[i].is_finite() { (dist[i] / reach).clamp(0.0, 1.0) } else { 1.0 };
                let profile = (1.0 - (1.0 - d) * (1.0 - d)).max(0.0).sqrt();
                front_of[i] = positions.len() as u32;
                positions.push((surface[i] + normals[i] * (thickness * profile)).to_array());
                paint_from.push(base[i]);
            }
            let mut indices = Vec::new();
            for t in (0..mesh.triangle_count()).filter(|&t| inside[t]) {
                let [a, b, c] = mesh.triangle(t).map(|v| local[&v]);
                indices.extend_from_slice(&[front_of[a as usize], front_of[b as usize], front_of[c as usize]]);
                indices.extend_from_slice(&[a, c, b]);
            }
            if flip {
                for tri in indices.as_chunks_mut::<3>().0 {
                    tri.swap(1, 2);
                }
            }
            let mut out = Mesh::new(positions, indices);
            if mesh.has_paint() {
                out.paint = paint_from.iter().map(|&v| mesh.paint[v as usize]).collect();
            }
            Ok(out)
        }
    }
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
