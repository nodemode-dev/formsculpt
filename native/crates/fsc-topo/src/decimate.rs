use std::cmp::Reverse;
use std::collections::BinaryHeap;

use fsc_core::Mesh;
use glam::{DVec3, Vec3};

use crate::edges::boundary_edges;

type Heap = BinaryHeap<Reverse<(u64, u32, u32, u32, u32)>>;

#[derive(Clone, Copy, Default)]
struct Quadric([f64; 10]);

impl Quadric {
    fn plane(n: DVec3, d: f64, w: f64) -> Quadric {
        let (a, b, c) = (n.x, n.y, n.z);
        Quadric([a * a * w, a * b * w, a * c * w, a * d * w, b * b * w, b * c * w, b * d * w, c * c * w, c * d * w, d * d * w])
    }

    fn add(&self, o: &Quadric) -> Quadric {
        let mut q = [0.0; 10];
        for (i, v) in q.iter_mut().enumerate() {
            *v = self.0[i] + o.0[i];
        }
        Quadric(q)
    }

    fn error(&self, p: DVec3) -> f64 {
        let q = &self.0;
        let (x, y, z) = (p.x, p.y, p.z);
        q[0] * x * x + 2.0 * q[1] * x * y + 2.0 * q[2] * x * z + 2.0 * q[3] * x + q[4] * y * y + 2.0 * q[5] * y * z + 2.0 * q[6] * y + q[7] * z * z + 2.0 * q[8] * z + q[9]
    }
}

/// What a decimation keeps beyond the shape.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DecimateOptions {
    /// Open borders held in place (as before: steep planes along each border edge).
    pub preserve_borders: bool,
    /// Vertex paint carried to the vertices that stay, and edges across a colour change made dearer to collapse.
    pub preserve_paint: bool,
    /// Face groups carried per triangle, and the borders between groups held like open borders.
    pub preserve_groups: bool,
    /// UVs carried per corner: vertices on a UV seam never move, the others take the UV of where they collapse to.
    pub preserve_uvs: bool,
}

pub fn decimate(mesh: &Mesh, target_triangles: usize, preserve_borders: bool) -> Mesh {
    decimate_with(mesh, target_triangles, DecimateOptions { preserve_borders, ..Default::default() })
}

