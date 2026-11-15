use std::collections::{HashMap, HashSet};

use fsc_core::Mesh;
use glam::Vec3;

use crate::edges::{key, Edge};

fn fan_neighbors(mesh: &Mesh, faces: &[u32], v: u32) -> Option<Vec<u32>> {
    let mut seen: Vec<u32> = Vec::with_capacity(faces.len() * 2);
    for &f in faces {
        seen.extend(mesh.triangle(f as usize).into_iter().filter(|&w| w != v));
    }
    seen.sort_unstable();
    let interior = seen.chunks(2).all(|c| c.len() == 2 && c[0] == c[1]) && seen.windows(3).all(|w| !(w[0] == w[1] && w[1] == w[2]));
    if !interior {
        return None;
    }
    seen.dedup();
    Some(seen)
}

fn face_normal(mesh: &Mesh, f: u32, moved: &[(u32, Vec3)]) -> Vec3 {
    let p = |v: u32| moved.iter().find(|m| m.0 == v).map(|m| m.1).unwrap_or_else(|| mesh.position(v));
    let [a, b, c] = mesh.triangle(f as usize);
    (p(b) - p(a)).cross(p(c) - p(a))
}

pub fn collapse_sphere(mesh: &mut Mesh, mask: &mut [f32], center: Vec3, radius: f32, min_edge: f32) -> usize {
    let min2 = min_edge * min_edge;
    let reach = radius + min_edge;
    let inside = |p: Vec3| p.distance_squared(center) <= reach * reach;
    let mut faces_of: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut short: HashMap<Edge, f32> = HashMap::new();
    for t in 0..mesh.triangle_count() {
        let tri = mesh.triangle(t);
        if !tri.iter().any(|&v| inside(mesh.position(v))) {
            continue;
        }
        for (k, &v) in tri.iter().enumerate() {
            faces_of.entry(v).or_default().push(t as u32);
            let w = tri[(k + 1) % 3];
            let (pa, pb) = (mesh.position(v), mesh.position(w));
            let l2 = pa.distance_squared(pb);
            if l2 < min2 && inside(pa) && inside(pb) && ((pa + pb) * 0.5).distance(center) <= radius {
                short.insert(key(v, w), l2);
            }
        }
    }
    let mut order: Vec<(Edge, f32)> = short.into_iter().collect();
    order.sort_by(|x, y| x.1.total_cmp(&y.1));
    let mut locked: HashSet<u32> = HashSet::new();
    let mut dead: HashSet<u32> = HashSet::new();
    let mut collapsed = 0;
    for ((a, b), _) in order {
        if locked.contains(&a) || locked.contains(&b) || mask[a as usize] > 0.5 || mask[b as usize] > 0.5 {
            continue;
        }
        let (Some(fa), Some(fb)) = (faces_of.get(&a), faces_of.get(&b)) else { continue };
        let shared: Vec<u32> = fa.iter().copied().filter(|f| fb.contains(f)).collect();
        if shared.len() != 2 {
            continue;
        }
        let (Some(na), Some(nb)) = (fan_neighbors(mesh, fa, a), fan_neighbors(mesh, fb, b)) else { continue };
        let opposite: Vec<u32> = shared.iter().map(|&f| mesh.triangle(f as usize).into_iter().find(|&w| w != a && w != b).unwrap_or(a)).collect();
        let common: Vec<u32> = na.iter().copied().filter(|w| nb.contains(w)).collect();
        if common.len() != 2 || !opposite.iter().all(|o| common.contains(o)) {
            continue;
        }
        let valence_ok = opposite.iter().all(|&o| faces_of.get(&o).and_then(|f| fan_neighbors(mesh, f, o)).is_some_and(|n| n.len() > 3));
        if !valence_ok || na.len() + nb.len() < 7 {
            continue;
        }
        let pm = (mesh.position(a) + mesh.position(b)) * 0.5;
        let moved = [(a, pm), (b, pm)];
        let flips = fa.iter().chain(fb.iter()).filter(|f| !shared.contains(f)).any(|&f| {
            let before = face_normal(mesh, f, &[]);
            let after = face_normal(mesh, f, &moved);
            after.length_squared() <= 1e-20 || before.normalize_or_zero().dot(after.normalize_or_zero()) < 0.3
        });
        if flips {
            continue;
        }
        let keep_faces: Vec<u32> = fa.iter().chain(fb.iter()).copied().filter(|f| !shared.contains(f)).collect();
        for &f in fb {
            if shared.contains(&f) {
                continue;
            }
            for k in 0..3 {
                if mesh.indices[3 * f as usize + k] == b {
                    mesh.indices[3 * f as usize + k] = a;
                }
            }
        }
        mesh.positions[a as usize] = pm.to_array();
        mask[a as usize] = 0.5 * (mask[a as usize] + mask[b as usize]);
        if mesh.has_paint() {
            mesh.paint[a as usize] = fsc_core::mix_paint(mesh.paint[a as usize], mesh.paint[b as usize], 0.5);
        }
        for &o in &opposite {
            if let Some(list) = faces_of.get_mut(&o) {
                list.retain(|f| !shared.contains(f));
            }
        }
        dead.extend(shared.iter().copied());
        faces_of.insert(a, keep_faces);
        faces_of.remove(&b);
        locked.extend(na.iter().chain(nb.iter()).copied());
        locked.insert(a);
        locked.insert(b);
        collapsed += 1;
    }
    if !dead.is_empty() {
        let old = std::mem::take(&mut mesh.indices);
        mesh.indices = old.as_chunks::<3>().0.iter().enumerate().filter(|(t, _)| !dead.contains(&(*t as u32))).flat_map(|(_, tri)| tri.iter().copied()).collect();
    }
    collapsed
}

