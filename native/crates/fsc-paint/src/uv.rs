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

/// Where chart borders (seams) go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Charts {
    /// Faces grouped by the axis they face most.
    Direction,
    /// One chart per connected piece of each face group, so painted groups place the seams; a piece that will not
    /// lie flat without folding (a closed shell, a tube) is split further by facing direction.
    Groups,
}

pub fn unwrap_with(mesh: &Mesh, padding: f32, flatten: Flatten) -> Unwrap {
    unwrap_charts(mesh, padding, flatten, Charts::Direction)
}

/// Connected pieces of `faces` (by shared edges) whose faces agree on `key`.
fn components(faces: &[u32], key: &[u32], neighbors: &dyn Fn(usize) -> Vec<usize>, tris: usize) -> Vec<Vec<u32>> {
    let mut inside = vec![false; tris];
    for &t in faces {
        inside[t as usize] = true;
    }
    let mut seen = vec![false; tris];
    let mut out = Vec::new();
    for &seed in faces {
        if seen[seed as usize] {
            continue;
        }
        seen[seed as usize] = true;
        let mut members = vec![seed];
        let mut at = 0;
        while at < members.len() {
            let t = members[at] as usize;
            at += 1;
            for f in neighbors(t) {
                if inside[f] && !seen[f] && key[f] == key[seed as usize] {
                    seen[f] = true;
                    members.push(f as u32);
                }
            }
        }
        out.push(members);
    }
    out
}

/// The facing label for a whole chart: the axis of its area-weighted normal.
fn chart_label(mesh: &Mesh, members: &[u32]) -> u8 {
    let mut sum = Vec3::ZERO;
    for &t in members {
        let [a, b, c] = mesh.triangle(t as usize).map(|v| mesh.position(v));
        sum += (b - a).cross(c - a);
    }
    label_of(sum.normalize_or(Vec3::Y))
}

fn projected(mesh: &Mesh, members: &[u32], label: u8) -> HashMap<u32, Vec2> {
    let mut at: HashMap<u32, Vec2> = HashMap::new();
    for &t in members {
        for v in mesh.triangle(t as usize) {
            at.entry(v).or_insert_with(|| project(mesh.position(v), label));
        }
    }
    at
}

pub fn unwrap_charts(mesh: &Mesh, padding: f32, flatten: Flatten, by: Charts) -> Unwrap {
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
    let all: Vec<u32> = (0..tris as u32).collect();
    let label_key: Vec<u32> = labels.iter().map(|&l| u32::from(l)).collect();
    let by_groups = by == Charts::Groups && mesh.has_groups();
    // A projected group still has to face one way, so projection splits groups by direction from the start.
    let start_key: Vec<u32> = match (by_groups, flatten) {
        (false, _) => label_key.clone(),
        (true, Flatten::Conformal) => mesh.groups.clone(),
        (true, Flatten::Project) => mesh.groups.iter().zip(&labels).map(|(&g, &l)| g.wrapping_mul(6).wrapping_add(u32::from(l))).collect(),
    };
    // Each chart's faces and flat coordinates per mesh vertex (a seam vertex has one in each chart it borders).
    let mut charts: Vec<Vec<u32>> = Vec::new();
    let mut flat: Vec<HashMap<u32, Vec2>> = Vec::new();
    for members in components(&all, &start_key, &neighbors, tris) {
        let label = if by_groups { chart_label(mesh, &members) } else { labels[members[0] as usize] };
        let mut at = projected(mesh, &members, label);
        if flatten == Flatten::Conformal && !conformal(mesh, &members, &mut at) && by_groups {
            let pieces = components(&members, &label_key, &neighbors, tris);
            if pieces.len() > 1 {
                for piece in pieces {
                    let mut at = projected(mesh, &piece, labels[piece[0] as usize]);
                    conformal(mesh, &piece, &mut at);
                    charts.push(piece);
                    flat.push(at);
                }
                continue;
            }
        }
        charts.push(members);
        flat.push(at);
    }
    let corners: Vec<Vec<usize>> = charts.iter().map(|members| members.iter().flat_map(|&t| (0..3).map(move |k| 3 * t as usize + k)).collect()).collect();
    let mut coords = vec![Vec2::ZERO; mesh.indices.len()];
    for (c, members) in charts.iter().enumerate() {
        for &t in members {
            for k in 0..3 {
                coords[3 * t as usize + k] = flat[c][&mesh.indices[3 * t as usize + k]];
            }
        }
    }
    let coverage = pack(&corners, &mut coords, padding);
    Unwrap { uvs: coords.iter().map(|q| q.to_array()).collect(), charts: charts.len(), coverage }
}

