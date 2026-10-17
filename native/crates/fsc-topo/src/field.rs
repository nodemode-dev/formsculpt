//! Field-aligned quad remesh, after Instant Meshes (Jakob, Tarini, Panozzo, Sorkine-Hornung 2015), written from the
//! paper's description: a smooth 4-rotationally-symmetric direction field over the surface (held along hard edges),
//! a position field placing a square lattice along it, and extraction of the lattice's cells as polygons, which are
//! split once into quads so the result is all quads and follows the surface's flow instead of the world axes.

use std::collections::{HashMap, HashSet, VecDeque};

use fsc_core::{Bvh, Mesh, Topology};
use glam::Vec3;

use crate::nearest::nearest_point;
use crate::subdivide::loop_subdivide;

/// What the field remesh takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldOptions {
    /// Wanted quad count of the result.
    pub target_faces: usize,
    /// Edges whose faces meet at more than this many degrees are hard: the field runs along them.
    pub hard_angle: f32,
    /// Paint sets the density: red areas get smaller quads (down to half), blue ones larger (up to double).
    pub density_from_paint: bool,
    /// Smoothing passes for the direction and position fields.
    pub iterations: u32,
}

impl Default for FieldOptions {
    fn default() -> FieldOptions {
        FieldOptions { target_faces: 2000, hard_angle: 40.0, density_from_paint: false, iterations: 40 }
    }
}

/// What came out, with the counts that describe it.
pub struct FieldRemesh {
    pub mesh: Mesh,
    /// Polygons extracted from the lattice before the quad split, and how many of them were already quads.
    pub polygons: usize,
    pub quad_polygons: usize,
    pub hard_edges: usize,
    /// Small holes the lattice left (near singular points) closed with quad fans.
    pub holes_filled: usize,
    /// Vertices of the working mesh whose direction a guide curve set.
    pub guided_vertices: usize,
    /// Vertices of the working mesh whose direction a bone set.
    pub bone_vertices: usize,
}

/// The direction in `n`'s plane among the four rotations of `o` that is closest to `to`.
fn closest_rotation(o: Vec3, n: Vec3, to: Vec3) -> Vec3 {
    let a = o;
    let b = n.cross(o);
    [a, b, -a, -b].into_iter().max_by(|x, y| x.dot(to).total_cmp(&y.dot(to))).unwrap_or(a)
}

fn tangent(v: Vec3, n: Vec3) -> Option<Vec3> {
    (v - n * v.dot(n)).try_normalize()
}

/// The lattice point (origin `t`, axes `a` and `b`, spacing `s`) nearest to `p`.
fn lattice_nearest(t: Vec3, a: Vec3, b: Vec3, s: f32, p: Vec3) -> Vec3 {
    let d = p - t;
    t + a * (s * (d.dot(a) / s).round()) + b * (s * (d.dot(b) / s).round())
}

/// Every hole's border as simple cycles of vertices. A border that passes through one vertex twice (a pinch, where
/// two gaps touch at a corner) is walked edge by edge and cut at the repeated vertex into separate cycles, so each
/// piece can be closed on its own; border_loops would stop at the repeat and lose the rest of the loop.
fn border_cycles(mesh: &Mesh) -> Vec<Vec<u32>> {
    let mut directed: HashSet<(u32, u32)> = HashSet::new();
    for t in mesh.indices.chunks_exact(3) {
        for k in 0..3 {
            directed.insert((t[k], t[(k + 1) % 3]));
        }
    }
    let mut border: Vec<(u32, u32)> = directed.iter().copied().filter(|&(a, b)| !directed.contains(&(b, a))).collect();
    border.sort_unstable();
    let mut out_edges: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(a, b) in &border {
        out_edges.entry(a).or_default().push(b);
    }
    let mut cycles = Vec::new();
    for &(a0, _) in &border {
        // Walk one closed border from a0, taking any unused edge out of each vertex.
        let mut walk = vec![a0];
        let mut at = a0;
        while let Some(list) = out_edges.get_mut(&at) {
            let Some(next) = list.pop() else { break };
            at = next;
            if at == a0 {
                break;
            }
            walk.push(at);
            if walk.len() > border.len() {
                break;
            }
        }
        if walk.len() < 3 || at != a0 {
            continue;
        }
        // Cut the closed walk into simple cycles at repeated vertices.
        let mut stack: Vec<u32> = Vec::new();
        let mut on_stack: HashMap<u32, usize> = HashMap::new();
        for &v in &walk {
            if let Some(&i) = on_stack.get(&v) {
                let cycle: Vec<u32> = stack.drain(i..).collect();
                for u in &cycle {
                    on_stack.remove(u);
                }
                if cycle.len() >= 3 {
                    cycles.push(cycle);
                }
            }
            on_stack.insert(v, stack.len());
            stack.push(v);
        }
        if stack.len() >= 3 {
            cycles.push(stack);
        }
    }
    cycles
}

