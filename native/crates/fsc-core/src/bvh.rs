use glam::Vec3;

use crate::mesh::{Aabb, Mesh};

const LEAF_SIZE: usize = 8;
const BINS: usize = 12;

#[derive(Debug, Clone, Copy)]
pub struct Node {
    pub bounds: Aabb,
    pub first: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Bvh {
    pub nodes: Vec<Node>,
    pub tris: Vec<u32>,
    parent: Vec<u32>,
    leaf_of: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub triangle: u32,
    pub t: f32,
    pub point: Vec3,
    pub u: f32,
    pub v: f32,
}

impl Bvh {
    pub fn build(mesh: &Mesh) -> Bvh {
        let n = mesh.triangle_count();
        let mut boxes = Vec::with_capacity(n);
        let mut centers = Vec::with_capacity(n);
        for t in 0..n {
            let mut b = Aabb::EMPTY;
            for v in mesh.triangle(t) {
                b.grow(mesh.position(v));
            }
            centers.push(b.center());
            boxes.push(b);
        }
        let mut bvh = Bvh { nodes: Vec::with_capacity(2 * n / LEAF_SIZE + 1), tris: (0..n as u32).collect(), parent: Vec::new(), leaf_of: Vec::new() };
        if n == 0 {
            return bvh;
        }
        bvh.nodes.push(Node { bounds: Aabb::EMPTY, first: 0, count: n as u32 });
        let mut stack = vec![0usize];
        while let Some(ni) = stack.pop() {
            let (first, count) = (bvh.nodes[ni].first as usize, bvh.nodes[ni].count as usize);
            let range = &mut bvh.tris[first..first + count];
            let mut bounds = Aabb::EMPTY;
            let mut cb = Aabb::EMPTY;
            for &t in range.iter() {
                bounds = bounds.merge(&boxes[t as usize]);
                cb.grow(centers[t as usize]);
            }
            bvh.nodes[ni].bounds = bounds;
            if count <= LEAF_SIZE {
                continue;
            }
            let ext = cb.extent();
            let axis = if ext.x >= ext.y && ext.x >= ext.z { 0 } else if ext.y >= ext.z { 1 } else { 2 };
            if ext[axis] <= f32::EPSILON {
                continue;
            }
            let split = sah_split(range, &boxes, &centers, cb.min[axis], ext[axis], axis);
            let mid = partition(range, |t| centers[t as usize][axis] < split);
            let mid = if mid == 0 || mid == count { count / 2 } else { mid };
            if mid == count / 2 && !(0..count).any(|i| centers[range[i] as usize][axis] < split) {
                range.select_nth_unstable_by(mid, |a, b| centers[*a as usize][axis].total_cmp(&centers[*b as usize][axis]));
            }
            let left = bvh.nodes.len();
            bvh.nodes.push(Node { bounds: Aabb::EMPTY, first: first as u32, count: mid as u32 });
            bvh.nodes.push(Node { bounds: Aabb::EMPTY, first: (first + mid) as u32, count: (count - mid) as u32 });
            bvh.nodes[ni].first = left as u32;
            bvh.nodes[ni].count = 0;
            stack.push(left);
            stack.push(left + 1);
        }
        bvh.parent = vec![u32::MAX; bvh.nodes.len()];
        bvh.leaf_of = vec![0; n];
        for (ni, node) in bvh.nodes.iter().enumerate() {
            if Bvh::is_leaf(node) {
                for &t in &bvh.tris[node.first as usize..(node.first + node.count) as usize] {
                    bvh.leaf_of[t as usize] = ni as u32;
                }
            } else {
                bvh.parent[node.first as usize] = ni as u32;
                bvh.parent[node.first as usize + 1] = ni as u32;
            }
        }
        bvh
    }

    pub fn refit_triangles(&mut self, mesh: &Mesh, tris: impl IntoIterator<Item = u32>) {
        let mut heap: std::collections::BinaryHeap<u32> = tris.into_iter().filter_map(|t| self.leaf_of.get(t as usize).copied()).collect();
        if heap.len() > self.tris.len() / 4 {
            self.refit(mesh);
            return;
        }
        let mut last = u32::MAX;
        while let Some(ni) = heap.pop() {
            if ni == last {
                continue;
            }
            last = ni;
            let node = self.nodes[ni as usize];
            let mut b = Aabb::EMPTY;
            if Bvh::is_leaf(&node) {
                for &t in &self.tris[node.first as usize..(node.first + node.count) as usize] {
                    for v in mesh.triangle(t as usize) {
                        b.grow(mesh.position(v));
                    }
                }
            } else {
                b = self.nodes[node.first as usize].bounds.merge(&self.nodes[node.first as usize + 1].bounds);
            }
            if b == node.bounds {
                continue;
            }
            self.nodes[ni as usize].bounds = b;
            if let Some(&p) = self.parent.get(ni as usize).filter(|&&p| p != u32::MAX) {
                heap.push(p);
            }
        }
    }

