//! ZModeler-style actions on quad meshes, aimed at one picked vertex, edge or face: insert an edge loop at any
//! fraction, extrude an open-border edge, close the hole an edge borders, inset or extrude a face, delete a face. Every
//! action keeps the mesh all quads.

use std::collections::{HashMap, HashSet};

use fsc_core::{Mesh, Paint};
use glam::Vec3;

use crate::loops::border_loops;

/// What sits under a surface point on a quad mesh: its quad, the quad edge nearest the point (as it runs in the quad,
/// with its index in the quad), the nearest corner, and whether the edge is an open border.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolyHit {
    pub quad: usize,
    pub corners: [u32; 4],
    pub edge: (u32, u32),
    pub edge_index: usize,
    pub vertex: u32,
    pub border: bool,
}

fn key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-20)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn edge_counts(quads: &[[u32; 4]]) -> HashMap<(u32, u32), u32> {
    let mut count = HashMap::new();
    for f in quads {
        for k in 0..4 {
            *count.entry(key(f[k], f[(k + 1) % 4])).or_insert(0) += 1;
        }
    }
    count
}

/// The quads of a quad mesh, or why there are none.
pub fn quads_of(mesh: &Mesh) -> Result<Vec<[u32; 4]>, String> {
    mesh.quad_faces().ok_or_else(|| "ZModeler actions need a quad mesh; run quad_remesh first".to_string())
}

/// The vertex, edge and face under a point on triangle `triangle` (each quad is two triangles, in order).
pub fn poly_hit(mesh: &Mesh, triangle: usize, p: Vec3) -> Result<PolyHit, String> {
    let quads = quads_of(mesh)?;
    let quad = triangle / 2;
    let f = *quads.get(quad).ok_or("the triangle is outside the mesh")?;
    let edge_index = (0..4)
        .min_by(|&x, &y| {
            let d = |k: usize| segment_distance(p, mesh.position(f[k]), mesh.position(f[(k + 1) % 4]));
            d(x).total_cmp(&d(y))
        })
        .expect("four edges");
    let vertex = *f.iter().min_by(|&&a, &&b| p.distance(mesh.position(a)).total_cmp(&p.distance(mesh.position(b)))).expect("four corners");
    let edge = (f[edge_index], f[(edge_index + 1) % 4]);
    let border = edge_counts(&quads).get(&key(edge.0, edge.1)).copied().unwrap_or(0) == 1;
    Ok(PolyHit { quad, corners: f, edge, edge_index, vertex, border })
}

struct Builder<'a> {
    src: &'a Mesh,
    positions: Vec<[f32; 3]>,
    paint: Vec<Paint>,
    /// Creased edges for the result: the source's, plus any the action adds.
    creases: Vec<(u32, u32)>,
}

