use std::collections::HashMap;

use fsc_core::{Bvh, Mesh};
use glam::Vec3;

use crate::edges::{key, Edge};
use crate::nearest::nearest_feature;

struct PseudoNormals {
    face: Vec<Vec3>,
    vertex: Vec<Vec3>,
    edge: HashMap<Edge, Vec3>,
}

impl PseudoNormals {
    fn new(mesh: &Mesh) -> PseudoNormals {
        let mut face = Vec::with_capacity(mesh.triangle_count());
        let mut vertex = vec![Vec3::ZERO; mesh.vertex_count()];
        let mut edge: HashMap<Edge, Vec3> = HashMap::with_capacity(mesh.indices.len());
        for t in 0..mesh.triangle_count() {
            let tri = mesh.triangle(t);
            let p = [mesh.position(tri[0]), mesh.position(tri[1]), mesh.position(tri[2])];
            let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            face.push(n);
            for k in 0..3 {
                let (a, b, c) = (p[k], p[(k + 1) % 3], p[(k + 2) % 3]);
                let angle = (b - a).normalize_or_zero().dot((c - a).normalize_or_zero()).clamp(-1.0, 1.0).acos();
                vertex[tri[k] as usize] += n * angle;
                *edge.entry(key(tri[k], tri[(k + 1) % 3])).or_insert(Vec3::ZERO) += n;
            }
        }
        PseudoNormals { face, vertex, edge }
    }

    fn at(&self, mesh: &Mesh, t: u32, bary: [f32; 3]) -> Vec3 {
        let tri = mesh.triangle(t as usize);
        let eps = 1e-5;
        let zero: Vec<usize> = (0..3).filter(|&k| bary[k] <= eps).collect();
        match zero.len() {
            2 => {
                let k = (0..3).find(|k| !zero.contains(k)).unwrap_or(0);
                self.vertex[tri[k] as usize]
            }
            1 => {
                let z = zero[0];
                let (a, b) = (tri[(z + 1) % 3], tri[(z + 2) % 3]);
                self.edge.get(&key(a, b)).copied().unwrap_or(self.face[t as usize])
            }
            _ => self.face[t as usize],
        }
    }
}

