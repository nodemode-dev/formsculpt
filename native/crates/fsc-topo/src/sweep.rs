use std::f32::consts::{FRAC_PI_2, TAU};

use fsc_core::{CurveSample, Mesh};
use glam::{Vec2, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cap {
    Flat,
    Round,
    None,
}

#[derive(Debug, Clone)]
pub struct SweepOptions {
    pub profile: Vec<Vec2>,
    pub twist: f32,
    pub spiral_turns: f32,
    pub spiral_offset: f32,
    pub cap: Cap,
    pub hole: f32,
}

const ROUND_CAP_RINGS: usize = 6;

fn strip(indices: &mut Vec<u32>, a: u32, b: u32, m: usize, flip: bool) {
    for j in 0..m {
        let (p, q, r, s) = (a + j as u32, a + ((j + 1) % m) as u32, b + ((j + 1) % m) as u32, b + j as u32);
        if flip {
            indices.extend_from_slice(&[p, r, q, p, s, r]);
        } else {
            indices.extend_from_slice(&[p, q, r, p, r, s]);
        }
    }
}

fn signed_area(poly: &[Vec2]) -> f32 {
    let n = poly.len();
    (0..n).map(|i| poly[i].perp_dot(poly[(i + 1) % n])).sum::<f32>() * 0.5
}

pub fn sweep(samples: &[CurveSample], closed: bool, o: &SweepOptions) -> Result<Mesh, String> {
    let n = samples.len();
    if n < 2 {
        return Err("a sweep needs at least two distinct points".into());
    }
    let mut profile = o.profile.clone();
    if profile.len() < 3 {
        return Err("a profile needs at least three points".into());
    }
    if signed_area(&profile) < 0.0 {
        profile.reverse();
    }
    let m = profile.len();
    let closed = closed && n >= 3;

    let mut normals: Vec<Vec3> = Vec::with_capacity(n);
    let mut nrm = samples[0].tangent.any_orthonormal_vector();
    for s in samples {
        nrm = (nrm - s.tangent * nrm.dot(s.tangent)).try_normalize().unwrap_or_else(|| s.tangent.any_orthonormal_vector());
        normals.push(nrm);
    }
    let mut holonomy = 0.0;
    if closed {
        let t0 = samples[0].tangent;
        if let Some(a) = (normals[n - 1] - t0 * normals[n - 1].dot(t0)).try_normalize() {
            holonomy = t0.dot(a.cross(normals[0])).atan2(a.dot(normals[0]));
        }
    }

    let hollow = o.hole > 0.0;
    let ring = |i: usize, scale: f32, shift: f32, along: f32| -> Vec<Vec3> {
        let s = &samples[i];
        let t = s.tangent;
        let share = holonomy * i as f32 / n as f32;
        let normal = normals[i];
        let bin = t.cross(normal);
        let theta = s.twist + o.twist * s.t + TAU * o.spiral_turns * s.t + share;
        let phi = TAU * o.spiral_turns * s.t + share;
        let offset = (normal * phi.cos() + bin * phi.sin()) * o.spiral_offset;
        let (prev, next) = if closed {
            (Some(samples[(i + n - 1) % n].pos), Some(samples[(i + 1) % n].pos))
        } else {
            (if i > 0 { Some(samples[i - 1].pos) } else { None }, if i + 1 < n { Some(samples[i + 1].pos) } else { None })
        };
        let mut miter = Vec3::ZERO;
        let mut factor = 1.0;
        if let (Some(p), Some(q)) = (prev, next) {
            let (d0, d1) = ((s.pos - p).normalize_or_zero(), (q - s.pos).normalize_or_zero());
            if let Some(dir) = (d1 - d0).try_normalize() {
                miter = dir;
                factor = 1.0 / t.dot(d0).max(1.0 / 3.0);
            }
        }
        let (sin, cos) = theta.sin_cos();
        let r = s.radius.max(1e-5) * scale;
        profile
            .iter()
            .map(|q| {
                let (x, y) = ((q.x * cos - q.y * sin) * r, (q.x * sin + q.y * cos) * r);
                let mut v = normal * x + bin * y;
                v += miter * (v.dot(miter) * (factor - 1.0));
                s.pos + offset + v + t * (along * shift)
            })
            .collect()
    };
    let inner_scale = |i: usize| -> f32 {
        let r = samples[i].radius.max(1e-5);
        ((r - o.hole) / r).max(0.05)
    };

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let push_ring = |positions: &mut Vec<[f32; 3]>, pts: Vec<Vec3>| -> u32 {
        let start = positions.len() as u32;
        positions.extend(pts.into_iter().map(|p| p.to_array()));
        start
    };

    let round = o.cap == Cap::Round && !hollow && !closed;
    let mut starts: Vec<u32> = Vec::new();
    if round {
        for k in (1..ROUND_CAP_RINGS).rev() {
            let a = FRAC_PI_2 * k as f32 / ROUND_CAP_RINGS as f32;
            let r = samples[0].radius.max(1e-5);
            starts.push(push_ring(&mut positions, ring(0, a.cos(), -a.sin() * r, 1.0)));
        }
    }
    let main_first = starts.len();
    for i in 0..n {
        starts.push(push_ring(&mut positions, ring(i, 1.0, 0.0, 0.0)));
    }
    let main_last = starts.len() - 1;
    if round {
        for k in 1..ROUND_CAP_RINGS {
            let a = FRAC_PI_2 * k as f32 / ROUND_CAP_RINGS as f32;
            let r = samples[n - 1].radius.max(1e-5);
            starts.push(push_ring(&mut positions, ring(n - 1, a.cos(), a.sin() * r, 1.0)));
        }
    }
    for w in starts.windows(2) {
        strip(&mut indices, w[0], w[1], m, false);
    }
    if closed {
        strip(&mut indices, starts[main_last], starts[main_first], m, false);
    }

    let mut inner: Vec<u32> = Vec::new();
    if hollow {
        for i in 0..n {
            inner.push(push_ring(&mut positions, ring(i, inner_scale(i), 0.0, 0.0)));
        }
        for w in inner.windows(2) {
            strip(&mut indices, w[0], w[1], m, true);
        }
        if closed {
            strip(&mut indices, inner[n - 1], inner[0], m, true);
        }
    }

    if !closed && o.cap != Cap::None {
        let ends: [(u32, Option<u32>, bool); 2] = [(starts[0], inner.first().copied(), true), (starts[starts.len() - 1], inner.last().copied(), false)];
        for (outer, inside, first) in ends {
            if let Some(inside) = inside {
                for j in 0..m as u32 {
                    let k = (j + 1) % m as u32;
                    let (oj, ok, ij, ik) = (outer + j, outer + k, inside + j, inside + k);
                    if first {
                        indices.extend_from_slice(&[oj, ij, ik, oj, ik, ok]);
                    } else {
                        indices.extend_from_slice(&[oj, ok, ik, oj, ik, ij]);
                    }
                }
                continue;
            }
            let center = positions.len() as u32;
            let apex = if round {
                let (i, sign) = if first { (0, -1.0) } else { (n - 1, 1.0) };
                samples[i].pos + samples[i].tangent * (sign * samples[i].radius.max(1e-5))
            } else {
                let sum: Vec3 = (0..m).map(|j| Vec3::from_array(positions[(outer as usize) + j])).sum();
                sum / m as f32
            };
            positions.push(apex.to_array());
            for j in 0..m as u32 {
                let (a, b) = (outer + j, outer + (j + 1) % m as u32);
                if first {
                    indices.extend_from_slice(&[center, b, a]);
                } else {
                    indices.extend_from_slice(&[center, a, b]);
                }
            }
        }
    }

    let mut mesh = Mesh::new(positions, indices);
    let watertight = closed || o.cap != Cap::None;
    if watertight && crate::voxel::signed_volume(&mesh) < 0.0 {
        for t in mesh.indices.chunks_mut(3) {
            t.swap(1, 2);
        }
        mesh.recompute_normals();
    }
    Ok(mesh)
}
