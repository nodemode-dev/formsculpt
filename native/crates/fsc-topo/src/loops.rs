//! Polygon edge tools on quad meshes and open borders: inserting an edge loop around a ring of quads, and bridging two
//! open borders with a band of quads.

use std::collections::HashMap;

use fsc_core::Mesh;
use glam::Vec3;

fn quads_to_mesh(src: &Mesh, positions: Vec<[f32; 3]>, paint: Vec<fsc_core::Paint>, quads: &[[u32; 4]]) -> Mesh {
    let indices: Vec<u32> = quads.iter().flat_map(|q| [q[0], q[1], q[2], q[0], q[2], q[3]]).collect();
    let mut out = Mesh::new(positions, indices);
    out.quads = true;
    if src.has_paint() {
        out.paint = paint;
    }
    out
}

fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-20)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Inserts an edge loop: starting from the quad edge nearest `near`, the ring of quads it crosses (each entered by one
/// edge and left by the opposite one) is split down the middle, until the ring closes or reaches an open border.
/// Returns the new mesh and how many quads were split. The mesh must be all quads (quad_remesh makes one).
pub fn insert_edge_loop(mesh: &Mesh, near: Vec3) -> Result<(Mesh, usize), String> {
    let quads = mesh.quad_faces().ok_or("edge loops need a quad mesh; run quad_remesh first")?;
    let key = |a: u32, b: u32| (a.min(b), a.max(b));
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
    // Each quad in the ring with the index of the edge the loop crosses first (its opposite edge is crossed too).
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
    let mut positions = mesh.positions.clone();
    let mut paint = mesh.paint.clone();
    let mut mids: HashMap<(u32, u32), u32> = HashMap::new();
    let mut mid = |a: u32, b: u32, positions: &mut Vec<[f32; 3]>, paint: &mut Vec<fsc_core::Paint>| -> u32 {
        *mids.entry(key(a, b)).or_insert_with(|| {
            positions.push(((mesh.position(a) + mesh.position(b)) * 0.5).to_array());
            if mesh.has_paint() {
                paint.push(fsc_core::mix_paint(mesh.paint[a as usize], mesh.paint[b as usize], 0.5));
            }
            positions.len() as u32 - 1
        })
    };
    let mut out: Vec<[u32; 4]> = Vec::with_capacity(quads.len() + ring.len());
    let split: HashMap<usize, usize> = ring.iter().copied().collect();
    for (q, f) in quads.iter().enumerate() {
        match split.get(&q) {
            Some(&k) => {
                let v = |i: usize| f[(k + i) % 4];
                let m1 = mid(v(0), v(1), &mut positions, &mut paint);
                let m2 = mid(v(2), v(3), &mut positions, &mut paint);
                out.push([v(0), m1, m2, v(3)]);
                out.push([m1, v(1), v(2), m2]);
            }
            None => out.push(*f),
        }
    }
    Ok((quads_to_mesh(mesh, positions, paint, &out), ring.len()))
}

/// Deletes (dissolves) the edge loop through the quad edge nearest `near`, the inverse of `insert_edge_loop`: from that
/// edge the loop runs on through each vertex by the edge that shares no quad with the one it came in by, until it closes
/// or ends on an open border. Every loop vertex must have one edge off the loop to each side (four edges inside the
/// mesh, three on a border); each pair of quads on either side of a loop edge becomes one quad, and the loop's vertices
/// go. Returns the new mesh and how many loop edges went.
pub fn delete_edge_loop(mesh: &Mesh, near: Vec3) -> Result<(Mesh, usize), String> {
    let quads = mesh.quad_faces().ok_or("deleting an edge loop needs a quad mesh; run quad_remesh first")?;
    let key = |a: u32, b: u32| (a.min(b), a.max(b));
    let mut edge_quads: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    let mut vertex_edges: HashMap<u32, Vec<u32>> = HashMap::new();
    for (q, f) in quads.iter().enumerate() {
        for k in 0..4 {
            let (a, b) = (f[k], f[(k + 1) % 4]);
            let list = edge_quads.entry(key(a, b)).or_default();
            if list.is_empty() {
                vertex_edges.entry(a).or_default().push(b);
                vertex_edges.entry(b).or_default().push(a);
            }
            list.push(q);
        }
    }
    let (sa, sb) = edge_quads
        .keys()
        .copied()
        .min_by(|x, y| segment_distance(near, mesh.position(x.0), mesh.position(x.1)).total_cmp(&segment_distance(near, mesh.position(y.0), mesh.position(y.1))))
        .ok_or("the mesh has no faces")?;
    // The next loop edge out of `at`, having come in from `from`: the one edge at `at` sharing no quad with (from, at).
    let onward = |from: u32, at: u32| -> Option<u32> {
        let came = &edge_quads[&key(from, at)];
        let edges = vertex_edges.get(&at)?;
        if edges.len() != 4 {
            return None;
        }
        let mut found = edges.iter().copied().filter(|&n| n != from && !edge_quads[&key(at, n)].iter().any(|q| came.contains(q)));
        let n = found.next()?;
        found.next().is_none().then_some(n)
    };
    // Walk both ways from the start edge.
    let mut forward = vec![sa, sb];
    let mut closed = false;
    while let Some(n) = onward(forward[forward.len() - 2], forward[forward.len() - 1]) {
        if n == forward[0] {
            closed = true;
            break;
        }
        if forward.contains(&n) {
            return Err("the loop runs into itself; pick another edge".into());
        }
        forward.push(n);
    }
    let mut chain = forward;
    if !closed {
        let mut back = vec![sb, sa];
        while let Some(n) = onward(back[back.len() - 2], back[back.len() - 1]) {
            if chain.contains(&n) {
                return Err("the loop runs into itself; pick another edge".into());
            }
            back.push(n);
        }
        let mut head: Vec<u32> = back.into_iter().skip(2).collect();
        head.reverse();
        head.extend(chain);
        chain = head;
    }
    let edges: Vec<(u32, u32)> = if closed { (0..chain.len()).map(|i| (chain[i], chain[(i + 1) % chain.len()])).collect() } else { chain.windows(2).map(|w| (w[0], w[1])).collect() };
    let on_loop: std::collections::HashSet<u32> = chain.iter().copied().collect();
    let mut gone = vec![false; quads.len()];
    let mut merged: Vec<[u32; 4]> = Vec::new();
    for &(a, b) in &edges {
        let pair = &edge_quads[&key(a, b)];
        if pair.len() != 2 {
            return Err("the loop runs along an open border; only loops with quads on both sides can be deleted".into());
        }
        // Each quad turned so it starts with the loop edge in its own direction: [p0, p1, p2, p3] and [p1, p0, q2, q3].
        let turned = |q: usize| -> Option<[u32; 4]> {
            let f = quads[q];
            (0..4).find(|&k| on_loop.contains(&f[k]) && on_loop.contains(&f[(k + 1) % 4]) && key(f[k], f[(k + 1) % 4]) == key(a, b)).map(|k| [f[k], f[(k + 1) % 4], f[(k + 2) % 4], f[(k + 3) % 4]])
        };
        let (Some(p), Some(q)) = (turned(pair[0]), turned(pair[1])) else { return Err("the loop's quads could not be read".into()) };
        if on_loop.contains(&p[2]) || on_loop.contains(&p[3]) || on_loop.contains(&q[2]) || on_loop.contains(&q[3]) {
            return Err("the loop is too tight (a quad has three corners on it); pick another edge".into());
        }
        gone[pair[0]] = true;
        gone[pair[1]] = true;
        merged.push([p[2], p[3], q[2], q[3]]);
    }
    // Keep every vertex but the loop's, renumbered.
    let mut remap = vec![u32::MAX; mesh.vertex_count()];
    let (mut positions, mut paint) = (Vec::new(), Vec::new());
    for v in 0..mesh.vertex_count() as u32 {
        if !on_loop.contains(&v) {
            remap[v as usize] = positions.len() as u32;
            positions.push(mesh.positions[v as usize]);
            if mesh.has_paint() {
                paint.push(mesh.paint[v as usize]);
            }
        }
    }
    let out: Vec<[u32; 4]> = quads.iter().enumerate().filter(|(q, _)| !gone[*q]).map(|(_, f)| *f).chain(merged).map(|f| f.map(|v| remap[v as usize])).collect();
    if out.iter().flatten().any(|&v| v == u32::MAX) {
        return Err("a quad off the loop used a loop vertex; pick another edge".into());
    }
    Ok((quads_to_mesh(mesh, positions, paint, &out), edges.len()))
}

/// The open borders of a mesh as closed vertex loops, each in the direction its faces run along it.
pub fn border_loops(mesh: &Mesh) -> Vec<Vec<u32>> {
    let mut directed: HashMap<(u32, u32), ()> = HashMap::new();
    for t in mesh.indices.chunks_exact(3) {
        for k in 0..3 {
            directed.insert((t[k], t[(k + 1) % 3]), ());
        }
    }
    let mut next: HashMap<u32, u32> = HashMap::new();
    // Sorted, so a vertex with two border edges leaving it (a pinched border) keeps the same one every run.
    let mut keys: Vec<(u32, u32)> = directed.keys().copied().collect();
    keys.sort_unstable();
    for &(a, b) in &keys {
        if !directed.contains_key(&(b, a)) {
            next.insert(a, b);
        }
    }
    let mut loops = Vec::new();
    let mut used: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut starts: Vec<u32> = next.keys().copied().collect();
    starts.sort_unstable();
    for s in starts {
        if used.contains(&s) {
            continue;
        }
        let mut lp = vec![s];
        used.insert(s);
        let mut at = next[&s];
        while at != s {
            if !used.insert(at) {
                break;
            }
            lp.push(at);
            match next.get(&at) {
                Some(&n) => at = n,
                None => break,
            }
        }
        if lp.len() >= 3 {
            loops.push(lp);
        }
    }
    loops
}

/// Bridges two open borders with a band of quads (`segments` rows), joining them into one surface: the borders must
/// have the same number of vertices; the start is matched to the nearest pairing. With exactly two borders on the mesh
/// they are used; otherwise the two nearest `near_a` and `near_b`.
pub fn bridge_borders(mesh: &Mesh, near: Option<(Vec3, Vec3)>, segments: usize) -> Result<Mesh, String> {
    let loops = border_loops(mesh);
    if loops.len() < 2 {
        return Err(format!("bridging needs two open borders; this mesh has {} (open one with delete_faces, or merge two open objects)", loops.len()));
    }
    let centre = |l: &Vec<u32>| l.iter().map(|&v| mesh.position(v)).sum::<Vec3>() / l.len() as f32;
    let (ia, ib) = match near {
        Some((pa, pb)) => {
            let pick = |p: Vec3, skip: Option<usize>| (0..loops.len()).filter(|i| Some(*i) != skip).min_by(|&x, &y| centre(&loops[x]).distance(p).total_cmp(&centre(&loops[y]).distance(p))).unwrap_or(0);
            let a = pick(pa, None);
            (a, pick(pb, Some(a)))
        }
        None if loops.len() == 2 => (0, 1),
        None => return Err(format!("the mesh has {} open borders; say which two with near_a and near_b", loops.len())),
    };
    let (a, b) = (&loops[ia], &loops[ib]);
    if a.len() != b.len() {
        return Err(format!("the borders have {} and {} vertices; bridging needs the same count", a.len(), b.len()));
    }
    let n = a.len();
    // B runs opposite to A across the bridge; choose the offset with the shortest total span.
    let offset = (0..n)
        .min_by(|&x, &y| {
            let cost = |o: usize| (0..n).map(|i| mesh.position(a[i]).distance(mesh.position(b[(o + n - i) % n]))).sum::<f32>();
            cost(x).total_cmp(&cost(y))
        })
        .unwrap_or(0);
    let bj = |i: usize| b[(offset + n - (i % n)) % n];
    let segments = segments.max(1);
    let mut positions = mesh.positions.clone();
    let mut paint = mesh.paint.clone();
    // Rows of vertices from A (row 0) to B (row segments).
    let mut rows: Vec<Vec<u32>> = vec![a.clone()];
    for r in 1..segments {
        let t = r as f32 / segments as f32;
        let row: Vec<u32> = (0..n)
            .map(|i| {
                positions.push(mesh.position(a[i]).lerp(mesh.position(bj(i)), t).to_array());
                if mesh.has_paint() {
                    paint.push(fsc_core::mix_paint(mesh.paint[a[i] as usize], mesh.paint[bj(i) as usize], t));
                }
                positions.len() as u32 - 1
            })
            .collect();
        rows.push(row);
    }
    rows.push((0..n).map(bj).collect());
    let mut indices = mesh.indices.clone();
    for r in 0..segments {
        for i in 0..n {
            let j = (i + 1) % n;
            // A's edge runs a[i] -> a[j] in its faces, so the band uses it reversed.
            let quad = [rows[r][j], rows[r][i], rows[r + 1][i], rows[r + 1][j]];
            indices.extend_from_slice(&[quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]);
        }
    }
    let mut out = Mesh::new(positions, indices);
    if mesh.has_paint() {
        out.paint = paint;
    }
    out.quads = mesh.quads;
    Ok(out)
}
