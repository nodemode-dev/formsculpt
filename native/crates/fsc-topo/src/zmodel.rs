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

/// An edge loop's vertices in order, each one's neighbour on the picked side, and whether it closes.
pub type EdgeLoop = (Vec<u32>, Vec<Option<u32>>, bool);

/// The edge loop through the picked edge: its vertices in order, each one's neighbour on the side of the picked face
/// (None where the loop leaves that side), and whether the loop closes. The walk goes through four-edge vertices and
/// stops at a pole (any other count), at an open border, or where it would repeat.
pub fn edge_loop(mesh: &Mesh, hit: &PolyHit) -> Result<EdgeLoop, String> {
    let quads = quads_of(mesh)?;
    let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    let mut ring: HashMap<u32, HashSet<u32>> = HashMap::new();
    for (q, f) in quads.iter().enumerate() {
        for k in 0..4 {
            let (a, b) = (f[k], f[(k + 1) % 4]);
            by_edge.entry(key(a, b)).or_default().push(q);
            ring.entry(a).or_default().insert(b);
            ring.entry(b).or_default().insert(a);
        }
    }
    // The neighbour of v in quad q that is not `not`.
    let adj = |q: usize, v: u32, not: u32| -> Option<u32> {
        let f = quads[q];
        let i = f.iter().position(|&x| x == v)?;
        [f[(i + 1) % 4], f[(i + 3) % 4]].into_iter().find(|&x| x != not)
    };
    let (a, b) = hit.edge;
    let mut side: HashMap<u32, u32> = HashMap::new();
    side.insert(a, adj(hit.quad, a, b).ok_or("the face is broken")?);
    side.insert(b, adj(hit.quad, b, a).ok_or("the face is broken")?);
    // One direction of the walk from `v` (reached from `prev` across the side quad `q`); returns the vertices added.
    let walk = |start: u32, from: u32, q0: usize, side: &mut HashMap<u32, u32>, stop: u32| -> (Vec<u32>, bool) {
        let (mut v, mut prev, mut q) = (start, from, q0);
        let mut added = Vec::new();
        loop {
            if ring.get(&v).map_or(0, |r| r.len()) != 4 {
                return (added, false);
            }
            let Some(&sv) = side.get(&v) else { return (added, false) };
            let Some(&q2) = by_edge.get(&key(v, sv)).and_then(|l| l.iter().find(|&&o| o != q)) else { return (added, false) };
            let Some(c) = adj(q2, v, sv) else { return (added, false) };
            if c == prev {
                return (added, false);
            }
            if c == stop {
                return (added, true);
            }
            if side.contains_key(&c) {
                return (added, false);
            }
            let Some(sc) = adj(q2, c, v) else { return (added, false) };
            side.insert(c, sc);
            added.push(c);
            prev = v;
            v = c;
            q = q2;
        }
    };
    let (forward, closed) = walk(b, a, hit.quad, &mut side, a);
    let mut order = vec![a, b];
    order.extend(forward);
    if !closed {
        let (back, _) = walk(a, b, hit.quad, &mut side, b);
        let mut front: Vec<u32> = back.into_iter().rev().collect();
        front.extend(order);
        order = front;
    }
    let sides = order.iter().map(|v| side.get(v).copied()).collect();
    Ok((order, sides, closed))
}