pub fn voxel_remesh(mesh: &Mesh, bvh: &Bvh, resolution: u32) -> Result<Mesh, String> {
    if mesh.triangle_count() == 0 {
        return Err("the mesh is empty".into());
    }
    let resolution = resolution.clamp(8, 320) as f32;
    let bounds = mesh.bounds();
    let extent = bounds.extent();
    let cell = extent.max_element().max(1e-6) / resolution;
    let pad = 3usize;
    let origin = bounds.min - Vec3::splat(cell * pad as f32);
    let dims = [
        (extent.x / cell).ceil() as usize + 2 * pad + 1,
        (extent.y / cell).ceil() as usize + 2 * pad + 1,
        (extent.z / cell).ceil() as usize + 2 * pad + 1,
    ];
    let total = dims[0] * dims[1] * dims[2];
    if total > 40_000_000 {
        return Err("the grid would be too large; lower the resolution".into());
    }
    let idx = |i: usize, j: usize, k: usize| (k * dims[1] + j) * dims[0] + i;
    let point = |i: usize, j: usize, k: usize| origin + Vec3::new(i as f32, j as f32, k as f32) * cell;
    let band = cell * 2.0;
    let signs = PseudoNormals::new(mesh);
    let mut marked = vec![false; total];
    for t in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(t);
        let (pa, pb, pc) = (mesh.position(a), mesh.position(b), mesh.position(c));
        let lo = (pa.min(pb).min(pc) - Vec3::splat(band) - origin) / cell;
        let hi = (pa.max(pb).max(pc) + Vec3::splat(band) - origin) / cell;
        let (i0, j0, k0) = (lo.x.floor().max(0.0) as usize, lo.y.floor().max(0.0) as usize, lo.z.floor().max(0.0) as usize);
        let (i1, j1, k1) = ((hi.x.ceil() as usize).min(dims[0] - 1), (hi.y.ceil() as usize).min(dims[1] - 1), (hi.z.ceil() as usize).min(dims[2] - 1));
        for k in k0..=k1 {
            for j in j0..=j1 {
                for i in i0..=i1 {
                    marked[idx(i, j, k)] = true;
                }
            }
        }
    }
    let sample = |at: usize| -> f32 {
        if !marked[at] {
            return f32::NAN;
        }
        let i = at % dims[0];
        let j = (at / dims[0]) % dims[1];
        let k = at / (dims[0] * dims[1]);
        let p = point(i, j, k);
        match nearest_feature(bvh, mesh, p, band * 1.5) {
            Some((q, t, bary, d)) => {
                if (p - q).dot(signs.at(mesh, t, bary)) < 0.0 {
                    -d
                } else {
                    d
                }
            }
            None => f32::NAN,
        }
    };
    #[cfg(feature = "parallel")]
    let field: Vec<f32> = {
        use rayon::prelude::*;
        (0..total).into_par_iter().map(sample).collect()
    };
    #[cfg(not(feature = "parallel"))]
    let field: Vec<f32> = (0..total).map(sample).collect();
    let cdims = [dims[0] - 1, dims[1] - 1, dims[2] - 1];
    let cidx = |i: usize, j: usize, k: usize| (k * cdims[1] + j) * cdims[0] + i;
    let mut cell_vertex = vec![u32::MAX; cdims[0] * cdims[1] * cdims[2]];
    let mut positions: Vec<[f32; 3]> = Vec::new();
    const CORNERS: [(usize, usize, usize); 8] = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (1, 1, 0), (0, 0, 1), (1, 0, 1), (0, 1, 1), (1, 1, 1)];
    const EDGES: [(usize, usize); 12] = [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)];
    for k in 0..cdims[2] {
        for j in 0..cdims[1] {
            for i in 0..cdims[0] {
                let mut vals = [0f32; 8];
                let mut known = true;
                let mut inside = 0;
                for (c, (di, dj, dk)) in CORNERS.iter().enumerate() {
                    let v = field[idx(i + di, j + dj, k + dk)];
                    if v.is_nan() {
                        known = false;
                        break;
                    }
                    vals[c] = v;
                    if v < 0.0 {
                        inside += 1;
                    }
                }
                if !known || inside == 0 || inside == 8 {
                    continue;
                }
                let mut sum = Vec3::ZERO;
                let mut count = 0.0;
                for (a, b) in EDGES {
                    let (va, vb) = (vals[a], vals[b]);
                    if (va < 0.0) != (vb < 0.0) {
                        let t = va / (va - vb);
                        let ca = CORNERS[a];
                        let cb = CORNERS[b];
                        let pa = Vec3::new(ca.0 as f32, ca.1 as f32, ca.2 as f32);
                        let pb = Vec3::new(cb.0 as f32, cb.1 as f32, cb.2 as f32);
                        sum += pa + (pb - pa) * t;
                        count += 1.0;
                    }
                }
                let local = sum / count;
                cell_vertex[cidx(i, j, k)] = positions.len() as u32;
                positions.push((point(i, j, k) + local * cell).to_array());
            }
        }
    }
    let mut indices: Vec<u32> = Vec::new();
    for axis in 0..3 {
        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
        for k in 0..dims[2] {
            for j in 0..dims[1] {
                for i in 0..dims[0] {
                    let x = [i, j, k];
                    if x[axis] + 1 >= dims[axis] || x[u] == 0 || x[v] == 0 {
                        continue;
                    }
                    let mut y = x;
                    y[axis] += 1;
                    let (f0, f1) = (field[idx(x[0], x[1], x[2])], field[idx(y[0], y[1], y[2])]);
                    if f0.is_nan() || f1.is_nan() || (f0 < 0.0) == (f1 < 0.0) {
                        continue;
                    }
                    let cellv = |du: usize, dv: usize| -> u32 {
                        let mut c = x;
                        c[u] -= du;
                        c[v] -= dv;
                        if c[0] >= cdims[0] || c[1] >= cdims[1] || c[2] >= cdims[2] {
                            return u32::MAX;
                        }
                        cell_vertex[cidx(c[0], c[1], c[2])]
                    };
                    let (c0, c1, c2, c3) = (cellv(1, 1), cellv(0, 1), cellv(0, 0), cellv(1, 0));
                    if [c0, c1, c2, c3].contains(&u32::MAX) {
                        continue;
                    }
                    if f0 < 0.0 {
                        indices.extend_from_slice(&[c0, c1, c2, c0, c2, c3]);
                    } else {
                        indices.extend_from_slice(&[c0, c2, c1, c0, c3, c2]);
                    }
                }
            }
        }
    }
    if indices.is_empty() {
        return Err("no surface was found at this resolution".into());
    }
    let mut out = Mesh::new(positions, indices);
    if signed_volume(&out) < 0.0 {
        for t in out.indices.chunks_mut(3) {
            t.swap(1, 2);
        }
        out.recompute_normals();
    }
    Ok(out)
}

pub(crate) fn signed_volume(mesh: &Mesh) -> f32 {
    (0..mesh.triangle_count())
        .map(|t| {
            let [a, b, c] = mesh.triangle(t);
            mesh.position(a).dot(mesh.position(b).cross(mesh.position(c))) / 6.0
        })
        .sum()
}
