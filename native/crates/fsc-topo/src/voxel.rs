use std::collections::HashMap;

use fsc_core::{Aabb, Bvh, Mesh};
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

struct Grid {
    origin: Vec3,
    cell: f32,
    dims: [usize; 3],
}

impl Grid {
    fn around(bounds: Aabb, resolution: u32) -> Result<Grid, String> {
        let resolution = resolution.clamp(8, 320) as f32;
        let extent = bounds.extent();
        let cell = extent.max_element().max(1e-6) / resolution;
        let pad = 3usize;
        let origin = bounds.min - Vec3::splat(cell * pad as f32);
        let dims = [
            (extent.x / cell).ceil() as usize + 2 * pad + 1,
            (extent.y / cell).ceil() as usize + 2 * pad + 1,
            (extent.z / cell).ceil() as usize + 2 * pad + 1,
        ];
        if dims[0] * dims[1] * dims[2] > 40_000_000 {
            return Err("the grid would be too large; lower the resolution".into());
        }
        Ok(Grid { origin, cell, dims })
    }

    fn total(&self) -> usize {
        self.dims[0] * self.dims[1] * self.dims[2]
    }
}

pub fn voxel_remesh(mesh: &Mesh, bvh: &Bvh, resolution: u32) -> Result<Mesh, String> {
    voxel_remesh_thick(mesh, bvh, resolution, 0.0)
}

/// voxel_remesh that keeps thin parts: with `thicken` (0 to 1) the field is joined with a shell of radius r = thicken
/// times half a voxel around the surface, so every part comes out at least 2r thick (a sheet or strand thinner than a
/// voxel survives instead of breaking up) while the rest of the surface moves out by r, under the voxel size.
pub fn voxel_remesh_thick(mesh: &Mesh, bvh: &Bvh, resolution: u32, thicken: f32) -> Result<Mesh, String> {
    if mesh.triangle_count() == 0 {
        return Err("the mesh is empty".into());
    }
    let grid = Grid::around(mesh.bounds(), resolution)?;
    let mut field = narrow_band(mesh, bvh, &grid, grid.cell * 2.0);
    let r = thicken.clamp(0.0, 1.0) * grid.cell * 0.5;
    if r > 0.0 {
        for f in field.iter_mut() {
            *f = f.min(f.abs() - r);
        }
    }
    surface_nets(&field, &grid)
}

pub fn implicit_mesh<F: Fn(Vec3) -> f32 + Sync>(bounds: Aabb, resolution: u32, field_at: F) -> Result<Mesh, String> {
    let grid = Grid::around(bounds, resolution)?;
    let (origin, cell, dims) = (grid.origin, grid.cell, grid.dims);
    let sample = |at: usize| -> f32 {
        let (i, j, k) = (at % dims[0], (at / dims[0]) % dims[1], at / (dims[0] * dims[1]));
        field_at(origin + Vec3::new(i as f32, j as f32, k as f32) * cell)
    };
    #[cfg(feature = "parallel")]
    let field: Vec<f32> = {
        use rayon::prelude::*;
        (0..grid.total()).into_par_iter().map(sample).collect()
    };
    #[cfg(not(feature = "parallel"))]
    let field: Vec<f32> = (0..grid.total()).map(sample).collect();
    surface_nets(&field, &grid)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Difference,
    Intersection,
}

pub fn voxel_boolean(a: &Mesh, a_bvh: &Bvh, b: &Mesh, b_bvh: &Bvh, op: BooleanOp, resolution: u32) -> Result<Mesh, String> {
    voxel_boolean_blend(a, a_bvh, b, b_bvh, op, 0.0, resolution)
}

/// A voxel boolean whose seam is filleted over `blend` world units (a smooth minimum or maximum of the two distance
/// fields), so a union fuses two meshes with a soft web where they meet and a difference leaves a rounded cut edge.
/// With blend 0 it is the sharp boolean. The distance band around each mesh widens to cover the blend.
pub fn voxel_boolean_blend(a: &Mesh, a_bvh: &Bvh, b: &Mesh, b_bvh: &Bvh, op: BooleanOp, blend: f32, resolution: u32) -> Result<Mesh, String> {
    if a.triangle_count() == 0 || b.triangle_count() == 0 {
        return Err("a mesh is empty".into());
    }
    let mut bounds = a.bounds().merge(&b.bounds());
    // A fillet can reach past both meshes' bounds by a fraction of the blend.
    bounds.grow(bounds.min - Vec3::splat(blend * 0.5));
    bounds.grow(bounds.max + Vec3::splat(blend * 0.5));
    let grid = Grid::around(bounds, resolution)?;
    let band = (grid.cell * 2.0).max(blend * 1.2);
    let mut fa = narrow_band(a, a_bvh, &grid, band);
    let mut fb = narrow_band(b, b_bvh, &grid, band);
    fill_signs(&mut fa, &grid, band);
    fill_signs(&mut fb, &grid, band);
    let k = blend.max(0.0);
    let smin = |x: f32, y: f32| -> f32 {
        if k <= 0.0 {
            return x.min(y);
        }
        let h = (k - (x - y).abs()).max(0.0) / k;
        x.min(y) - h * h * k * 0.25
    };
    let field: Vec<f32> = fa
        .iter()
        .zip(&fb)
        .map(|(&x, &y)| match op {
            BooleanOp::Union => smin(x, y),
            BooleanOp::Difference => -smin(-x, y),
            BooleanOp::Intersection => -smin(-x, -y),
        })
        .collect();
    let mesh = surface_nets(&field, &grid)?;
    Ok(crate::manifold::split_nonmanifold(&mesh).0)
}