/// New positions for the picked edge loop: `align` puts an open loop on the straight line between its ends and a
/// closed loop on its best-fit plane; `slide` moves each vertex toward its neighbour on the picked face's side by
/// `amount` (0 to 0.95). Returns (vertex, new position) pairs for the vertices that move.
pub fn loop_moves(mesh: &Mesh, hit: &PolyHit, align: bool, amount: f32) -> Result<Vec<(u32, Vec3)>, String> {
    let (order, sides, closed) = edge_loop(mesh, hit)?;
    let p: Vec<Vec3> = order.iter().map(|&v| mesh.position(v)).collect();
    let mut out = Vec::new();
    if align {
        if closed {
            let c = p.iter().copied().sum::<Vec3>() / p.len() as f32;
            // Newell's normal of the loop polygon.
            let n = (0..p.len()).map(|i| (p[i] - c).cross(p[(i + 1) % p.len()] - c)).sum::<Vec3>().try_normalize().ok_or("the loop is flat to a line")?;
            for (i, &v) in order.iter().enumerate() {
                out.push((v, p[i] - n * (p[i] - c).dot(n)));
            }
        } else {
            let (s, e) = (p[0], p[p.len() - 1]);
            let dir = (e - s).try_normalize().ok_or("the loop's ends meet")?;
            for (i, &v) in order.iter().enumerate() {
                out.push((v, s + dir * (p[i] - s).dot(dir)));
            }
        }
    } else {
        let t = amount.clamp(0.0, 0.95);
        for (i, &v) in order.iter().enumerate() {
            if let Some(s) = sides[i] {
                out.push((v, p[i].lerp(mesh.position(s), t)));
            }
        }
    }
    out.retain(|(v, q)| mesh.position(*v).distance(*q) > 1e-9);
    Ok(out)
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

/// Splits every quad that has two of the given edges as opposite sides, through the new vertex on each (`mids`, keyed
/// by edge), so a strip of quads gains a loop along it. Quads with none of the edges pass through unchanged.
fn split_by_mids(quads: &[[u32; 4]], mids: &HashMap<(u32, u32), u32>) -> Result<Vec<[u32; 4]>, String> {
    let mut out = Vec::with_capacity(quads.len() * 2);
    for f in quads {
        let hits = (0..4).filter(|&k| mids.contains_key(&key(f[k], f[(k + 1) % 4]))).count();
        if hits == 0 {
            out.push(*f);
            continue;
        }
        let k = (0..4)
            .find(|&k| mids.contains_key(&key(f[k], f[(k + 1) % 4])) && mids.contains_key(&key(f[(k + 3) % 4], f[(k + 2) % 4])))
            .filter(|_| hits == 2)
            .ok_or("a face would get a vertex on only one side; the loop must run through quads only")?;
        let v = |i: usize| f[(k + i) % 4];
        let m1 = mids[&key(v(0), v(1))];
        let m2 = mids[&key(v(3), v(2))];
        out.push([v(0), m1, m2, v(3)]);
        out.push([m1, v(1), v(2), m2]);
    }
    Ok(out)
}

/// Edge inset: supporting loops on both sides of the edge loop through the picked edge, each `amount` of the shortest
/// edge leaving the loop away from it (the same distance all along, ZModeler's equidistant spacing). With `keep_centre`
/// false the original loop is removed between them (border only). With `crease` the new loops are creased. The loop
/// must close or end on open borders: at a pole the side loops would leave a face with a vertex on one side only.
/// Returns the mesh and the loop's vertex count.
pub fn inset_edge(mesh: &Mesh, hit: &PolyHit, amount: f32, keep_centre: bool, crease: bool) -> Result<(Mesh, usize), String> {
    let quads = quads_of(mesh)?;
    let counts = edge_counts(&quads);
    let (order, side_a, closed) = edge_loop(mesh, hit)?;
    // The quad across the picked edge gives the loop's other side.
    let other = quads
        .iter()
        .enumerate()
        .find(|(q, f)| *q != hit.quad && (0..4).any(|k| key(f[k], f[(k + 1) % 4]) == key(hit.edge.0, hit.edge.1)))
        .map(|(q, _)| q);
    let side_b: Vec<Option<u32>> = match other {
        Some(q) => {
            let flip = PolyHit { quad: q, corners: quads[q], edge: hit.edge, edge_index: 0, vertex: hit.vertex, border: false };
            let (o2, s2, _) = edge_loop(mesh, &flip)?;
            order.iter().map(|v| o2.iter().position(|x| x == v).and_then(|i| s2[i])).collect()
        }
        None => vec![None; order.len()],
    };
    let n = order.len();
    if !closed {
        let on_border = |v: u32| quads.iter().flat_map(|f| (0..4).map(move |k| (f[k], f[(k + 1) % 4]))).any(|(a, b)| (a == v || b == v) && counts.get(&key(a, b)) == Some(&1));
        if !on_border(order[0]) || !on_border(order[n - 1]) {
            return Err("the edge loop ends at a pole (a corner with other than four edges); edge inset needs a loop that closes or runs border to border".into());
        }
    }
    let p = |v: u32| mesh.position(v);
    let shortest = order
        .iter()
        .enumerate()
        .flat_map(|(i, &v)| [side_a[i], side_b[i]].into_iter().flatten().map(move |s| p(v).distance(p(s))))
        .fold(f32::INFINITY, f32::min);
    if !shortest.is_finite() || shortest <= 1e-9 {
        return Err("the loop has no edges leaving it".into());
    }
    let dist = shortest * amount.clamp(0.02, 0.45);
    let mut b = Builder::new(mesh);
    let mut mids: HashMap<(u32, u32), u32> = HashMap::new();
    let mut ring_a: Vec<Option<u32>> = Vec::with_capacity(n);
    let mut ring_b: Vec<Option<u32>> = Vec::with_capacity(n);
    for (i, &v) in order.iter().enumerate() {
        for (side, ring) in [(side_a[i], &mut ring_a), (side_b[i], &mut ring_b)] {
            let made = side.map(|s| {
                let t = (dist / p(v).distance(p(s)).max(1e-9)).min(0.9);
                let m = b.add(p(v).lerp(p(s), t), v);
                mids.insert(key(v, s), m);
                m
            });
            ring.push(made);
        }
    }
    if ring_a.iter().chain(ring_b.iter()).filter(|m| m.is_none()).count() > if other.is_none() { n } else { 0 } {
        return Err("the loop leaves its quads partway; edge inset needs a clean edge loop".into());
    }
    let mut faces = split_by_mids(&quads, &mids)?;
    let segments = if closed { n } else { n - 1 };
    if crease {
        for i in 0..segments {
            let j = (i + 1) % n;
            for ring in [&ring_a, &ring_b] {
                if let (Some(x), Some(y)) = (ring[i], ring[j]) {
                    b.crease(x, y);
                }
            }
        }
    }
    if !keep_centre {
        if other.is_none() {
            return Err("border only needs faces on both sides of the loop; this edge is on an open border".into());
        }
        // Each loop edge now has one thin quad on each side; the two become one quad between the side loops.
        let loop_edges: HashSet<(u32, u32)> = (0..segments).map(|i| key(order[i], order[(i + 1) % n])).collect();
        let mut merged: Vec<[u32; 4]> = Vec::new();
        for i in 0..segments {
            let j = (i + 1) % n;
            let (Some(a0), Some(a1), Some(b0), Some(b1)) = (ring_a[i], ring_a[j], ring_b[i], ring_b[j]) else { continue };
            // Keep the winding of the side-A thin quad, which runs v_i, v_j, a1, a0 or the reverse.
            let thin = faces.iter().find(|f| f.contains(&order[i]) && f.contains(&order[j]) && f.contains(&a0)).ok_or("the inset strip was not found")?;
            let forward = (0..4).any(|k| thin[k] == order[i] && thin[(k + 1) % 4] == order[j]);
            merged.push(if forward { [b0, b1, a1, a0] } else { [a0, a1, b1, b0] });
        }
        faces.retain(|f| !(0..4).any(|k| loop_edges.contains(&key(f[k], f[(k + 1) % 4]))));
        faces.extend(merged);
    }
    Ok((b.mesh(&faces), n))
}

/// Point split: a circular inset round the picked corner. Each quad meeting the corner becomes three, through a new
/// point on each of its edges at the corner and one inside it, all `amount` of the shortest edge from the corner (with
/// `equalize` on a circle; otherwise the inner points sit that share of the way to the opposite corner), so the corner
/// keeps its quads in the middle (ZModeler's keep-quad centre). With `crease` the new ring is creased. Returns the mesh
/// and how many quads met the corner.
pub fn split_point(mesh: &Mesh, hit: &PolyHit, amount: f32, equalize: bool, crease: bool) -> Result<(Mesh, usize), String> {
    let quads = quads_of(mesh)?;
    let v = hit.vertex;
    let around: Vec<usize> = (0..quads.len()).filter(|&q| quads[q].contains(&v)).collect();
    if around.is_empty() {
        return Err("the corner belongs to no face".into());
    }
    let pv = mesh.position(v);
    let shortest = around
        .iter()
        .flat_map(|&q| {
            let f = quads[q];
            let i = f.iter().position(|&x| x == v).expect("contains");
            [f[(i + 1) % 4], f[(i + 3) % 4]]
        })
        .map(|w| pv.distance(mesh.position(w)))
        .fold(f32::INFINITY, f32::min);
    if shortest <= 1e-9 {
        return Err("the corner has a zero-length edge".into());
    }
    let r = shortest * amount.clamp(0.05, 0.9);
    let mut b = Builder::new(mesh);
    let mut on_edge: HashMap<u32, u32> = HashMap::new();
    let mut edge_point = |b: &mut Builder, w: u32| -> u32 {
        *on_edge.entry(w).or_insert_with(|| {
            let d = (mesh.position(w) - pv).normalize_or_zero();
            b.add(pv + d * r, v)
        })
    };
    let mut faces: Vec<[u32; 4]> = Vec::with_capacity(quads.len() + 2 * around.len());
    for (q, f) in quads.iter().enumerate() {
        let Some(i) = f.iter().position(|&x| x == v) else {
            faces.push(*f);
            continue;
        };
        let (n1, d, n2) = (f[(i + 1) % 4], f[(i + 2) % 4], f[(i + 3) % 4]);
        let p1 = edge_point(&mut b, n1);
        let p2 = edge_point(&mut b, n2);
        let pd = mesh.position(d);
        let c_pos = if equalize { pv + (pd - pv).normalize_or_zero() * r } else { pv.lerp(pd, (r / shortest).min(0.9)) };
        let c = b.add(c_pos, v);
        let _ = q;
        faces.push([v, p1, c, p2]);
        faces.push([p1, n1, d, c]);
        faces.push([c, d, n2, p2]);
        if crease {
            b.crease(p1, c);
            b.crease(c, p2);
        }
    }
    Ok((b.mesh(&faces), around.len()))
}

/// Where a stitch puts the welded point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StitchAlign {
    /// At the second point (the first moves to it).
    End,
    Mid,
    /// At the first point.
    Start,
}

