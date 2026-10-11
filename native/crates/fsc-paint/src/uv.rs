use std::collections::HashMap;

use fsc_core::Mesh;
use glam::{Vec2, Vec3};

fn label_of(n: Vec3) -> u8 {
    let a = n.abs();
    let k = if a.x >= a.y && a.x >= a.z { 0 } else if a.y >= a.z { 1 } else { 2 };
    k * 2 + u8::from(n[k as usize] < 0.0)
}

fn axis_dir(label: u8) -> Vec3 {
    let mut d = Vec3::ZERO;
    d[(label / 2) as usize] = if label % 2 == 0 { 1.0 } else { -1.0 };
    d
}

fn project(p: Vec3, label: u8) -> Vec2 {
    let k = (label / 2) as usize;
    let (u, v) = (p[(k + 1) % 3], p[(k + 2) % 3]);
    if label % 2 == 0 {
        Vec2::new(u, v)
    } else {
        Vec2::new(-u, v)
    }
}

pub struct Unwrap {
    pub uvs: Vec<[f32; 2]>,
    pub charts: usize,
    pub coverage: f32,
}

pub fn unwrap(mesh: &Mesh, padding: f32) -> Unwrap {
    let tris = mesh.triangle_count();
    let normals: Vec<Vec3> = mesh.face_normals().into_iter().map(|n| n.normalize_or(Vec3::Y)).collect();
    let mut labels: Vec<u8> = normals.iter().map(|&n| label_of(n)).collect();
    let mut edge_faces: HashMap<(u32, u32), Vec<u32>> = HashMap::with_capacity(tris * 2);
    for t in 0..tris {
        let tri = mesh.triangle(t);
        for k in 0..3 {
            let (a, b) = (tri[k], tri[(k + 1) % 3]);
            edge_faces.entry((a.min(b), a.max(b))).or_default().push(t as u32);
        }
    }
    let neighbors = |t: usize| -> Vec<usize> {
        let tri = mesh.triangle(t);
        let mut out = Vec::with_capacity(3);
        for k in 0..3 {
            let (a, b) = (tri[k], tri[(k + 1) % 3]);
            if let Some(fs) = edge_faces.get(&(a.min(b), a.max(b))) {
                out.extend(fs.iter().map(|&f| f as usize).filter(|&f| f != t));
            }
        }
        out
    };
    for _ in 0..3 {
        let prev = labels.clone();
        for t in 0..tris {
            let mut votes = [0u8; 6];
            votes[prev[t] as usize] += 1;
            for f in neighbors(t) {
                votes[prev[f] as usize] += 1;
            }
            let best = (0..6u8).max_by_key(|&l| (votes[l as usize], u8::from(l == prev[t]))).unwrap_or(prev[t]);
            if best != prev[t] && votes[best as usize] >= 2 && normals[t].dot(axis_dir(best)) > 0.34 {
                labels[t] = best;
            }
        }
    }
    let mut chart = vec![u32::MAX; tris];
    let mut charts: Vec<Vec<u32>> = Vec::new();
    for seed in 0..tris {
        if chart[seed] != u32::MAX {
            continue;
        }
        let id = charts.len() as u32;
        let mut members = vec![seed as u32];
        chart[seed] = id;
        let mut at = 0;
        while at < members.len() {
            let t = members[at] as usize;
            at += 1;
            for f in neighbors(t) {
                if chart[f] == u32::MAX && labels[f] == labels[seed] {
                    chart[f] = id;
                    members.push(f as u32);
                }
            }
        }
        charts.push(members);
    }
    let mut boxes: Vec<(Vec2, Vec2)> = Vec::with_capacity(charts.len());
    let mut area = 0.0;
    for members in &charts {
        let label = labels[members[0] as usize];
        let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
        for &t in members {
            for v in mesh.triangle(t as usize) {
                let q = project(mesh.position(v), label);
                lo = lo.min(q);
                hi = hi.max(q);
            }
        }
        let size = (hi - lo).max(Vec2::splat(1e-6));
        area += size.x * size.y;
        boxes.push((lo, size));
    }
    let mut order: Vec<usize> = (0..charts.len()).collect();
    order.sort_by(|&a, &b| boxes[b].1.y.total_cmp(&boxes[a].1.y));
    let mut side = area.sqrt() * 1.1;
    let offsets = loop {
        let pad = side * padding;
        let (mut x, mut y, mut row) = (pad, pad, 0.0f32);
        let mut out = vec![Vec2::ZERO; charts.len()];
        let mut fits = true;
        for &c in &order {
            let s = boxes[c].1;
            if x + s.x + pad > side && x > pad {
                x = pad;
                y += row + pad;
                row = 0.0;
            }
            out[c] = Vec2::new(x, y);
            x += s.x + pad;
            row = row.max(s.y);
            if y + s.y + pad > side || x > side {
                fits = false;
                break;
            }
        }
        if fits {
            break out;
        }
        side *= 1.05;
    };
    let mut uvs = vec![[0.0f32; 2]; mesh.indices.len()];
    for (c, members) in charts.iter().enumerate() {
        let label = labels[members[0] as usize];
        for &t in members {
            for k in 0..3 {
                let v = mesh.indices[3 * t as usize + k];
                let q = (project(mesh.position(v), label) - boxes[c].0 + offsets[c]) / side;
                uvs[3 * t as usize + k] = q.to_array();
            }
        }
    }
    Unwrap { uvs, charts: charts.len(), coverage: area / (side * side) }
}