pub fn compact(mesh: &mut Mesh, mask: &mut Vec<f32>) -> usize {
    let n = mesh.vertex_count();
    let mut remap = vec![u32::MAX; n];
    let mut next = 0u32;
    for &i in &mesh.indices {
        if remap[i as usize] == u32::MAX {
            remap[i as usize] = 0;
        }
    }
    for slot in remap.iter_mut() {
        if *slot == 0 {
            *slot = next;
            next += 1;
        }
    }
    let removed = n - next as usize;
    if removed == 0 {
        return 0;
    }
    let keep = |v: usize| remap[v] != u32::MAX;
    let mut k = 0;
    for v in 0..n {
        if keep(v) {
            mesh.positions[k] = mesh.positions[v];
            mesh.normals[k] = mesh.normals[v];
            mask[k] = mask[v];
            if !mesh.paint.is_empty() {
                mesh.paint[k] = mesh.paint[v];
            }
            k += 1;
        }
    }
    mesh.positions.truncate(k);
    mesh.normals.truncate(k);
    mask.truncate(k);
    if !mesh.paint.is_empty() {
        mesh.paint.truncate(k);
    }
    for i in mesh.indices.iter_mut() {
        *i = remap[*i as usize];
    }
    removed
}

pub fn refine_sphere(mesh: &mut Mesh, mask: &mut Vec<f32>, center: Vec3, radius: f32, max_edge: f32, passes: u32) -> usize {
    refine_sphere_protected(mesh, mask, center, radius, max_edge, passes, false)
}

/// refine_sphere; with `protect_mask` an edge with a masked end (mask above 0.5) is never split, so masked areas keep
/// their topology as they keep their shape.
pub fn refine_sphere_protected(mesh: &mut Mesh, mask: &mut Vec<f32>, center: Vec3, radius: f32, max_edge: f32, passes: u32, protect_mask: bool) -> usize {
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
                if protect_mask && (mask[a as usize] > 0.5 || mask[b as usize] > 0.5) {
                    continue;
                }
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
            if mesh.has_paint() {
                let p = fsc_core::mix_paint(mesh.paint[a as usize], mesh.paint[b as usize], 0.5);
                mesh.paint.push(p);
            }
            mask.push(0.5 * (mask[a as usize] + mask[b as usize]));
            for &f in faces {
                let base = 3 * f as usize;
                let tri = [mesh.indices[base], mesh.indices[base + 1], mesh.indices[base + 2]];
                let Some(k) = (0..3).find(|&k| key(tri[k], tri[(k + 1) % 3]) == (a, b)) else { continue };
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
