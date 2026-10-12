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

/// How each chart is laid flat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flatten {
    /// Straight projection along the chart's axis: fast, but faces at an angle to the axis are stretched.
    Project,
    /// Least-squares conformal maps (Levy et al. 2002) started from the projection: angles kept, little stretch.
    Conformal,
}

/// The box-projection unwrap (charts by facing direction, projected flat, packed).
pub fn unwrap(mesh: &Mesh, padding: f32) -> Unwrap {
    unwrap_with(mesh, padding, Flatten::Project)
}

pub fn unwrap_with(mesh: &Mesh, padding: f32, flatten: Flatten) -> Unwrap {
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
    // Each chart's flat coordinates per mesh vertex (a seam vertex has one in each chart it borders).
    let flat: Vec<HashMap<u32, Vec2>> = charts
        .iter()
        .map(|members| {
            let label = labels[members[0] as usize];
            let mut at: HashMap<u32, Vec2> = HashMap::new();
            for &t in members {
                for v in mesh.triangle(t as usize) {
                    at.entry(v).or_insert_with(|| project(mesh.position(v), label));
                }
            }
            if flatten == Flatten::Conformal {
                conformal(mesh, members, &mut at);
            }
            at
        })
        .collect();
    let mut boxes: Vec<(Vec2, Vec2)> = Vec::with_capacity(charts.len());
    let mut area = 0.0;
    for at in &flat {
        let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
        for &q in at.values() {
            lo = lo.min(q);
            hi = hi.max(q);
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
        for &t in members {
            for k in 0..3 {
                let v = mesh.indices[3 * t as usize + k];
                let q = (flat[c][&v] - boxes[c].0 + offsets[c]) / side;
                uvs[3 * t as usize + k] = q.to_array();
            }
        }
    }
    Unwrap { uvs, charts: charts.len(), coverage: area / (side * side) }
}

/// A triangle in its own plane: corners at q[0] = origin and q[1] on +x, counter-clockwise about the normal, and twice
/// its area; None when it is degenerate.
fn local_triangle(p: [Vec3; 3]) -> Option<([[f64; 2]; 3], f64)> {
    let n = (p[1] - p[0]).cross(p[2] - p[0]);
    let twice = n.length();
    let along = (p[1] - p[0]).length();
    if twice <= 1e-20 || along <= 1e-12 {
        return None;
    }
    let x = (p[1] - p[0]) / along;
    let y = (n / twice).cross(x);
    let q = p.map(|c| [f64::from((c - p[0]).dot(x)), f64::from((c - p[0]).dot(y))]);
    Some((q, f64::from(twice)))
}

/// Edge opposite each corner, counter-clockwise: e[i] = q[i+2] - q[i+1].
fn opposite_edges(q: &[[f64; 2]; 3]) -> [[f64; 2]; 3] {
    std::array::from_fn(|i| {
        let (a, b) = (q[(i + 1) % 3], q[(i + 2) % 3]);
        [b[0] - a[0], b[1] - a[1]]
    })
}

/// Lays one chart flat by least-squares conformal maps: for every triangle, the residual of the Cauchy-Riemann
/// condition (grad v = rot90 grad u), area-weighted, minimised by conjugate gradients with two vertices pinned.
/// `at` holds the projection on entry (the start and the pins) and the result on return; a chart whose solution
/// folds over itself keeps its projection.
fn conformal(mesh: &Mesh, members: &[u32], at: &mut HashMap<u32, Vec2>) {
    let mut verts: Vec<u32> = at.keys().copied().collect();
    verts.sort_unstable();
    let n = verts.len();
    if n < 4 {
        return;
    }
    let index: HashMap<u32, usize> = verts.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    // The projection may be mirrored relative to the faces' winding; start from its orientation-preserving twin.
    let mut signed = 0.0f64;
    for &t in members {
        let [a, b, c] = mesh.triangle(t as usize).map(|v| at[&v]);
        signed += f64::from((b - a).perp_dot(c - a));
    }
    let mirror = if signed < 0.0 { -1.0 } else { 1.0 };
    let mut x = vec![0.0f64; 2 * n];
    for (i, v) in verts.iter().enumerate() {
        let q = at[v];
        x[i] = f64::from(q.x) * mirror;
        x[n + i] = f64::from(q.y);
    }
    // Rows of the energy: two per triangle, six (column, coefficient) entries each.
    let mut rows: Vec<[(usize, f64); 6]> = Vec::with_capacity(members.len() * 2);
    let mut area3 = 0.0f64;
    for &t in members {
        let tri = mesh.triangle(t as usize);
        let Some((q, twice)) = local_triangle(tri.map(|v| mesh.position(v))) else { continue };
        area3 += twice * 0.5;
        let e = opposite_edges(&q);
        let w = 1.0 / (2.0 * twice).sqrt();
        let id = tri.map(|v| index[&v]);
        rows.push(std::array::from_fn(|k| if k < 3 { (id[k], e[k][0] * w) } else { (n + id[k - 3], -e[k - 3][1] * w) }));
        rows.push(std::array::from_fn(|k| if k < 3 { (id[k], e[k][1] * w) } else { (n + id[k - 3], e[k - 3][0] * w) }));
    }
    if rows.is_empty() {
        return;
    }
    // Pins: the two vertices furthest apart along the projection's longer side.
    let wide = {
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for i in 0..n {
            for k in 0..2 {
                lo[k] = lo[k].min(x[k * n + i]);
                hi[k] = hi[k].max(x[k * n + i]);
            }
        }
        usize::from(hi[1] - lo[1] > hi[0] - lo[0])
    };
    let key = |i: usize| x[wide * n + i];
    let pin_a = (0..n).min_by(|&i, &j| key(i).total_cmp(&key(j))).unwrap_or(0);
    let pin_b = (0..n).max_by(|&i, &j| key(i).total_cmp(&key(j))).unwrap_or(n - 1);
    if pin_a == pin_b {
        return;
    }
    let free = |c: usize| c % n != pin_a && c % n != pin_b;
    // H d = M^T (M d), restricted to the free unknowns.
    let apply = |d: &[f64], out: &mut [f64]| {
        out.iter_mut().for_each(|o| *o = 0.0);
        for row in &rows {
            let s: f64 = row.iter().map(|&(c, a)| a * d[c]).sum();
            for &(c, a) in row {
                out[c] += a * s;
            }
        }
        for (c, o) in out.iter_mut().enumerate() {
            if !free(c) {
                *o = 0.0;
            }
        }
    };
    let mut diag = vec![0.0f64; 2 * n];
    for row in &rows {
        for &(c, a) in row {
            diag[c] += a * a;
        }
    }
    let mut r = vec![0.0f64; 2 * n];
    apply(&x, &mut r);
    r.iter_mut().for_each(|v| *v = -*v);
    let precondition = |r: &[f64], z: &mut [f64]| {
        for c in 0..2 * n {
            z[c] = if free(c) && diag[c] > 0.0 { r[c] / diag[c] } else { 0.0 };
        }
    };
    let mut z = vec![0.0f64; 2 * n];
    precondition(&r, &mut z);
    let mut p = z.clone();
    let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
    let start = rz.abs().max(1e-300);
    let mut hp = vec![0.0f64; 2 * n];
    // Bounded work: about 6e8 multiply-adds per chart, between 200 and 4000 iterations.
    let budget = (600_000_000 / (rows.len() * 12 + 1)).clamp(200, 4000);
    for _ in 0..budget {
        if rz.abs() <= start * 1e-14 {
            break;
        }
        apply(&p, &mut hp);
        let php: f64 = p.iter().zip(&hp).map(|(a, b)| a * b).sum();
        if php <= 0.0 || !php.is_finite() {
            break;
        }
        let alpha = rz / php;
        for c in 0..2 * n {
            x[c] += alpha * p[c];
            r[c] -= alpha * hp[c];
        }
        precondition(&r, &mut z);
        let next: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        let beta = next / rz;
        rz = next;
        for c in 0..2 * n {
            p[c] = z[c] + beta * p[c];
        }
    }
    // Keep the result only if nothing folded; then give it the chart's true area so chart sizes stay comparable.
    let mut flat_area = 0.0f64;
    let mut folds = 0usize;
    for &t in members {
        let id = mesh.triangle(t as usize).map(|v| index[&v]);
        let [a, b, c] = id.map(|i| [x[i], x[n + i]]);
        let s = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
        if !s.is_finite() {
            return;
        }
        if s <= 0.0 {
            folds += 1;
        }
        flat_area += s * 0.5;
    }
    if folds * 200 > members.len() || flat_area <= 0.0 {
        return;
    }
    let scale = (area3 / flat_area).sqrt();
    for (i, v) in verts.iter().enumerate() {
        at.insert(*v, Vec2::new((x[i] * scale) as f32, (x[n + i] * scale) as f32));
    }
}

/// Angle distortion of a UV layout: per triangle the ratio of the larger to the smaller singular value of the map
/// from the surface to UV space (1 is conformal), as an area-weighted mean and the worst triangle.
pub fn stretch(mesh: &Mesh, uvs: &[[f32; 2]]) -> (f32, f32) {
    if uvs.len() != mesh.indices.len() {
        return (0.0, 0.0);
    }
    let (mut sum, mut weight, mut worst) = (0.0f64, 0.0f64, 1.0f64);
    for t in 0..mesh.triangle_count() {
        let Some((q, twice)) = local_triangle(mesh.triangle(t).map(|v| mesh.position(v))) else { continue };
        let e = opposite_edges(&q);
        let uv: [[f64; 2]; 3] = std::array::from_fn(|k| uvs[3 * t + k].map(f64::from));
        // grad f = sum f_i rot90(e_i) / (2A), rot90(x, y) = (-y, x).
        let grad = |c: usize| -> [f64; 2] {
            let (mut gx, mut gy) = (0.0, 0.0);
            for i in 0..3 {
                gx -= uv[i][c] * e[i][1];
                gy += uv[i][c] * e[i][0];
            }
            [gx / twice, gy / twice]
        };
        let ([a, b], [c, d]) = (grad(0), grad(1));
        let half = (a * a + b * b + c * c + d * d) * 0.5;
        let det = (a * d - b * c).abs();
        let root = (half * half - det * det).max(0.0).sqrt();
        let (big, small) = ((half + root).sqrt(), (half - root).max(0.0).sqrt());
        if big <= 1e-12 {
            continue;
        }
        let ratio = if small > big * 1e-6 { big / small } else { 1e6 };
        sum += ratio * twice;
        weight += twice;
        worst = worst.max(ratio);
    }
    if weight <= 0.0 {
        return (0.0, 0.0);
    }
    ((sum / weight) as f32, worst as f32)
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