/// Quadric edge-collapse decimation to `target_triangles`, keeping what `opts` asks for. Hidden flags always carry
/// per triangle. With seams or groups kept the target may not be reached: protected vertices do not collapse.
pub fn decimate_with(mesh: &Mesh, target_triangles: usize, opts: DecimateOptions) -> Mesh {
    let preserve_borders = opts.preserve_borders;
    let nv = mesh.vertex_count();
    let nf = mesh.triangle_count();
    if target_triangles >= nf {
        return mesh.clone();
    }
    let mut pos: Vec<DVec3> = mesh.positions.iter().map(|p| DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64)).collect();
    let mut tris: Vec<[u32; 3]> = (0..nf).map(|t| mesh.triangle(t)).collect();
    let mut alive_f = vec![true; nf];
    let mut alive_v = vec![true; nv];
    let mut vfaces: Vec<Vec<u32>> = vec![Vec::new(); nv];
    let mut quad = vec![Quadric::default(); nv];
    for (t, tri) in tris.iter().enumerate() {
        let (a, b, c) = (pos[tri[0] as usize], pos[tri[1] as usize], pos[tri[2] as usize]);
        let n = (b - a).cross(c - a);
        let area = n.length();
        let n = if area > 0.0 { n / area } else { DVec3::Y };
        let q = Quadric::plane(n, -n.dot(a), area.max(1e-12));
        for &v in tri {
            quad[v as usize] = quad[v as usize].add(&q);
            vfaces[v as usize].push(t as u32);
        }
    }
    if preserve_borders {
        for (a, b) in boundary_edges(mesh) {
            let (pa, pb) = (pos[a as usize], pos[b as usize]);
            let t = vfaces[a as usize].iter().copied().find(|&f| tris[f as usize].contains(&b));
            let Some(t) = t else { continue };
            let [x, y, z] = tris[t as usize];
            let fnrm = (pos[y as usize] - pos[x as usize]).cross(pos[z as usize] - pos[x as usize]).normalize_or_zero();
            let side = (pb - pa).cross(fnrm).normalize_or_zero();
            let q = Quadric::plane(side, -side.dot(pa), (pb - pa).length_squared() * 1000.0);
            quad[a as usize] = quad[a as usize].add(&q);
            quad[b as usize] = quad[b as usize].add(&q);
        }
    }
    let groups = opts.preserve_groups && mesh.has_groups();
    if groups {
        // An edge between two faces of different groups gets the same steep side planes an open border gets.
        let mut by_edge: std::collections::HashMap<(u32, u32), Vec<u32>> = std::collections::HashMap::new();
        for (t, tri) in tris.iter().enumerate() {
            for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                by_edge.entry((a.min(b), a.max(b))).or_default().push(t as u32);
            }
        }
        for (&(a, b), fs) in &by_edge {
            if fs.len() != 2 || mesh.groups[fs[0] as usize] == mesh.groups[fs[1] as usize] {
                continue;
            }
            let (pa, pb) = (pos[a as usize], pos[b as usize]);
            for &f in fs {
                let [x, y, z] = tris[f as usize];
                let fnrm = (pos[y as usize] - pos[x as usize]).cross(pos[z as usize] - pos[x as usize]).normalize_or_zero();
                let side = (pb - pa).cross(fnrm).normalize_or_zero();
                let q = Quadric::plane(side, -side.dot(pa), (pb - pa).length_squared() * 1000.0);
                quad[a as usize] = quad[a as usize].add(&q);
                quad[b as usize] = quad[b as usize].add(&q);
            }
        }
    }
    // UVs: a vertex whose corners disagree lies on a seam and is locked; every other vertex has one UV.
    let uvs = opts.preserve_uvs && mesh.uvs.len() == mesh.indices.len() && !mesh.indices.is_empty();
    let mut locked = vec![false; nv];
    let mut vuv: Vec<Option<[f32; 2]>> = vec![None; if uvs { nv } else { 0 }];
    if uvs {
        for (c, &v) in mesh.indices.iter().enumerate() {
            let uv = mesh.uvs[c];
            match vuv[v as usize] {
                None => vuv[v as usize] = Some(uv),
                Some(o) if (o[0] - uv[0]).abs() > 1e-6 || (o[1] - uv[1]).abs() > 1e-6 => locked[v as usize] = true,
                _ => {}
            }
        }
    }
    let paint = opts.preserve_paint && mesh.has_paint();
    let mut vpaint: Vec<fsc_core::Paint> = if paint { mesh.paint.clone() } else { Vec::new() };
    let mut version = vec![0u32; nv];
    let mut heap: Heap = BinaryHeap::new();
    let best = |q: &Quadric, a: DVec3, b: DVec3| -> (f64, DVec3) {
        let m = (a + b) * 0.5;
        [m, a, b].into_iter().map(|p| (q.error(p).max(0.0), p)).min_by(|x, y| x.0.total_cmp(&y.0)).unwrap_or((0.0, m))
    };
    // An edge across a colour change costs extra in proportion to the change and the edge's length, so painted
    // boundaries collapse last.
    let paint_cost = |vpaint: &[fsc_core::Paint], pos: &[DVec3], u: u32, v: u32| -> f64 {
        if !paint {
            return 0.0;
        }
        let (a, b) = (vpaint[u as usize], vpaint[v as usize]);
        let diff: f64 = (0..3).map(|c| ((a[c] - b[c]) as f64).abs()).sum();
        diff * pos[u as usize].distance_squared(pos[v as usize]) * 10.0
    };
    let push = |heap: &mut Heap, quad: &[Quadric], pos: &[DVec3], version: &[u32], vpaint: &[fsc_core::Paint], u: u32, v: u32| {
        if locked[u as usize] || locked[v as usize] {
            return;
        }
        let q = quad[u as usize].add(&quad[v as usize]);
        let (cost, _) = best(&q, pos[u as usize], pos[v as usize]);
        let cost = cost + paint_cost(vpaint, pos, u, v);
        heap.push(Reverse(((cost as f32).to_bits() as u64, u, v, version[u as usize], version[v as usize])));
    };
    for tri in &tris {
        for (u, v) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            if u < v {
                push(&mut heap, &quad, &pos, &version, &vpaint, u, v);
            }
        }
    }
    let mut faces_alive = nf;
    let mut scratch_u: Vec<u32> = Vec::new();
    let mut scratch_v: Vec<u32> = Vec::new();
    while faces_alive > target_triangles {
        let Some(Reverse((_, u, v, vu, vv))) = heap.pop() else { break };
        if !alive_v[u as usize] || !alive_v[v as usize] || version[u as usize] != vu || version[v as usize] != vv {
            continue;
        }
        let q = quad[u as usize].add(&quad[v as usize]);
        let (_, p) = best(&q, pos[u as usize], pos[v as usize]);
        let neighbors = |w: u32, out: &mut Vec<u32>, vfaces: &Vec<Vec<u32>>, tris: &Vec<[u32; 3]>, alive_f: &Vec<bool>| {
            out.clear();
            for &f in &vfaces[w as usize] {
                if alive_f[f as usize] {
                    out.extend(tris[f as usize].iter().copied().filter(|&x| x != w));
                }
            }
            out.sort_unstable();
            out.dedup();
        };
        neighbors(u, &mut scratch_u, &vfaces, &tris, &alive_f);
        neighbors(v, &mut scratch_v, &vfaces, &tris, &alive_f);
        let common = scratch_u.iter().filter(|x| scratch_v.binary_search(x).is_ok()).count();
        let shared: Vec<u32> = vfaces[u as usize].iter().copied().filter(|&f| alive_f[f as usize] && tris[f as usize].contains(&v)).collect();
        if common != shared.len() || shared.is_empty() {
            continue;
        }
        let mut flips = false;
        for &w in &[u, v] {
            for &f in &vfaces[w as usize] {
                if !alive_f[f as usize] || shared.contains(&f) {
                    continue;
                }
                let tri = tris[f as usize];
                let old = |k: u32| pos[k as usize];
                let new = |k: u32| if k == u || k == v { p } else { pos[k as usize] };
                let n0 = (old(tri[1]) - old(tri[0])).cross(old(tri[2]) - old(tri[0]));
                let n1 = (new(tri[1]) - new(tri[0])).cross(new(tri[2]) - new(tri[0]));
                if n1.length_squared() < 1e-24 || n0.normalize_or_zero().dot(n1.normalize_or_zero()) < 0.2 {
                    flips = true;
                    break;
                }
            }
            if flips {
                break;
            }
        }
        if flips {
            continue;
        }
        for &f in &shared {
            alive_f[f as usize] = false;
            faces_alive -= 1;
        }
        let moved: Vec<u32> = vfaces[v as usize].clone();
        for f in moved {
            if !alive_f[f as usize] {
                continue;
            }
            for k in tris[f as usize].iter_mut() {
                if *k == v {
                    *k = u;
                }
            }
            vfaces[u as usize].push(f);
        }
        vfaces[u as usize].retain(|&f| alive_f[f as usize]);
        vfaces[v as usize].clear();
        alive_v[v as usize] = false;
        // Paint and UV follow the chosen point: u's, v's, or their average at the midpoint.
        let (pu, pv) = (pos[u as usize], pos[v as usize]);
        let t = if p == pu { 0.0 } else if p == pv { 1.0 } else { 0.5 };
        if paint {
            vpaint[u as usize] = fsc_core::mix_paint(vpaint[u as usize], vpaint[v as usize], t);
        }
        if uvs {
            if let (Some(a), Some(b)) = (vuv[u as usize], vuv[v as usize]) {
                vuv[u as usize] = Some([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        }
        pos[u as usize] = p;
        quad[u as usize] = q;
        version[u as usize] += 1;
        neighbors(u, &mut scratch_u, &vfaces, &tris, &alive_f);
        for &w in &scratch_u.clone() {
            push(&mut heap, &quad, &pos, &version, &vpaint, u.min(w), u.max(w));
        }
    }
    let mut remap = vec![u32::MAX; nv];
    let mut positions = Vec::new();
    let mut indices = Vec::with_capacity(faces_alive * 3);
    let (mut out_paint, mut out_groups, mut out_hidden, mut out_uvs) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let hidden = mesh.has_hidden();
    for (t, tri) in tris.iter().enumerate() {
        if !alive_f[t] {
            continue;
        }
        for (corner, &k) in tri.iter().enumerate() {
            if remap[k as usize] == u32::MAX {
                remap[k as usize] = positions.len() as u32;
                let p = pos[k as usize];
                positions.push(Vec3::new(p.x as f32, p.y as f32, p.z as f32).to_array());
                if paint {
                    out_paint.push(vpaint[k as usize]);
                }
            }
            indices.push(remap[k as usize]);
            if uvs {
                // A seam vertex never moved, so this triangle's own corner UV for it still holds when the triangle
                // still has its original corner there; otherwise the vertex has a single UV.
                let original = mesh.triangle(t)[corner] == k;
                out_uvs.push(if locked[k as usize] && original { mesh.uvs[3 * t + corner] } else { vuv[k as usize].unwrap_or(mesh.uvs[3 * t + corner]) });
            }
        }
        if groups {
            out_groups.push(mesh.groups[t]);
        }
        if hidden {
            out_hidden.push(mesh.hidden[t]);
        }
    }
    let mut out = Mesh::new(positions, indices);
    out.paint = out_paint;
    out.groups = out_groups;
    out.hidden = out_hidden;
    out.uvs = out_uvs;
    out
}
