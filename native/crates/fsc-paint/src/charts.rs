use std::collections::{HashMap, VecDeque};

use fsc_core::Mesh;
use glam::Vec3;

pub struct Segmented {
    pub keys: Vec<u32>,
    pub count: usize,
}

fn edge_key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// Splits the mesh into charts by flood fill. A chart grows from its lowest unassigned face over shared edges; a
/// neighbour joins while its normal is within `angle_deg` of the chart's area-weighted average normal and the chart
/// stays a disc. A face keeps a disc a disc when it meets the chart along one edge with its third vertex outside, or
/// along two edges (a notch); a face touching only at a vertex, or closing a loop, is refused. Charts under 1 percent
/// of the area or under 8 faces are then merged into the neighbour they share the longest border with whose result is
/// still a disc (Euler characteristic 1); a tiny chart with no such neighbour stays as it is.
pub fn segment(mesh: &Mesh, angle_deg: f32) -> Segmented {
    let tris = mesh.triangle_count();
    let cos_limit = angle_deg.to_radians().cos();
    let mut normals = vec![Vec3::Y; tris];
    let mut areas = vec![0.0f32; tris];
    for t in 0..tris {
        let [a, b, c] = mesh.triangle(t).map(|v| mesh.position(v));
        let n = (b - a).cross(c - a);
        areas[t] = n.length() * 0.5;
        normals[t] = n.normalize_or(Vec3::Y);
    }
    let total: f32 = areas.iter().sum();
    let mut edge_faces: HashMap<(u32, u32), Vec<u32>> = HashMap::with_capacity(tris * 2);
    for t in 0..tris {
        let tri = mesh.triangle(t);
        for k in 0..3 {
            edge_faces.entry(edge_key(tri[k], tri[(k + 1) % 3])).or_default().push(t as u32);
        }
    }
    let neighbors = |t: usize| -> Vec<usize> {
        let tri = mesh.triangle(t);
        let mut out = Vec::with_capacity(3);
        for k in 0..3 {
            if let Some(fs) = edge_faces.get(&edge_key(tri[k], tri[(k + 1) % 3])) {
                out.extend(fs.iter().map(|&f| f as usize).filter(|&f| f != t));
            }
        }
        out
    };

    let mut chart = vec![u32::MAX; tris];
    let mut in_chart = vec![u32::MAX; mesh.vertex_count()];
    let mut count = 0u32;
    for seed in 0..tris {
        if chart[seed] != u32::MAX {
            continue;
        }
        let id = count;
        count += 1;
        chart[seed] = id;
        let mut sum = normals[seed] * areas[seed];
        let mut chart_area = areas[seed];
        for v in mesh.triangle(seed) {
            in_chart[v as usize] = id;
        }
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.extend(neighbors(seed));
        while let Some(t) = queue.pop_front() {
            if chart[t] != u32::MAX || normals[t].dot(sum.normalize_or(normals[seed])) < cos_limit {
                continue;
            }
            let tri = mesh.triangle(t);
            let inside = tri.iter().filter(|&&v| in_chart[v as usize] == id).count();
            let shared = (0..3)
                .filter(|&k| {
                    edge_faces
                        .get(&edge_key(tri[k], tri[(k + 1) % 3]))
                        .is_some_and(|fs| fs.iter().any(|&f| f as usize != t && chart[f as usize] == id))
                })
                .count();
            let keeps_disc = (shared == 1 && inside == 2) || (shared == 2 && inside == 3);
            if !keeps_disc {
                continue;
            }
            let grown = sum + normals[t] * areas[t];
            if grown.length() < cos_limit * (chart_area + areas[t]) {
                continue;
            }
            chart[t] = id;
            sum = grown;
            chart_area += areas[t];
            for v in tri {
                in_chart[v as usize] = id;
            }
            queue.extend(neighbors(t).into_iter().filter(|&f| chart[f] == u32::MAX));
        }
    }

    let mut count = count as usize;
    loop {
        let mut area = vec![0.0f32; count];
        let mut faces = vec![0usize; count];
        for t in 0..tris {
            area[chart[t] as usize] += areas[t];
            faces[chart[t] as usize] += 1;
        }
        let mut tiny: Vec<usize> = (0..count).filter(|&c| area[c] < total * 0.01 || faces[c] < 8).collect();
        tiny.sort_by(|&a, &b| area[a].total_cmp(&area[b]).then(a.cmp(&b)));
        let mut merged = false;
        for small in tiny {
            let mut border: HashMap<u32, f32> = HashMap::new();
            for (&(a, b), fs) in &edge_faces {
                if fs.len() != 2 {
                    continue;
                }
                let (ca, cb) = (chart[fs[0] as usize], chart[fs[1] as usize]);
                if ca != cb && (ca as usize == small || cb as usize == small) {
                    let other = if ca as usize == small { cb } else { ca };
                    *border.entry(other).or_insert(0.0) += (mesh.position(a) - mesh.position(b)).length();
                }
            }
            let mut options: Vec<(u32, f32)> = border.into_iter().collect();
            options.sort_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
            for (into, _) in options {
                let moved: Vec<usize> = (0..tris).filter(|&t| chart[t] as usize == small).collect();
                for &t in &moved {
                    chart[t] = into;
                }
                if is_disc(mesh, &chart, into) {
                    merged = true;
                    break;
                }
                for &t in &moved {
                    chart[t] = small as u32;
                }
            }
            if merged {
                break;
            }
        }
        if !merged {
            break;
        }
        let mut used = vec![u32::MAX; count];
        let mut next = 0;
        for c in chart.iter_mut() {
            if used[*c as usize] == u32::MAX {
                used[*c as usize] = next;
                next += 1;
            }
            *c = used[*c as usize];
        }
        count = next as usize;
    }
    Segmented { keys: chart, count }
}

/// Whether chart `id` has the Euler characteristic of a disc (one connected piece with no holes or handles).
fn is_disc(mesh: &Mesh, chart: &[u32], id: u32) -> bool {
    let mut verts = std::collections::HashSet::new();
    let mut edges = std::collections::HashSet::new();
    let mut faces = 0i64;
    for (t, &c) in chart.iter().enumerate() {
        if c != id {
            continue;
        }
        faces += 1;
        let tri = mesh.triangle(t);
        for k in 0..3 {
            verts.insert(tri[k]);
            edges.insert(edge_key(tri[k], tri[(k + 1) % 3]));
        }
    }
    verts.len() as i64 - edges.len() as i64 + faces == 1
}