/// Point stitch: welds corner `a` to corner `b`. A quad with both as opposite corners collapses and disappears (its
/// two pairs of sides become two edges); a weld across one edge is refused, since it would leave triangles; a weld of
/// far-apart corners (two border points, say) joins the mesh there. Refused when an edge would end up with more than
/// two faces. Returns the mesh and the quads removed.
pub fn stitch_points(mesh: &Mesh, a: u32, b: u32, align: StitchAlign) -> Result<(Mesh, usize), String> {
    let quads = quads_of(mesh)?;
    if a == b {
        return Err("pick two different points".into());
    }
    if quads.iter().any(|f| (0..4).any(|k| key(f[k], f[(k + 1) % 4]) == key(a, b))) {
        return Err("the two points share an edge; welding them would leave triangles (stitch points across a face or across a gap)".into());
    }
    let mut out: Vec<[u32; 4]> = Vec::with_capacity(quads.len());
    let mut removed = 0;
    for f in &quads {
        let g = f.map(|x| if x == a { b } else { x });
        let mut u = g.to_vec();
        u.sort_unstable();
        u.dedup();
        if u.len() < 4 {
            removed += 1;
            continue;
        }
        out.push(g);
    }
    if out.is_empty() {
        return Err("the weld would remove every face".into());
    }
    if edge_counts(&out).values().any(|&c| c > 2) {
        return Err("the weld would put more than two faces on one edge".into());
    }
    let mut bld = Builder::new(mesh);
    let (pa, pb) = (mesh.position(a), mesh.position(b));
    let at = match align {
        StitchAlign::End => pb,
        StitchAlign::Mid => (pa + pb) * 0.5,
        StitchAlign::Start => pa,
    };
    bld.positions[b as usize] = at.to_array();
    for c in bld.creases.iter_mut() {
        if c.0 == a {
            c.0 = b;
        }
        if c.1 == a {
            c.1 = b;
        }
        *c = key(c.0, c.1);
    }
    Ok((bld.mesh(&out), removed))
}
