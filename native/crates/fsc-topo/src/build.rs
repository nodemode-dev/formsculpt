use std::f32::consts::TAU;

use fsc_core::{Aabb, Mesh};
use glam::Vec3;

use crate::voxel::implicit_mesh;

#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub center: Vec3,
    pub radius: f32,
}

fn round_cone(p: Vec3, a: Ball, b: Ball) -> f32 {
    let ba = b.center - a.center;
    let l2 = ba.length_squared();
    let rr = a.radius - b.radius;
    if l2 <= rr * rr {
        let big = if a.radius > b.radius { a } else { b };
        return p.distance(big.center) - big.radius;
    }
    let a2 = l2 - rr * rr;
    let il2 = 1.0 / l2;
    let pa = p - a.center;
    let y = pa.dot(ba);
    let z = y - l2;
    let x2 = (pa * l2 - ba * y).length_squared();
    let y2 = y * y * l2;
    let z2 = z * z * l2;
    let k = rr.signum() * rr * rr * x2;
    if z.signum() * a2 * z2 > k {
        return (x2 + z2).sqrt() * il2 - b.radius;
    }
    if y.signum() * a2 * y2 < k {
        return (x2 + y2).sqrt() * il2 - a.radius;
    }
    ((x2 * a2 * il2).sqrt() + y * rr) * il2 - a.radius
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

pub fn skeleton_mesh(balls: &[Ball], links: &[(usize, usize)], blend: f32, resolution: u32) -> Result<Mesh, String> {
    if balls.is_empty() {
        return Err("give at least one sphere".into());
    }
    let mut bounds = Aabb::EMPTY;
    for b in balls {
        bounds.grow(b.center - Vec3::splat(b.radius * 1.2));
        bounds.grow(b.center + Vec3::splat(b.radius * 1.2));
    }
    let mean_r = balls.iter().map(|b| b.radius).sum::<f32>() / balls.len() as f32;
    let k = blend.clamp(0.0, 1.0) * mean_r;
    let linked: Vec<bool> = (0..balls.len()).map(|i| links.iter().any(|&(a, b)| a == i || b == i)).collect();
    let field = |p: Vec3| -> f32 {
        let mut d = f32::INFINITY;
        for &(a, b) in links {
            d = smin(d, round_cone(p, balls[a], balls[b]), k);
        }
        for (i, b) in balls.iter().enumerate() {
            if !linked[i] {
                d = smin(d, p.distance(b.center) - b.radius, k);
            }
        }
        d
    };
    let (mut mesh, _) = crate::manifold::split_nonmanifold(&implicit_mesh(bounds, resolution, field)?);
    mesh.quads = true;
    Ok(mesh)
}

fn flip_if_inside_out(mut mesh: Mesh) -> Mesh {
    if crate::voxel::signed_volume(&mesh) < 0.0 {
        for t in mesh.indices.chunks_mut(3) {
            t.swap(1, 2);
        }
        mesh.recompute_normals();
    }
    mesh
}

fn catmull(points: &[(Vec3, f32)], steps: usize, closed: bool) -> Vec<(Vec3, f32)> {
    let n = points.len();
    if n < 3 || steps <= 1 {
        return points.to_vec();
    }
    let at = |i: isize| -> (Vec3, f32) {
        if closed {
            points[i.rem_euclid(n as isize) as usize]
        } else {
            points[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let segs = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(segs * steps + 1);
    for s in 0..segs as isize {
        let (p0, p1, p2, p3) = (at(s - 1), at(s), at(s + 1), at(s + 2));
        for k in 0..steps {
            let t = k as f32 / steps as f32;
            let (t2, t3) = (t * t, t * t * t);
            let c = |a: f32, b: f32, c: f32, d: f32| 0.5 * (2.0 * b + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3);
            let p = Vec3::new(c(p0.0.x, p1.0.x, p2.0.x, p3.0.x), c(p0.0.y, p1.0.y, p2.0.y, p3.0.y), c(p0.0.z, p1.0.z, p2.0.z, p3.0.z));
            out.push((p, p1.1 + (p2.1 - p1.1) * t));
        }
    }
    if !closed {
        out.push(points[n - 1]);
    }
    out
}

pub fn tube(points: &[(Vec3, f32)], segments: usize, smooth: usize, closed: bool) -> Result<Mesh, String> {
    let mut pts: Vec<(Vec3, f32)> = Vec::with_capacity(points.len());
    for &p in points {
        if pts.last().is_none_or_far(p.0) {
            pts.push(p);
        }
    }
    if pts.len() < 2 {
        return Err("a tube needs at least two distinct points".into());
    }
    let closed = closed && pts.len() >= 3;
    let path = catmull(&pts, smooth.max(1), closed);
    let n = path.len();
    let seg = segments.clamp(3, 128);
    let tangent = |i: usize| -> Vec3 {
        let (prev, next) = if closed { (path[(i + n - 1) % n].0, path[(i + 1) % n].0) } else { (path[i.saturating_sub(1)].0, path[(i + 1).min(n - 1)].0) };
        (next - prev).normalize_or(Vec3::Y)
    };
    let mut normal = tangent(0).any_orthonormal_vector();
    let mut positions = Vec::with_capacity(n * seg + 2);
    for (i, &(c, r)) in path.iter().enumerate() {
        let t = tangent(i);
        normal = (normal - t * normal.dot(t)).try_normalize().unwrap_or_else(|| t.any_orthonormal_vector());
        let bin = t.cross(normal);
        for j in 0..seg {
            let a = TAU * j as f32 / seg as f32;
            positions.push((c + (normal * a.cos() + bin * a.sin()) * r.max(1e-5)).to_array());
        }
    }
    let ring = |i: usize, j: usize| (i * seg + j % seg) as u32;
    let mut indices = Vec::new();
    let rings = if closed { n } else { n - 1 };
    for i in 0..rings {
        let i2 = (i + 1) % n;
        for j in 0..seg {
            let (a, b, c, d) = (ring(i, j), ring(i, j + 1), ring(i2, j + 1), ring(i2, j));
            indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }
    if !closed {
        for (end, i) in [(0usize, 0usize), (1, n - 1)] {
            let center = positions.len() as u32;
            positions.push(path[i].0.to_array());
            for j in 0..seg {
                let (a, b) = (ring(i, j), ring(i, j + 1));
                if end == 0 {
                    indices.extend_from_slice(&[center, b, a]);
                } else {
                    indices.extend_from_slice(&[center, a, b]);
                }
            }
        }
    }
    Ok(flip_if_inside_out(Mesh::new(positions, indices)))
}

trait Far {
    fn is_none_or_far(&self, p: Vec3) -> bool;
}

impl Far for Option<&(Vec3, f32)> {
    fn is_none_or_far(&self, p: Vec3) -> bool {
        self.map_or(true, |q| q.0.distance(p) > 1e-5)
    }
}

pub fn lathe(profile: &[[f32; 2]], segments: usize) -> Result<Mesh, String> {
    if profile.len() < 2 {
        return Err("a profile needs at least two points".into());
    }
    let seg = segments.clamp(3, 256);
    let eps = 1e-5;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut rows: Vec<Vec<u32>> = Vec::new();
    for &[r, y] in profile {
        let r = r.max(0.0);
        if r <= eps {
            positions.push([0.0, y, 0.0]);
            rows.push(vec![positions.len() as u32 - 1]);
        } else {
            let row = (0..seg)
                .map(|j| {
                    let a = TAU * j as f32 / seg as f32;
                    positions.push([r * a.cos(), y, r * a.sin()]);
                    positions.len() as u32 - 1
                })
                .collect();
            rows.push(row);
        }
    }
    let at = |row: &Vec<u32>, j: usize| if row.len() == 1 { row[0] } else { row[j % seg] };
    let mut indices = Vec::new();
    for w in rows.windows(2) {
        let (r0, r1) = (&w[0], &w[1]);
        if r0.len() == 1 && r1.len() == 1 {
            continue;
        }
        for j in 0..seg {
            let (a, b, c, d) = (at(r0, j), at(r0, j + 1), at(r1, j + 1), at(r1, j));
            if r0.len() == 1 {
                indices.extend_from_slice(&[a, c, d]);
            } else if r1.len() == 1 {
                indices.extend_from_slice(&[a, b, c]);
            } else {
                indices.extend_from_slice(&[a, b, c, a, c, d]);
            }
        }
    }
    for (row, y, first) in [(&rows[0], profile[0][1], true), (&rows[rows.len() - 1], profile[profile.len() - 1][1], false)] {
        if row.len() == 1 {
            continue;
        }
        positions.push([0.0, y, 0.0]);
        let center = positions.len() as u32 - 1;
        for j in 0..seg {
            let (a, b) = (row[j], row[(j + 1) % seg]);
            if first {
                indices.extend_from_slice(&[center, b, a]);
            } else {
                indices.extend_from_slice(&[center, a, b]);
            }
        }
    }
    if indices.is_empty() {
        return Err("the profile has no width".into());
    }
    Ok(flip_if_inside_out(Mesh::new(positions, indices)))
}