    pub fn refit(&mut self, mesh: &Mesh) {
        for ni in (0..self.nodes.len()).rev() {
            let node = self.nodes[ni];
            let mut b = Aabb::EMPTY;
            if Bvh::is_leaf(&node) {
                for &t in &self.tris[node.first as usize..(node.first + node.count) as usize] {
                    for v in mesh.triangle(t as usize) {
                        b.grow(mesh.position(v));
                    }
                }
            } else {
                let (l, r) = (self.nodes[node.first as usize].bounds, self.nodes[node.first as usize + 1].bounds);
                b = l.merge(&r);
            }
            self.nodes[ni].bounds = b;
        }
    }

    pub fn is_leaf(node: &Node) -> bool {
        node.count > 0
    }

    pub fn raycast(&self, mesh: &Mesh, origin: Vec3, dir: Vec3) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = Vec3::ONE / dir;
        let mut best: Option<Hit> = None;
        let mut t_max = f32::INFINITY;
        let mut stack = vec![0usize];
        // Hidden faces are not there for the ray: brushes, picking and the stroke start pass through them.
        let skip_hidden = mesh.has_hidden();
        while let Some(ni) = stack.pop() {
            let node = &self.nodes[ni];
            if node.bounds.ray_entry(origin, inv, t_max).is_none() {
                continue;
            }
            if Bvh::is_leaf(node) {
                for &t in &self.tris[node.first as usize..(node.first + node.count) as usize] {
                    if skip_hidden && mesh.hidden[t as usize] {
                        continue;
                    }
                    let [a, b, c] = mesh.triangle(t as usize);
                    if let Some((d, u, v)) = intersect(origin, dir, mesh.position(a), mesh.position(b), mesh.position(c)) {
                        if d < t_max {
                            t_max = d;
                            best = Some(Hit { triangle: t, t: d, point: origin + dir * d, u, v });
                        }
                    }
                }
            } else {
                stack.push(node.first as usize);
                stack.push(node.first as usize + 1);
            }
        }
        best
    }

    /// The visible triangles whose bounding box reaches within `radius` of `center`.
    pub fn triangles_in_sphere(&self, mesh: &Mesh, center: Vec3, radius: f32, out: &mut Vec<u32>) {
        out.clear();
        if self.nodes.is_empty() {
            return;
        }
        let r2 = radius * radius;
        let mut stack = vec![0usize];
        while let Some(ni) = stack.pop() {
            let node = &self.nodes[ni];
            if node.bounds.distance_sq_to(center) > r2 {
                continue;
            }
            if Bvh::is_leaf(node) {
                for &t in &self.tris[node.first as usize..(node.first + node.count) as usize] {
                    if mesh.face_hidden(t as usize) {
                        continue;
                    }
                    let mut b = Aabb::EMPTY;
                    for v in mesh.triangle(t as usize) {
                        b.grow(mesh.position(v));
                    }
                    if b.distance_sq_to(center) <= r2 {
                        out.push(t);
                    }
                }
            } else {
                stack.push(node.first as usize);
                stack.push(node.first as usize + 1);
            }
        }
        out.sort_unstable();
        out.dedup();
    }

    pub fn vertices_in_sphere(&self, mesh: &Mesh, center: Vec3, radius: f32, out: &mut Vec<u32>) {
        out.clear();
        if self.nodes.is_empty() {
            return;
        }
        let r2 = radius * radius;
        let mut stack = vec![0usize];
        while let Some(ni) = stack.pop() {
            let node = &self.nodes[ni];
            if node.bounds.distance_sq_to(center) > r2 {
                continue;
            }
            if Bvh::is_leaf(node) {
                for &t in &self.tris[node.first as usize..(node.first + node.count) as usize] {
                    for v in mesh.triangle(t as usize) {
                        if mesh.position(v).distance_squared(center) <= r2 {
                            out.push(v);
                        }
                    }
                }
            } else {
                stack.push(node.first as usize);
                stack.push(node.first as usize + 1);
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}

fn sah_split(range: &[u32], boxes: &[Aabb], centers: &[Vec3], lo: f32, extent: f32, axis: usize) -> f32 {
    let mut bins = [(Aabb::EMPTY, 0u32); BINS];
    let scale = BINS as f32 / extent;
    for &t in range {
        let b = (((centers[t as usize][axis] - lo) * scale) as usize).min(BINS - 1);
        bins[b].0 = bins[b].0.merge(&boxes[t as usize]);
        bins[b].1 += 1;
    }
    let mut best = (f32::INFINITY, lo + extent * 0.5);
    for split in 1..BINS {
        let (mut lb, mut lc, mut rb, mut rc) = (Aabb::EMPTY, 0u32, Aabb::EMPTY, 0u32);
        for (b, c) in &bins[..split] {
            lb = lb.merge(b);
            lc += c;
        }
        for (b, c) in &bins[split..] {
            rb = rb.merge(b);
            rc += c;
        }
        if lc == 0 || rc == 0 {
            continue;
        }
        let cost = lb.surface_area() * lc as f32 + rb.surface_area() * rc as f32;
        if cost < best.0 {
            best = (cost, lo + extent * split as f32 / BINS as f32);
        }
    }
    best.1
}

fn partition(range: &mut [u32], pred: impl Fn(u32) -> bool) -> usize {
    let mut i = 0;
    for j in 0..range.len() {
        if pred(range[j]) {
            range.swap(i, j);
            i += 1;
        }
    }
    i
}

fn intersect(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<(f32, f32, f32)> {
    let e1 = b - a;
    let e2 = c - a;
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-6).then_some((t, u, v))
}