/// Which edges round_edges softens: convex edges (an opening: shrink by the radius, then grow back), concave ones (a
/// closing: grow, then shrink) or both (opening, then closing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundEdges {
    Convex,
    Concave,
    Both,
}

/// Volumetric bevel: every edge sharper than `radius` allows becomes a round of that radius, flat faces stay where they
/// were. Each pass offsets the true signed distance field and re-measures the distance from the offset surface, so the
/// faces return to their place and only the edges change. The result is remeshed watertight at `resolution`.
pub fn round_edges(mesh: &Mesh, bvh: &Bvh, radius: f32, which: RoundEdges, resolution: u32) -> Result<Mesh, String> {
    if mesh.triangle_count() == 0 {
        return Err("the mesh is empty".into());
    }
    let mut bounds = mesh.bounds();
    bounds.grow(bounds.min - Vec3::splat(radius * 1.5));
    bounds.grow(bounds.max + Vec3::splat(radius * 1.5));
    let grid = Grid::around(bounds, resolution)?;
    if radius < grid.cell * 1.5 {
        return Err(format!("the radius is under a voxel and a half ({:.4}); raise it or the resolution", grid.cell * 1.5));
    }
    let band = (grid.cell * 2.0).max(radius * 1.3);
    // Signed distance to `m`, true within the band.
    let field_of = |m: &Mesh| -> Vec<f32> {
        let b = Bvh::build(m);
        let mut f = narrow_band(m, &b, &grid, band);
        fill_signs(&mut f, &grid, band);
        f
    };
    let offset = |f: &[f32], by: f32| -> Result<Mesh, String> {
        let shifted: Vec<f32> = f.iter().map(|d| d + by).collect();
        Ok(crate::manifold::split_nonmanifold(&surface_nets(&shifted, &grid)?).0)
    };
    let mut current = {
        let mut f = narrow_band(mesh, bvh, &grid, band);
        fill_signs(&mut f, &grid, band);
        f
    };
    let passes: &[f32] = match which {
        RoundEdges::Convex => &[1.0, -1.0],
        RoundEdges::Concave => &[-1.0, 1.0],
        RoundEdges::Both => &[1.0, -1.0, -1.0, 1.0],
    };
    let mut out = None;
    for (k, sign) in passes.iter().enumerate() {
        let m = offset(&current, sign * radius)?;
        if m.triangle_count() == 0 {
            return Err("the radius swallows the whole object; use a smaller radius".into());
        }
        if k + 1 < passes.len() {
            current = field_of(&m);
        } else {
            out = Some(m);
        }
    }
    out.ok_or_else(|| "nothing was made".into())
}

fn fill_signs(field: &mut [f32], g: &Grid, band: f32) {
    let dims = g.dims;
    let idx = |i: usize, j: usize, k: usize| (k * dims[1] + j) * dims[0] + i;
    let far = (g.cell * 4.0).max(band * 2.0);
    let mut outside = vec![false; field.len()];
    let mut stack: Vec<usize> = Vec::new();
    for k in 0..dims[2] {
        for j in 0..dims[1] {
            for i in 0..dims[0] {
                let edge = i == 0 || j == 0 || k == 0 || i + 1 == dims[0] || j + 1 == dims[1] || k + 1 == dims[2];
                let at = idx(i, j, k);
                if edge && (field[at].is_nan() || field[at] > 0.0) {
                    outside[at] = true;
                    stack.push(at);
                }
            }
        }
    }
    while let Some(at) = stack.pop() {
        let (i, j, k) = (at % dims[0], (at / dims[0]) % dims[1], at / (dims[0] * dims[1]));
        let mut visit = |i: usize, j: usize, k: usize| {
            let n = idx(i, j, k);
            if !outside[n] && (field[n].is_nan() || field[n] > 0.0) {
                outside[n] = true;
                stack.push(n);
            }
        };
        if i > 0 {
            visit(i - 1, j, k);
        }
        if i + 1 < dims[0] {
            visit(i + 1, j, k);
        }
        if j > 0 {
            visit(i, j - 1, k);
        }
        if j + 1 < dims[1] {
            visit(i, j + 1, k);
        }
        if k > 0 {
            visit(i, j, k - 1);
        }
        if k + 1 < dims[2] {
            visit(i, j, k + 1);
        }
    }
    for (at, v) in field.iter_mut().enumerate() {
        if v.is_nan() {
            *v = if outside[at] { far } else { -far };
        }
    }
}

fn narrow_band(mesh: &Mesh, bvh: &Bvh, g: &Grid, band: f32) -> Vec<f32> {
    let (origin, cell, dims, total) = (g.origin, g.cell, g.dims, g.total());
    let idx = |i: usize, j: usize, k: usize| (k * dims[1] + j) * dims[0] + i;
    let point = |i: usize, j: usize, k: usize| origin + Vec3::new(i as f32, j as f32, k as f32) * cell;
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
    field
}

fn surface_nets(field: &[f32], g: &Grid) -> Result<Mesh, String> {
    let (origin, cell, dims) = (g.origin, g.cell, g.dims);
    let idx = |i: usize, j: usize, k: usize| (k * dims[1] + j) * dims[0] + i;
    let point = |i: usize, j: usize, k: usize| origin + Vec3::new(i as f32, j as f32, k as f32) * cell;
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