/// Packs charts (each a list of corner indices into `coords`) into the unit square by rows, tallest first, keeping
/// their relative sizes, and rewrites `coords` in place; returns the share of the square their boxes cover.
pub fn pack(charts: &[Vec<usize>], coords: &mut [Vec2], padding: f32) -> f32 {
    let mut boxes: Vec<(Vec2, Vec2)> = Vec::with_capacity(charts.len());
    let mut area = 0.0;
    for corners in charts {
        let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
        for &c in corners {
            lo = lo.min(coords[c]);
            hi = hi.max(coords[c]);
        }
        let size = (hi - lo).max(Vec2::splat(1e-6));
        area += size.x * size.y;
        boxes.push((lo, size));
    }
    if charts.is_empty() {
        return 0.0;
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
    for (c, corners) in charts.iter().enumerate() {
        for &k in corners {
            coords[k] = (coords[k] - boxes[c].0 + offsets[c]) / side;
        }
    }
    area / (side * side)
}

/// A picture of a UV layout, `size` pixels square with v up: each island filled in its own muted hue with its
/// triangle edges drawn over it, the `highlight` island brighter, on the window's dark background. `island` is
/// [`islands`]'s result for the mesh.
pub fn layout_image(mesh: &Mesh, island: &[u32], size: usize, highlight: Option<u32>) -> Vec<[u8; 4]> {
    let mut px = vec![[28u8, 27, 29, 255]; size * size];
    if island.len() != mesh.triangle_count() {
        return px;
    }
    let s = size as f32;
    let to_px = |uv: [f32; 2]| Vec2::new(uv[0] * s, (1.0 - uv[1]) * s);
    let hue = |i: u32| -> [f32; 3] {
        let h = (i as f32 * 0.618_034).fract() * 6.0;
        let x = 1.0 - ((h % 2.0) - 1.0).abs();
        match h as u32 {
            0 => [1.0, x, 0.0],
            1 => [x, 1.0, 0.0],
            2 => [0.0, 1.0, x],
            3 => [0.0, x, 1.0],
            4 => [x, 0.0, 1.0],
            _ => [1.0, 0.0, x],
        }
    };
    let mix = |c: [f32; 3], k: f32, base: f32| -> [u8; 4] {
        let f = |v: f32| ((base + v * k) * 255.0).clamp(0.0, 255.0) as u8;
        [f(c[0]), f(c[1]), f(c[2]), 255]
    };
    for (t, &own) in island.iter().enumerate() {
        let q = [0, 1, 2].map(|k| to_px(mesh.uvs[3 * t + k]));
        let lit = highlight == Some(own);
        let fill = mix(hue(own), if lit { 0.55 } else { 0.28 }, if lit { 0.25 } else { 0.12 });
        let (lo, hi) = (q[0].min(q[1]).min(q[2]), q[0].max(q[1]).max(q[2]));
        let area = (q[1] - q[0]).perp_dot(q[2] - q[0]);
        if area.abs() > 1e-9 {
            for y in (lo.y.floor().max(0.0) as usize)..(hi.y.ceil().min(s) as usize) {
                for x in (lo.x.floor().max(0.0) as usize)..(hi.x.ceil().min(s) as usize) {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = (q[1] - p).perp_dot(q[2] - p) / area;
                    let w1 = (q[2] - p).perp_dot(q[0] - p) / area;
                    if w0 >= -1e-4 && w1 >= -1e-4 && w0 + w1 <= 1.0 + 1e-4 {
                        px[y * size + x] = fill;
                    }
                }
            }
        }
    }
    // Edges after every fill, so a neighbour's fill does not cover them.
    for (t, &own) in island.iter().enumerate() {
        let q = [0, 1, 2].map(|k| to_px(mesh.uvs[3 * t + k]));
        let line = mix(hue(own), 0.6, if highlight == Some(own) { 0.4 } else { 0.25 });
        for k in 0..3 {
            let (a, b) = (q[k], q[(k + 1) % 3]);
            let steps = (b - a).abs().max_element().ceil().max(1.0) as usize;
            for i in 0..=steps {
                let p = a + (b - a) * (i as f32 / steps as f32);
                let (x, y) = (p.x.floor() as isize, p.y.floor() as isize);
                if x >= 0 && y >= 0 && (x as usize) < size && (y as usize) < size {
                    px[y as usize * size + x as usize] = line;
                }
            }
        }
    }
    px
}

/// The island under a UV point: the first triangle containing it, or None over empty space.
pub fn island_at(mesh: &Mesh, island: &[u32], uv: [f32; 2]) -> Option<u32> {
    if island.len() != mesh.triangle_count() {
        return None;
    }
    let p = Vec2::from_array(uv);
    (0..mesh.triangle_count()).find_map(|t| {
        let q = [0, 1, 2].map(|k| Vec2::from_array(mesh.uvs[3 * t + k]));
        let area = (q[1] - q[0]).perp_dot(q[2] - q[0]);
        if area.abs() < 1e-14 {
            return None;
        }
        let w0 = (q[1] - p).perp_dot(q[2] - p) / area;
        let w1 = (q[2] - p).perp_dot(q[0] - p) / area;
        (w0 >= 0.0 && w1 >= 0.0 && w0 + w1 <= 1.0).then_some(island[t])
    })
}

/// A triangle on an edge with its UVs at the edge's lower and higher vertex.
type EdgeUv = (u32, [f32; 2], [f32; 2]);

/// UV islands: faces joined across an edge whose two ends have the same UVs on both sides. One id per triangle,
/// numbered in order of first appearance; empty when the mesh has no UVs.
pub fn islands(mesh: &Mesh) -> Vec<u32> {
    let tris = mesh.triangle_count();
    if mesh.uvs.len() != mesh.indices.len() {
        return Vec::new();
    }
    let same = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6;
    // Each directed corner pair keyed by its vertices, so the face across an edge can be found with its UVs.
    let mut edges: HashMap<(u32, u32), Vec<EdgeUv>> = HashMap::with_capacity(tris * 2);
    for t in 0..tris {
        for k in 0..3 {
            let (i, j) = (3 * t + k, 3 * t + (k + 1) % 3);
            let (a, b) = (mesh.indices[i], mesh.indices[j]);
            let (ua, ub) = if a < b { (mesh.uvs[i], mesh.uvs[j]) } else { (mesh.uvs[j], mesh.uvs[i]) };
            edges.entry((a.min(b), a.max(b))).or_default().push((t as u32, ua, ub));
        }
    }
    let mut id = vec![u32::MAX; tris];
    let mut next = 0;
    for seed in 0..tris {
        if id[seed] != u32::MAX {
            continue;
        }
        id[seed] = next;
        let mut stack = vec![seed];
        while let Some(t) = stack.pop() {
            for k in 0..3 {
                let (a, b) = (mesh.indices[3 * t + k], mesh.indices[3 * t + (k + 1) % 3]);
                let Some(list) = edges.get(&(a.min(b), a.max(b))) else { continue };
                let Some(&(_, ua, ub)) = list.iter().find(|e| e.0 as usize == t) else { continue };
                for &(f, fa, fb) in list {
                    if id[f as usize] == u32::MAX && same(ua, fa) && same(ub, fb) {
                        id[f as usize] = next;
                        stack.push(f as usize);
                    }
                }
            }
        }
        next += 1;
    }
    id
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
/// folds over itself keeps its projection and returns false (a closed shell or a tube cannot lie flat in one piece);
/// true otherwise, including charts too small to need solving.
fn conformal(mesh: &Mesh, members: &[u32], at: &mut HashMap<u32, Vec2>) -> bool {
    let mut verts: Vec<u32> = at.keys().copied().collect();
    verts.sort_unstable();
    let n = verts.len();
    if n < 4 {
        return true;
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
        return true;
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
        return true;
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
            return false;
        }
        if s <= 0.0 {
            folds += 1;
        }
        flat_area += s * 0.5;
    }
    if folds * 200 > members.len() || flat_area <= 0.0 {
        return false;
    }
    let scale = (area3 / flat_area).sqrt();
    for (i, v) in verts.iter().enumerate() {
        at.insert(*v, Vec2::new((x[i] * scale) as f32, (x[n + i] * scale) as f32));
    }
    true
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