pub struct Expanded {
    pub corner: Vec<u32>,
    pub source: Vec<u32>,
    pub uv: Vec<[f32; 2]>,
    pub tangent: Vec<[f32; 4]>,
}

pub fn expand(mesh: &Mesh) -> Expanded {
    let mut key: HashMap<(u32, u32, u32), u32> = HashMap::with_capacity(mesh.vertex_count() * 2);
    let mut corner = Vec::with_capacity(mesh.indices.len());
    let mut source = Vec::new();
    let mut uv = Vec::new();
    for (c, &v) in mesh.indices.iter().enumerate() {
        let w = mesh.uvs[c];
        let id = *key.entry((v, w[0].to_bits(), w[1].to_bits())).or_insert_with(|| {
            source.push(v);
            uv.push(w);
            source.len() as u32 - 1
        });
        corner.push(id);
    }
    let n = source.len();
    let mut tan = vec![Vec3::ZERO; n];
    let mut bit = vec![Vec3::ZERO; n];
    for t in 0..mesh.triangle_count() {
        let ids = [corner[3 * t], corner[3 * t + 1], corner[3 * t + 2]];
        let p: Vec<Vec3> = ids.iter().map(|&i| mesh.position(source[i as usize])).collect();
        let q: Vec<Vec2> = ids.iter().map(|&i| Vec2::from_array(uv[i as usize])).collect();
        let (e1, e2) = (p[1] - p[0], p[2] - p[0]);
        let (d1, d2) = (q[1] - q[0], q[2] - q[0]);
        let r = d1.x * d2.y - d2.x * d1.y;
        if r.abs() < 1e-20 {
            continue;
        }
        let sdir = (e1 * d2.y - e2 * d1.y) / r;
        let tdir = (e2 * d1.x - e1 * d2.x) / r;
        for &i in &ids {
            tan[i as usize] += sdir;
            bit[i as usize] += tdir;
        }
    }
    let tangent = (0..n)
        .map(|i| {
            let nrm = Vec3::from_array(mesh.normals[source[i] as usize]);
            let t = (tan[i] - nrm * nrm.dot(tan[i])).try_normalize().unwrap_or_else(|| nrm.any_orthonormal_vector());
            let w = if nrm.cross(t).dot(bit[i]) < 0.0 { -1.0 } else { 1.0 };
            [t.x, t.y, t.z, w]
        })
        .collect();
    Expanded { corner, source, uv, tangent }
}
