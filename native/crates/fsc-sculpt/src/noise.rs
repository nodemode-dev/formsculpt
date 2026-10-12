//! Noise patterns for surface noise: fractal value noise, ridged and turbulent variants, cellular (Worley) noise
//! and an alpha tiled over the surface by triplanar projection. Each returns roughly -1..1 so `amount` reads the same
//! for every pattern.

use glam::Vec3;

use crate::alpha::Alpha;
use crate::stroke::value_noise;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoisePattern {
    Value,
    Ridged,
    Turbulence,
    Cells,
    Alpha,
}

impl NoisePattern {
    pub const NAMES: [&'static str; 5] = ["value", "ridged", "turbulence", "cells", "alpha"];

    pub fn parse(name: &str) -> Option<NoisePattern> {
        Some(match name {
            "value" => NoisePattern::Value,
            "ridged" => NoisePattern::Ridged,
            "turbulence" => NoisePattern::Turbulence,
            "cells" => NoisePattern::Cells,
            "alpha" => NoisePattern::Alpha,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoiseSettings {
    pub pattern: NoisePattern,
    /// Layers of detail, each at twice the frequency (1 to 8).
    pub octaves: u32,
    /// How much each finer octave counts against the one before (0.1 to 0.9).
    pub gain: f32,
}

/// Value noise summed over octaves, normalised to about -1..1.
fn fractal(p: Vec3, s: &NoiseSettings) -> f32 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut norm = 0.0;
    let mut q = p;
    for o in 0..s.octaves.clamp(1, 8) {
        sum += value_noise(q + Vec3::splat(o as f32 * 17.13)) * amp;
        norm += amp;
        amp *= s.gain.clamp(0.1, 0.9);
        q *= 2.03;
    }
    sum / norm
}

fn hash(c: [i32; 3]) -> Vec3 {
    let mut h = (c[0] as u32).wrapping_mul(0x8da6_b343) ^ (c[1] as u32).wrapping_mul(0xd816_3841) ^ (c[2] as u32).wrapping_mul(0xcb1a_b31f);
    let mut next = || {
        h ^= h >> 13;
        h = h.wrapping_mul(0x5bd1_e995);
        h ^= h >> 15;
        (h & 0xffff) as f32 / 65535.0
    };
    Vec3::new(next(), next(), next())
}

/// Worley noise: distance to the nearest of one jittered point per cell, as -1 at a point to about 1 at a border.
fn cells(p: Vec3) -> f32 {
    let f = p.floor();
    let mut best = f32::MAX;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let c = [f.x as i32 + dx, f.y as i32 + dy, f.z as i32 + dz];
                let q = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32) + hash(c);
                best = best.min(q.distance_squared(p));
            }
        }
    }
    (best.sqrt() * 2.0 - 1.0).clamp(-1.0, 1.0)
}

/// The pattern's value at `p` (in feature-size units) on a surface whose normal is `n`.
pub fn noise_at(p: Vec3, n: Vec3, s: &NoiseSettings, alpha: Option<&Alpha>) -> f32 {
    match s.pattern {
        NoisePattern::Value => fractal(p, s),
        NoisePattern::Ridged => 1.0 - 2.0 * fractal(p, s).abs(),
        NoisePattern::Turbulence => 2.0 * fractal(p, s).abs() - 1.0,
        NoisePattern::Cells => {
            let mut sum = 0.0;
            let mut amp = 1.0;
            let mut norm = 0.0;
            let mut q = p;
            for _ in 0..s.octaves.clamp(1, 8) {
                sum += cells(q) * amp;
                norm += amp;
                amp *= s.gain.clamp(0.1, 0.9);
                q *= 2.03;
            }
            sum / norm
        }
        NoisePattern::Alpha => {
            // Triplanar: the alpha tiled on the three axis planes, blended by how squarely the surface faces each.
            let Some(a) = alpha else { return 0.0 };
            let w = Vec3::new(n.x.abs(), n.y.abs(), n.z.abs()).powf(4.0);
            let total = (w.x + w.y + w.z).max(1e-6);
            let tile = |u: f32, v: f32| a.sample(u.rem_euclid(1.0) * 2.0 - 1.0, v.rem_euclid(1.0) * 2.0 - 1.0);
            let value = (tile(p.y, p.z) * w.x + tile(p.x, p.z) * w.y + tile(p.x, p.y) * w.z) / total;
            value * 2.0 - 1.0
        }
    }
}
