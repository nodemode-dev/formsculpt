use fsc_core::{Bvh, Mesh};
use glam::Vec3;

pub fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (Vec3, [f32; 3]) {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, [1.0, 0.0, 0.0]);
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, [0.0, 1.0, 0.0]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (a + ab * v, [1.0 - v, v, 0.0]);
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, [0.0, 0.0, 1.0]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (a + ac * w, [1.0 - w, 0.0, w]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (b + (c - b) * w, [0.0, 1.0 - w, w]);
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (a + ab * v + ac * w, [1.0 - v - w, v, w])
}

pub fn nearest_point(bvh: &Bvh, mesh: &Mesh, p: Vec3, max_dist: f32) -> Option<(Vec3, Vec3, f32)> {
    nearest_feature(bvh, mesh, p, max_dist).map(|(q, t, bary, d)| {
        let [ia, ib, ic] = mesh.triangle(t as usize);
        let n = Vec3::from_array(mesh.normals[ia as usize]) * bary[0] + Vec3::from_array(mesh.normals[ib as usize]) * bary[1] + Vec3::from_array(mesh.normals[ic as usize]) * bary[2];
        (q, n, d)
    })
}

pub fn nearest_feature(bvh: &Bvh, mesh: &Mesh, p: Vec3, max_dist: f32) -> Option<(Vec3, u32, [f32; 3], f32)> {
    if bvh.nodes.is_empty() {
        return None;
    }
    let mut best_d2 = max_dist * max_dist;
    let mut best: Option<(Vec3, u32, [f32; 3])> = None;
    let mut stack = vec![0usize];
    while let Some(ni) = stack.pop() {
        let node = &bvh.nodes[ni];
        if node.bounds.distance_sq_to(p) > best_d2 {
            continue;
        }
        if Bvh::is_leaf(node) {
            for &t in &bvh.tris[node.first as usize..(node.first + node.count) as usize] {
                let [ia, ib, ic] = mesh.triangle(t as usize);
                let (q, bary) = closest_on_triangle(p, mesh.position(ia), mesh.position(ib), mesh.position(ic));
                let d2 = q.distance_squared(p);
                if d2 < best_d2 {
                    best_d2 = d2;
                    best = Some((q, t, bary));
                }
            }
        } else {
            let (l, r) = (node.first as usize, node.first as usize + 1);
            let (dl, dr) = (bvh.nodes[l].bounds.distance_sq_to(p), bvh.nodes[r].bounds.distance_sq_to(p));
            if dl < dr {
                stack.push(r);
                stack.push(l);
            } else {
                stack.push(l);
                stack.push(r);
            }
        }
    }
    best.map(|(q, t, bary)| (q, t, bary, best_d2.sqrt()))
}