struct Dsu(Vec<u32>);

impl Dsu {
    fn find(&mut self, x: u32) -> u32 {
        let mut r = x;
        while self.0[r as usize] != r {
            r = self.0[r as usize];
        }
        let mut c = x;
        while self.0[c as usize] != r {
            let next = self.0[c as usize];
            self.0[c as usize] = r;
            c = next;
        }
        r
    }

    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[ra.max(rb) as usize] = ra.min(rb);
        }
    }
}

pub fn field_remesh(mesh: &Mesh, bvh: &Bvh, opts: FieldOptions) -> Result<FieldRemesh, String> {
    field_remesh_guided(mesh, bvh, opts, &[])
}

/// field_remesh with guide curves (polylines near the surface): along each one the direction field is held to the
/// curve's direction, so edge loops run along it, as ZRemesher's curves steer it; elsewhere the field blends between
/// the guides and the hard edges.
pub fn field_remesh_guided(mesh: &Mesh, bvh: &Bvh, opts: FieldOptions, guides: &[Vec<Vec3>]) -> Result<FieldRemesh, String> {
    field_remesh_steered(mesh, bvh, opts, guides, &[])
}

/// field_remesh steered by guide curves and by bones (segments inside the shape, as a skeleton's links): every
/// surface point clearly owned by one bone and away from its ends (where limbs meet) takes that bone's direction, so
/// edge loops ring each limb and run along it; around the joints the field blends freely.
pub fn field_remesh_steered(mesh: &Mesh, bvh: &Bvh, opts: FieldOptions, guides: &[Vec<Vec3>], bones: &[(Vec3, Vec3)]) -> Result<FieldRemesh, String> {
    let area: f32 = mesh.face_normals().iter().map(|n| n.length() * 0.5).sum();
    if area <= 0.0 || mesh.triangle_count() == 0 {
        return Err("the mesh has no area".into());
    }
    // Each lattice cell becomes about four quads in the final split, so the lattice is twice the final quad size.
    let quad = (area / opts.target_faces.max(8) as f32).sqrt();
    let scale = quad * 2.0;
    // Work on a mesh fine enough for the lattice: split triangles (in place, no smoothing) until edges are under a
    // third of a cell, carrying paint for the density.
    let mut work = mesh.clone();
    for _ in 0..5 {
        let mean = {
            let mut sum = 0.0;
            let mut count = 0;
            for t in 0..work.triangle_count() {
                let [a, b, c] = work.triangle(t).map(|v| work.position(v));
                sum += a.distance(b) + b.distance(c) + c.distance(a);
                count += 3;
            }
            sum / count.max(1) as f32
        };
        if mean < scale / 3.0 || work.triangle_count() > 400_000 {
            break;
        }
        let sub = loop_subdivide(&work, false);
        let mut next = sub.mesh;
        if work.has_paint() && sub.parents.len() == next.vertex_count() {
            next.paint = sub.parents.iter().map(|&(a, b)| fsc_core::mix_paint(work.paint[a as usize], work.paint[b as usize], 0.5)).collect();
        }
        // Subdivision numbers its new vertices in hash order, which differs from run to run; renumbering them by their
        // parent pair makes the field (grown in vertex order) and so the result the same every time.
        if sub.parents.len() == next.vertex_count() {
            let mut order: Vec<u32> = (0..next.vertex_count() as u32).collect();
            order.sort_by_key(|&v| sub.parents[v as usize]);
            let mut new_index = vec![0u32; order.len()];
            for (i, &v) in order.iter().enumerate() {
                new_index[v as usize] = i as u32;
            }
            let positions = order.iter().map(|&v| next.positions[v as usize]).collect();
            let indices = next.indices.iter().map(|&v| new_index[v as usize]).collect();
            let paint = if next.has_paint() { order.iter().map(|&v| next.paint[v as usize]).collect() } else { Vec::new() };
            next = Mesh::new(positions, indices);
            next.paint = paint;
        }
        work = next;
    }
    work.recompute_normals();
    let nv = work.vertex_count();
    let topo = Topology::build(&work);
    let p: Vec<Vec3> = work.positions.iter().map(|&q| Vec3::from(q)).collect();
    let n: Vec<Vec3> = work.normals.iter().map(|&q| Vec3::from(q).normalize_or(Vec3::Y)).collect();
    // Per-vertex lattice spacing: paint red (denser) to blue (sparser) when asked.
    let s: Vec<f32> = (0..nv)
        .map(|v| {
            if opts.density_from_paint && work.has_paint() {
                let c = work.paint[v];
                scale * 2f32.powf((c[2] - c[0]).clamp(-1.0, 1.0))
            } else {
                scale
            }
        })
        .collect();

    // Hard edges: faces meeting at more than hard_angle; their vertices take the edge's direction and keep it.
    let face_n = work.face_normals();
    let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for t in 0..work.triangle_count() {
        let [a, b, c] = work.triangle(t);
        for (x, y) in [(a, b), (b, c), (c, a)] {
            edge_faces.entry((x.min(y), x.max(y))).or_default().push(t);
        }
    }
    let cos_hard = opts.hard_angle.to_radians().cos();
    let mut o: Vec<Vec3> = vec![Vec3::ZERO; nv];
    let mut fixed = vec![false; nv];
    let mut hard_edges = 0;
    // In a fixed order, so where hard edges meet at a corner the result is the same on every run.
    let mut sorted: Vec<(&(u32, u32), &Vec<usize>)> = edge_faces.iter().collect();
    sorted.sort_unstable_by_key(|(e, _)| **e);
    for (&(a, b), fs) in sorted {
        let hard = fs.len() == 1 || (fs.len() == 2 && face_n[fs[0]].normalize_or_zero().dot(face_n[fs[1]].normalize_or_zero()) < cos_hard);
        if !hard {
            continue;
        }
        hard_edges += 1;
        let dir = p[b as usize] - p[a as usize];
        for v in [a, b] {
            if let Some(d) = tangent(dir, n[v as usize]) {
                o[v as usize] = if fixed[v as usize] { closest_rotation(d, n[v as usize], o[v as usize]) } else { d };
                fixed[v as usize] = true;
            }
        }
    }

    // Guide curves: sampled at a quarter cell, and every vertex within about half a cell of a sample (and not already
    // held by a hard edge) takes the curve's direction there and keeps it.
    let mut guided_vertices = 0;
    if guides.iter().any(|g| g.len() >= 2) {
        let cell = scale * 0.6;
        let key = |q: Vec3| ((q.x / cell).floor() as i32, (q.y / cell).floor() as i32, (q.z / cell).floor() as i32);
        let mut grid: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
        for (v, &q) in p.iter().enumerate() {
            grid.entry(key(q)).or_default().push(v as u32);
        }
        let mut by_guide = vec![false; nv];
        for g in guides {
            for seg in g.windows(2) {
                let dir = seg[1] - seg[0];
                let len = dir.length();
                if len <= 1e-9 {
                    continue;
                }
                let steps = (len / (scale * 0.25)).ceil().max(1.0) as usize;
                for k in 0..=steps {
                    let q = seg[0] + dir * (k as f32 / steps as f32);
                    let (cx, cy, cz) = key(q);
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            for dz in -1..=1 {
                                let Some(list) = grid.get(&(cx + dx, cy + dy, cz + dz)) else { continue };
                                for &v in list {
                                    let vi = v as usize;
                                    if p[vi].distance(q) > cell || (fixed[vi] && !by_guide[vi]) {
                                        continue;
                                    }
                                    let Some(d) = tangent(dir, n[vi]) else { continue };
                                    o[vi] = if by_guide[vi] { closest_rotation(d, n[vi], o[vi]) } else { d };
                                    if !by_guide[vi] {
                                        guided_vertices += 1;
                                    }
                                    by_guide[vi] = true;
                                    fixed[vi] = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Bones: a point takes its nearest bone's direction when that bone is clearly nearest (the next is at least half
    // as far again) and the closest place on it is in its middle 70 percent; hard edges and guides win.
    let mut bone_vertices = 0;
    let usable: Vec<(Vec3, Vec3)> = bones.iter().copied().filter(|(a, b)| a.distance(*b) > 1e-6).collect();
    if !usable.is_empty() {
        for v in 0..nv {
            if fixed[v] {
                continue;
            }
            let mut best = (f32::INFINITY, 0usize, 0.0f32);
            let mut second = f32::INFINITY;
            for (i, &(a, b)) in usable.iter().enumerate() {
                let ab = b - a;
                let t = ((p[v] - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
                let d = p[v].distance(a + ab * t);
                if d < best.0 {
                    second = best.0;
                    best = (d, i, t);
                } else if d < second {
                    second = d;
                }
            }
            let (d, i, t) = best;
            if !(0.15..=0.85).contains(&t) || second < d * 1.5 {
                continue;
            }
            let (a, b) = usable[i];
            if let Some(dir) = tangent(b - a, n[v]) {
                o[v] = dir;
                fixed[v] = true;
                bone_vertices += 1;
            }
        }
    }

    // Direction field: grown outward from the hard edges (or one seed) so neighbours start consistent, then smoothed.
    let mut seen = fixed.clone();
    let mut queue: VecDeque<u32> = (0..nv as u32).filter(|&v| fixed[v as usize]).collect();
    if queue.is_empty() {
        let seed = 0u32;
        o[0] = tangent(Vec3::X, n[0]).or_else(|| tangent(Vec3::Z, n[0])).unwrap_or(Vec3::X);
        seen[0] = true;
        queue.push_back(seed);
    }
    let mut components = 0;
    loop {
        while let Some(v) = queue.pop_front() {
            for &w in topo.neighbors_of(v) {
                if seen[w as usize] {
                    continue;
                }
                let wn = n[w as usize];
                o[w as usize] = tangent(o[v as usize], wn).or_else(|| tangent(Vec3::X, wn)).unwrap_or(Vec3::X);
                seen[w as usize] = true;
                queue.push_back(w);
            }
        }
        components += 1;
        // A part not reached (a separate shell) gets its own seed.
        match (0..nv).find(|&v| !seen[v]) {
            Some(v) if components < 10_000 => {
                o[v] = tangent(Vec3::X, n[v]).or_else(|| tangent(Vec3::Z, n[v])).unwrap_or(Vec3::X);
                seen[v] = true;
                queue.push_back(v as u32);
            }
            _ => break,
        }
    }
    for _ in 0..opts.iterations {
        for v in 0..nv {
            if fixed[v] {
                continue;
            }
            let nb = topo.neighbors_of(v as u32);
            if nb.is_empty() {
                continue;
            }
            let mut acc = o[v];
            for &w in nb {
                let Some(ow) = tangent(o[w as usize], n[v]) else { continue };
                acc += closest_rotation(ow, n[v], acc);
            }
            if let Some(d) = tangent(acc, n[v]) {
                o[v] = d;
            }
        }
    }

    // Position field: a lattice origin per vertex, the lattice point nearest the vertex, smoothed toward the lattice
    // points its neighbours' lattices put there.
    let axes = |v: usize| (o[v], n[v].cross(o[v]));
    let mut t: Vec<Vec3> = p.clone();
    let mut seen = vec![false; nv];
    for start in 0..nv {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut queue = VecDeque::from([start as u32]);
        while let Some(v) = queue.pop_front() {
            for &w in topo.neighbors_of(v) {
                if seen[w as usize] {
                    continue;
                }
                let (wi, vi) = (w as usize, v as usize);
                let a = closest_rotation(o[vi], n[vi], o[wi]);
                let b = n[vi].cross(a);
                let c = lattice_nearest(t[vi], a, b, s[vi], p[wi]);
                let (wa, wb) = axes(wi);
                t[wi] = lattice_nearest(c - n[wi] * (c - p[wi]).dot(n[wi]), wa, wb, s[wi], p[wi]);
                seen[wi] = true;
                queue.push_back(w);
            }
        }
    }
    for _ in 0..opts.iterations {
        for v in 0..nv {
            let nb = topo.neighbors_of(v as u32);
            if nb.is_empty() {
                continue;
            }
            let (va, vb) = axes(v);
            let mut acc = t[v];
            let mut weight = 1.0;
            for &w in nb {
                let wi = w as usize;
                let a = closest_rotation(o[wi], n[wi], va);
                let b = n[wi].cross(a);
                // The neighbour's lattice point nearest the running estimate (not the vertex), so neighbours that
                // agree on the lattice never average to a point half a step away.
                let c = lattice_nearest(t[wi], a, b, s[wi], acc / weight);
                acc += c - n[v] * (c - p[v]).dot(n[v]);
                weight += 1.0;
            }
            t[v] = lattice_nearest(acc / weight, va, vb, s[v], p[v]);
        }
    }

    // Extraction: vertices whose lattice points coincide form one cluster; neighbours one lattice step apart along an
    // axis link their clusters.
    let mut dsu = Dsu((0..nv as u32).collect());
    let mut links: Vec<(u32, u32)> = Vec::new();
    for v in 0..nv {
        let (a, b) = axes(v);
        for &w in topo.neighbors_of(v as u32) {
            let wi = w as usize;
            if wi <= v {
                continue;
            }
            let sv = 0.5 * (s[v] + s[wi]);
            let d = t[wi] - t[v];
            let (fu, fv) = (d.dot(a) / sv, d.dot(b) / sv);
            // Offsets far from whole steps mean the two lattices disagree there; such pairs neither merge nor link,
            // so a seam in the field cannot chain clusters together.
            if (fu - fu.round()).abs() > 0.3 || (fv - fv.round()).abs() > 0.3 {
                continue;
            }
            let (du, dv) = (fu.round() as i32, fv.round() as i32);
            match du.abs() + dv.abs() {
                0 => dsu.union(v as u32, w),
                1 => links.push((v as u32, w)),
                _ => {}
            }
        }
    }
    let mut cluster_of = vec![u32::MAX; nv];
    let mut members: Vec<Vec<u32>> = Vec::new();
    let mut root_index: HashMap<u32, u32> = HashMap::new();
    for v in 0..nv as u32 {
        let r = dsu.find(v);
        let id = *root_index.entry(r).or_insert_with(|| {
            members.push(Vec::new());
            members.len() as u32 - 1
        });
        cluster_of[v as usize] = id;
        members[id as usize].push(v);
    }
    let nc = members.len();
    let cpos: Vec<Vec3> = members.iter().map(|m| m.iter().map(|&v| t[v as usize]).sum::<Vec3>() / m.len() as f32).collect();
    let cnrm: Vec<Vec3> = members.iter().map(|m| m.iter().map(|&v| n[v as usize]).sum::<Vec3>().normalize_or(Vec3::Y)).collect();
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); nc];
    let mut linked: HashSet<(u32, u32)> = HashSet::new();
    for (v, w) in links {
        let (a, b) = (cluster_of[v as usize], cluster_of[w as usize]);
        if a == b || !linked.insert((a.min(b), a.max(b))) {
            continue;
        }
        adj[a as usize].push(b);
        adj[b as usize].push(a);
    }
    // Each cluster's links in counter-clockwise order around its normal, for walking faces.
    for c in 0..nc {
        let nrm = cnrm[c];
        let ref_dir = adj[c].first().and_then(|&f| tangent(cpos[f as usize] - cpos[c], nrm)).unwrap_or(Vec3::X);
        let side = nrm.cross(ref_dir);
        let origin = cpos[c];
        adj[c].sort_by(|&x, &y| {
            let ang = |k: u32| {
                let d = cpos[k as usize] - origin;
                d.dot(side).atan2(d.dot(ref_dir))
            };
            ang(x).total_cmp(&ang(y))
        });
    }
    // Face walk: from the directed link u -> v, the next link leaves v just clockwise of v -> u, which traces each face
    // counter-clockwise; faces longer than six links are holes in the lattice and are dropped.
    let mut used: HashSet<(u32, u32)> = HashSet::new();
    let mut faces: Vec<Vec<u32>> = Vec::new();
    for u in 0..nc as u32 {
        for &v in &adj[u as usize] {
            if used.contains(&(u, v)) {
                continue;
            }
            let mut face = vec![u];
            let (mut a, mut b) = (u, v);
            let mut ok = false;
            for _ in 0..12 {
                used.insert((a, b));
                face.push(b);
                let list = &adj[b as usize];
                let Some(i) = list.iter().position(|&x| x == a) else { break };
                let next = list[(i + list.len() - 1) % list.len()];
                a = b;
                b = next;
                if a == u && b == v {
                    ok = true;
                    break;
                }
                if b == u {
                    used.insert((a, b));
                    ok = (a, b) != (u, v);
                    break;
                }
            }
            if ok {
                face.dedup();
                if face.last() == face.first() {
                    face.pop();
                }
                let mut uniq = face.clone();
                uniq.sort_unstable();
                uniq.dedup();
                if (3..=12).contains(&face.len()) && uniq.len() == face.len() {
                    faces.push(face);
                }
            }
        }
    }
    if faces.is_empty() {
        return Err("the field produced no faces; raise the target or lower the hard angle".into());
    }
    // Keep each face facing out like the surface under it.
    for f in faces.iter_mut() {
        let c = f.iter().map(|&k| cpos[k as usize]).sum::<Vec3>() / f.len() as f32;
        let mut fnrm = Vec3::ZERO;
        for i in 0..f.len() {
            fnrm += (cpos[f[i] as usize] - c).cross(cpos[f[(i + 1) % f.len()] as usize] - c);
        }
        let surf = f.iter().map(|&k| cnrm[k as usize]).sum::<Vec3>();
        if fnrm.dot(surf) < 0.0 {
            f.reverse();
        }
    }
    // At a sharp rim the averaged normals can disagree with a face's true facing, so some faces come out wound against
    // their neighbours. Winding is made consistent across shared edges, spreading from the faces whose facing agrees
    // most clearly with the surface (an edge shared by more than two faces is skipped).
    {
        let mut confidence: Vec<(f32, usize)> = faces
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let c = f.iter().map(|&k| cpos[k as usize]).sum::<Vec3>() / f.len() as f32;
                let mut fnrm = Vec3::ZERO;
                for j in 0..f.len() {
                    fnrm += (cpos[f[j] as usize] - c).cross(cpos[f[(j + 1) % f.len()] as usize] - c);
                }
                let surf = f.iter().map(|&k| cnrm[k as usize]).sum::<Vec3>();
                (fnrm.normalize_or_zero().dot(surf.normalize_or_zero()), i)
            })
            .collect();
        confidence.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (i, f) in faces.iter().enumerate() {
            for j in 0..f.len() {
                let (a, b) = (f[j], f[(j + 1) % f.len()]);
                by_edge.entry((a.min(b), a.max(b))).or_default().push(i);
            }
        }
        let has_directed = |f: &[u32], a: u32, b: u32| (0..f.len()).any(|j| f[j] == a && f[(j + 1) % f.len()] == b);
        let mut done = vec![false; faces.len()];
        for &(_, seed) in &confidence {
            if done[seed] {
                continue;
            }
            done[seed] = true;
            let mut queue = VecDeque::from([seed]);
            while let Some(i) = queue.pop_front() {
                let f = faces[i].clone();
                for j in 0..f.len() {
                    let (a, b) = (f[j], f[(j + 1) % f.len()]);
                    let Some(list) = by_edge.get(&(a.min(b), a.max(b))) else { continue };
                    if list.len() != 2 {
                        continue;
                    }
                    let other = if list[0] == i { list[1] } else { list[0] };
                    if done[other] {
                        continue;
                    }
                    if has_directed(&faces[other], a, b) {
                        faces[other].reverse();
                    }
                    done[other] = true;
                    queue.push_back(other);
                }
            }
        }
        // Where winding still conflicts (an odd cycle of flips round a singular point) or more than two faces share an
        // edge, the less certain faces are dropped; the holes they leave have even sides and close with the fans below.
        let mut rank = vec![0usize; faces.len()];
        for (r, &(_, i)) in confidence.iter().enumerate() {
            rank[i] = r;
        }
        let mut drop = vec![false; faces.len()];
        let mut directed: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (i, f) in faces.iter().enumerate() {
            for j in 0..f.len() {
                directed.entry((f[j], f[(j + 1) % f.len()])).or_default().push(i);
            }
        }
        // (faces on one edge, how many of them may stay): one per direction, two per edge.
        let mut groups: Vec<(Vec<usize>, usize)> = directed.into_values().filter(|l| l.len() > 1).map(|l| (l, 1)).collect();
        groups.extend(by_edge.into_values().filter(|l| l.len() > 2).map(|l| (l, 2)));
        groups.sort();
        for (mut list, keep) in groups {
            list.sort_by_key(|&i| rank[i]);
            list.dedup();
            let extra: Vec<usize> = list.iter().copied().filter(|&i| !drop[i]).skip(keep).collect();
            for i in extra {
                drop[i] = true;
            }
        }
        let mut k = 0;
        faces.retain(|_| {
            k += 1;
            !drop[k - 1]
        });
    }
    let quad_polygons = faces.iter().filter(|f| f.len() == 4).count();

    // Split every polygon into quads (centre, edge midpoints, corners), so the result is all quads and stays
    // conforming across shared edges.
    let mut positions: Vec<Vec3> = cpos.clone();
    let mut mids: HashMap<(u32, u32), u32> = HashMap::new();
    let mut quads: Vec<[u32; 4]> = Vec::new();
    for f in &faces {
        let c = positions.len() as u32;
        positions.push(f.iter().map(|&k| cpos[k as usize]).sum::<Vec3>() / f.len() as f32);
        let mut mid = |a: u32, b: u32, positions: &mut Vec<Vec3>| -> u32 {
            *mids.entry((a.min(b), a.max(b))).or_insert_with(|| {
                positions.push((positions[a as usize] + positions[b as usize]) * 0.5);
                positions.len() as u32 - 1
            })
        };
        let k = f.len();
        for i in 0..k {
            let (prev, cur, next) = (f[(i + k - 1) % k], f[i], f[(i + 1) % k]);
            let m0 = mid(prev, cur, &mut positions);
            let m1 = mid(cur, next, &mut positions);
            quads.push([cur, m1, c, m0]);
        }
    }
    // Drop clusters no face uses, then lay every point on the original surface with a few tangential relaxes.
    let mut remap = vec![u32::MAX; positions.len()];
    let mut out_pos: Vec<[f32; 3]> = Vec::new();
    for q in &quads {
        for &v in q {
            if remap[v as usize] == u32::MAX {
                remap[v as usize] = out_pos.len() as u32;
                out_pos.push(positions[v as usize].to_array());
            }
        }
    }
    let indices: Vec<u32> = quads.iter().flat_map(|q| {
        let r = q.map(|v| remap[v as usize]);
        [r[0], r[1], r[2], r[0], r[2], r[3]]
    }).collect();
    let mut out = Mesh::new(out_pos, indices);
    // Holes are left where the lattice could not be read (near the field's singular points). On a closed source every
    // hole is one of those; every border edge here is half of a split polygon edge, so each hole has an even number of
    // sides and closes with a fan of quads around its centre, as close_border does. An open source keeps all its
    // holes, since its real openings cannot be told from them.
    let mut holes_filled = 0;
    let mut extra: Vec<[u32; 4]> = Vec::new();
    let mut fill_pos: Vec<[f32; 3]> = out.positions.clone();
    let closed_source = crate::edges::boundary_edges(mesh).is_empty();
    for lp in border_cycles(&out) {
        let k = lp.len();
        if !closed_source || k < 4 || k % 2 == 1 {
            continue;
        }
        let c = lp.iter().map(|&v| out.position(v)).sum::<Vec3>() / k as f32;
        if k == 4 {
            extra.push([lp[3], lp[2], lp[1], lp[0]]);
        } else {
            let ci = fill_pos.len() as u32;
            fill_pos.push(c.to_array());
            for i in (0..k).step_by(2) {
                extra.push([lp[(i + 2) % k], lp[(i + 1) % k], lp[i], ci]);
            }
        }
        holes_filled += 1;
    }
    if holes_filled > 0 {
        let mut idx = std::mem::take(&mut out.indices);
        idx.extend(extra.iter().flat_map(|q| [q[0], q[1], q[2], q[0], q[2], q[3]]));
        out = Mesh::new(fill_pos, idx);
    }
    let reach = mesh.bounds().extent().length();
    for pass in 0..4 {
        let otopo = Topology::build(&out);
        let next: Vec<[f32; 3]> = (0..out.vertex_count() as u32)
            .map(|v| {
                let q = out.position(v);
                let nb = otopo.neighbors_of(v);
                let moved = if pass == 0 || nb.is_empty() {
                    q
                } else {
                    let avg = nb.iter().map(|&w| out.position(w)).sum::<Vec3>() / nb.len() as f32;
                    let nn = Vec3::from(out.normals[v as usize]);
                    let d = avg - q;
                    q + (d - nn * d.dot(nn)) * 0.5
                };
                nearest_point(bvh, mesh, moved, reach).map(|(r, _, _)| r).unwrap_or(moved).to_array()
            })
            .collect();
        out.positions = next;
        out.recompute_normals();
    }
    out.quads = true;
    Ok(FieldRemesh { mesh: out, polygons: faces.len(), quad_polygons, hard_edges, holes_filled, guided_vertices, bone_vertices })
}