impl<'a> Builder<'a> {
    fn new(src: &'a Mesh) -> Builder<'a> {
        Builder { src, positions: src.positions.clone(), paint: src.paint.clone(), creases: src.creases.clone() }
    }

    fn crease(&mut self, a: u32, b: u32) {
        self.creases.push(key(a, b));
    }

    /// A new vertex at `p` painted as vertex `like`.
    fn add(&mut self, p: Vec3, like: u32) -> u32 {
        self.positions.push(p.to_array());
        if self.src.has_paint() {
            self.paint.push(self.src.paint[like as usize]);
        }
        self.positions.len() as u32 - 1
    }

    fn mesh(self, quads: &[[u32; 4]]) -> Mesh {
        let used: HashSet<u32> = quads.iter().flatten().copied().collect();
        // Drops vertices no quad uses any more (after a delete), keeping order.
        let mut remap = vec![u32::MAX; self.positions.len()];
        let (mut positions, mut paint) = (Vec::new(), Vec::new());
        for (v, slot) in remap.iter_mut().enumerate() {
            if used.contains(&(v as u32)) {
                *slot = positions.len() as u32;
                positions.push(self.positions[v]);
                if self.src.has_paint() {
                    paint.push(self.paint[v]);
                }
            }
        }
        let indices: Vec<u32> = quads.iter().flat_map(|q| {
            let r = q.map(|v| remap[v as usize]);
            [r[0], r[1], r[2], r[0], r[2], r[3]]
        }).collect();
        let mut out = Mesh::new(positions, indices);
        out.quads = true;
        if self.src.has_paint() {
            out.paint = paint;
        }
        // Creases kept where both ends survive and they are still quad edges.
        let edges: HashSet<(u32, u32)> = quads.iter().flat_map(|q| (0..4).map(move |k| key(q[k], q[(k + 1) % 4]))).collect();
        let mut creases: Vec<(u32, u32)> = self
            .creases
            .iter()
            .filter(|&&(a, b)| edges.contains(&key(a, b)))
            .map(|&(a, b)| key(remap[a as usize], remap[b as usize]))
            .collect();
        creases.sort_unstable();
        creases.dedup();
        out.creases = creases;
        out
    }
}

/// Inserts an edge loop across the ring of quads through the edge nearest `near`, placed at fraction `at` (0 to 1)
/// across each quad, measured from the same side of the ring all the way round; with `crease` the new loop's edges are
/// creased (auto-crease). A creased edge the loop splits stays creased in both halves. Returns the mesh and the quads
/// split.
pub fn insert_loop_at(mesh: &Mesh, near: Vec3, at: f32, crease: bool) -> Result<(Mesh, usize), String> {
    let quads = quads_of(mesh)?;
    let at = at.clamp(0.02, 0.98);
    let mut edge_quads: HashMap<(u32, u32), Vec<(usize, usize)>> = HashMap::new();
    for (q, f) in quads.iter().enumerate() {
        for k in 0..4 {
            edge_quads.entry(key(f[k], f[(k + 1) % 4])).or_default().push((q, k));
        }
    }
    let (start_q, start_k) = quads
        .iter()
        .enumerate()
        .flat_map(|(q, f)| (0..4).map(move |k| (q, k, f)))
        .min_by(|x, y| {
            let d = |(_, k, f): &(usize, usize, &[u32; 4])| segment_distance(near, mesh.position(f[*k]), mesh.position(f[(*k + 1) % 4]));
            d(x).total_cmp(&d(y))
        })
        .map(|(q, k, _)| (q, k))
        .ok_or("the mesh has no faces")?;
    let mut ring: Vec<(usize, usize)> = vec![(start_q, start_k)];
    let mut seen = vec![false; quads.len()];
    seen[start_q] = true;
    for direction in 0..2 {
        let (mut q, mut k) = (start_q, if direction == 0 { (start_k + 2) % 4 } else { start_k });
        loop {
            let f = quads[q];
            let across = edge_quads.get(&key(f[k], f[(k + 1) % 4])).and_then(|list| list.iter().find(|(o, _)| *o != q).copied());
            let Some((next, nk)) = across else { break };
            if seen[next] {
                break;
            }
            seen[next] = true;
            ring.push((next, nk));
            q = next;
            k = (nk + 2) % 4;
        }
    }
    // Side A of the ring: the start quad's v0 and v3, carried across each shared edge so `at` is measured from one side.
    let mut side: HashSet<u32> = HashSet::new();
    let mut split: HashMap<usize, (usize, bool)> = HashMap::new();
    for &(q, k) in &ring {
        let f = quads[q];
        let v = |i: usize| f[(k + i) % 4];
        let flip = !side.is_empty() && !(side.contains(&v(0)) || side.contains(&v(3)));
        if flip {
            side.insert(v(1));
            side.insert(v(2));
        } else {
            side.insert(v(0));
            side.insert(v(3));
        }
        split.insert(q, (k, flip));
    }
    let mut b = Builder::new(mesh);
    let was_creased: HashSet<(u32, u32)> = mesh.creases.iter().map(|&(x, y)| key(x, y)).collect();
    let mut mids: HashMap<(u32, u32), u32> = HashMap::new();
    let mut mid = |b: &mut Builder, from: u32, to: u32| -> u32 {
        *mids.entry(key(from, to)).or_insert_with(|| {
            let p = mesh.position(from).lerp(mesh.position(to), at);
            let m = b.add(p, if at <= 0.5 { from } else { to });
            if was_creased.contains(&key(from, to)) {
                b.crease(from, m);
                b.crease(m, to);
            }
            m
        })
    };
    let mut out: Vec<[u32; 4]> = Vec::with_capacity(quads.len() + ring.len());
    for (q, f) in quads.iter().enumerate() {
        match split.get(&q) {
            Some(&(k, flip)) => {
                let v = |i: usize| f[(k + i) % 4];
                let (m1, m2) = if flip { (mid(&mut b, v(1), v(0)), mid(&mut b, v(2), v(3))) } else { (mid(&mut b, v(0), v(1)), mid(&mut b, v(3), v(2))) };
                if crease {
                    b.crease(m1, m2);
                }
                out.push([v(0), m1, m2, v(3)]);
                out.push([m1, v(1), v(2), m2]);
            }
            None => out.push(*f),
        }
    }
    Ok((b.mesh(&out), ring.len()))
}

/// Extrudes an open-border edge outward in the plane of its face by `distance`, as one new quad.
pub fn extrude_edge(mesh: &Mesh, hit: &PolyHit, distance: f32) -> Result<Mesh, String> {
    if !hit.border {
        return Err("only an open-border edge extrudes; pick an edge on the mesh's rim".into());
    }
    let quads = quads_of(mesh)?;
    let (a, bv) = hit.edge;
    let (pa, pb) = (mesh.position(a), mesh.position(bv));
    let centre = hit.corners.iter().map(|&v| mesh.position(v)).sum::<Vec3>() / 4.0;
    let along = (pb - pa).normalize_or_zero();
    let mut out = (pa + pb) * 0.5 - centre;
    out -= along * out.dot(along);
    let out = out.try_normalize().ok_or("the face is degenerate")?;
    let mut b = Builder::new(mesh);
    let a2 = b.add(pa + out * distance, a);
    let b2 = b.add(pb + out * distance, bv);
    let mut faces = quads.clone();
    faces.push([bv, a, a2, b2]);
    Ok(b.mesh(&faces))
}

/// Closes the open border the picked edge lies on: four sides with one quad, more (an even number) with a fan of
/// quads around a centre vertex.
pub fn close_border(mesh: &Mesh, hit: &PolyHit) -> Result<(Mesh, usize), String> {
    if !hit.border {
        return Err("pick an edge on the rim of the hole to close".into());
    }
    let quads = quads_of(mesh)?;
    let lp = border_loops(mesh)
        .into_iter()
        .find(|l| (0..l.len()).any(|i| key(l[i], l[(i + 1) % l.len()]) == key(hit.edge.0, hit.edge.1)))
        .ok_or("the edge's border loop was not found")?;
    let n = lp.len();
    if n % 2 == 1 {
        return Err(format!("the hole has {n} sides; quads close an even number (insert an edge loop that reaches it first)"));
    }
    let mut b = Builder::new(mesh);
    let mut faces = quads.clone();
    if n == 4 {
        faces.push([lp[3], lp[2], lp[1], lp[0]]);
    } else {
        let centre = lp.iter().map(|&v| mesh.position(v)).sum::<Vec3>() / n as f32;
        let c = b.add(centre, lp[0]);
        for i in (0..n).step_by(2) {
            faces.push([lp[(i + 2) % n], lp[(i + 1) % n], lp[i], c]);
        }
    }
    let added = faces.len() - quads.len();
    Ok((b.mesh(&faces), added))
}

/// Insets the picked face by `inset` (0 to 0.95 of the way to its centre) and lifts the inner face by `height` along
/// its normal, with a wall quad on each side: inset alone, extrude alone (inset 0) or both. With `crease` the inner
/// face's four edges and the original face's outline are creased (auto-crease).
pub fn inset_face(mesh: &Mesh, hit: &PolyHit, inset: f32, height: f32, crease: bool) -> Result<Mesh, String> {
    let quads = quads_of(mesh)?;
    let f = quads[hit.quad];
    let p: [Vec3; 4] = f.map(|v| mesh.position(v));
    let centre = (p[0] + p[1] + p[2] + p[3]) / 4.0;
    let normal = (p[2] - p[0]).cross(p[3] - p[1]).try_normalize().ok_or("the face is degenerate")?;
    let mut b = Builder::new(mesh);
    let inner: [u32; 4] = std::array::from_fn(|i| b.add(p[i].lerp(centre, inset.clamp(0.0, 0.95)) + normal * height, f[i]));
    let mut faces: Vec<[u32; 4]> = quads.iter().enumerate().filter(|(q, _)| *q != hit.quad).map(|(_, f)| *f).collect();
    for i in 0..4 {
        let j = (i + 1) % 4;
        faces.push([f[i], f[j], inner[j], inner[i]]);
        if crease {
            b.crease(inner[i], inner[j]);
            b.crease(f[i], f[j]);
        }
    }
    faces.push(inner);
    Ok(b.mesh(&faces))
}

/// The creases after creasing (`on`) or uncreasing the picked edge, or with `around_point` every quad edge that meets
/// the picked corner. Returns the new list and how many edges changed.
pub fn set_creases(mesh: &Mesh, hit: &PolyHit, around_point: bool, on: bool) -> Result<(Vec<(u32, u32)>, usize), String> {
    let quads = quads_of(mesh)?;
    let targets: HashSet<(u32, u32)> = if around_point {
        quads.iter().flat_map(|q| (0..4).map(move |k| (q[k], q[(k + 1) % 4]))).filter(|&(a, b)| a == hit.vertex || b == hit.vertex).map(|(a, b)| key(a, b)).collect()
    } else {
        [key(hit.edge.0, hit.edge.1)].into_iter().collect()
    };
    let mut now: HashSet<(u32, u32)> = mesh.creases.iter().map(|&(a, b)| key(a, b)).collect();
    let before = now.len();
    for e in &targets {
        if on {
            now.insert(*e);
        } else {
            now.remove(e);
        }
    }
    let changed = now.len().abs_diff(before);
    let mut list: Vec<(u32, u32)> = now.into_iter().collect();
    list.sort_unstable();
    Ok((list, changed))
}

/// Deletes the picked face, opening the mesh there.
pub fn delete_face(mesh: &Mesh, hit: &PolyHit) -> Result<Mesh, String> {
    let quads = quads_of(mesh)?;
    if quads.len() <= 1 {
        return Err("that would delete every face; delete the object instead".into());
    }
    let faces: Vec<[u32; 4]> = quads.iter().enumerate().filter(|(q, _)| *q != hit.quad).map(|(_, f)| *f).collect();
    Ok(Builder::new(mesh).mesh(&faces))
}
