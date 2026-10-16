use std::f32::consts::{PI, TAU};

/// A grayscale brush alpha: values 0 to 1 on a square grid, sampled across the brush's disc. White pushes fully, black
/// not at all, so an alpha shapes a stroke into a pattern (scales, cracks, rivets) the way ZBrush's and Nomad's do.
#[derive(Debug, Clone, PartialEq)]
pub struct Alpha {
    pub size: usize,
    pub data: Vec<f32>,
}

impl Alpha {
    pub const BUILTIN: [&'static str; 7] = ["square", "noise", "cracks", "rings", "star", "stripes", "dots"];

    pub fn from_fn(size: usize, f: impl Fn(f32, f32) -> f32) -> Alpha {
        let mut data = Vec::with_capacity(size * size);
        for j in 0..size {
            for i in 0..size {
                // -1 to 1 across, y up.
                let x = (i as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let y = 1.0 - (j as f32 + 0.5) / size as f32 * 2.0;
                data.push(f(x, y).clamp(0.0, 1.0));
            }
        }
        Alpha { size, data }
    }

    /// The built-in alphas, generated rather than shipped as images.
    pub fn builtin(name: &str) -> Option<Alpha> {
        let n = 128;
        let hash = |x: i32, y: i32| -> f32 {
            let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841);
            h ^= h >> 13;
            h = h.wrapping_mul(0x5bd1_e995);
            (h & 0xffff) as f32 / 65535.0
        };
        Some(match name {
            "square" => Alpha::from_fn(n, |x, y| if x.abs() < 0.8 && y.abs() < 0.8 { 1.0 } else { 0.0 }),
            "noise" => Alpha::from_fn(n, |x, y| {
                let (gx, gy) = ((x + 1.0) * 6.0, (y + 1.0) * 6.0);
                let (ix, iy) = (gx.floor() as i32, gy.floor() as i32);
                let (fx, fy) = (gx - gx.floor(), gy - gy.floor());
                let s = |t: f32| t * t * (3.0 - 2.0 * t);
                let a = hash(ix, iy) + (hash(ix + 1, iy) - hash(ix, iy)) * s(fx);
                let b = hash(ix, iy + 1) + (hash(ix + 1, iy + 1) - hash(ix, iy + 1)) * s(fx);
                a + (b - a) * s(fy)
            }),
            "cracks" => Alpha::from_fn(n, |x, y| {
                // Distance to the nearest edge between Voronoi cells: thin dark lines become white grooves.
                let (gx, gy) = ((x + 1.0) * 3.0, (y + 1.0) * 3.0);
                let (ix, iy) = (gx.floor() as i32, gy.floor() as i32);
                let mut d1 = f32::INFINITY;
                let mut d2 = f32::INFINITY;
                for oy in -1..=1 {
                    for ox in -1..=1 {
                        let (cx, cy) = (ix + ox, iy + oy);
                        let px = cx as f32 + hash(cx, cy);
                        let py = cy as f32 + hash(cy * 7, cx * 13);
                        let d = ((gx - px).powi(2) + (gy - py).powi(2)).sqrt();
                        if d < d1 {
                            d2 = d1;
                            d1 = d;
                        } else if d < d2 {
                            d2 = d;
                        }
                    }
                }
                1.0 - ((d2 - d1) / 0.12).min(1.0)
            }),
            "rings" => Alpha::from_fn(n, |x, y| 0.5 + 0.5 * ((x * x + y * y).sqrt() * PI * 5.0).cos()),
            "star" => Alpha::from_fn(n, |x, y| {
                let r = (x * x + y * y).sqrt();
                let a = y.atan2(x);
                let edge = 0.55 + 0.35 * (a * 5.0).cos();
                if r < edge { 1.0 } else { 0.0 }
            }),
            "stripes" => Alpha::from_fn(n, |_, y| 0.5 + 0.5 * (y * PI * 6.0).cos()),
            "dots" => Alpha::from_fn(n, |x, y| {
                let (fx, fy) = (((x + 1.0) * 3.0).fract() - 0.5, ((y + 1.0) * 3.0).fract() - 0.5);
                if fx * fx + fy * fy < 0.09 { 1.0 } else { 0.0 }
            }),
            _ => return None,
        })
    }

    /// Bilinear sample at (x, y) in -1 to 1 across the disc, y up; 0 outside.
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        if !(-1.0..=1.0).contains(&x) || !(-1.0..=1.0).contains(&y) {
            return 0.0;
        }
        let n = self.size as f32;
        let fx = ((x + 1.0) * 0.5 * n - 0.5).clamp(0.0, n - 1.0);
        let fy = ((1.0 - y) * 0.5 * n - 0.5).clamp(0.0, n - 1.0);
        let (i, j) = (fx.floor() as usize, fy.floor() as usize);
        let (i1, j1) = ((i + 1).min(self.size - 1), (j + 1).min(self.size - 1));
        let (tx, ty) = (fx - i as f32, fy - j as f32);
        let at = |i: usize, j: usize| self.data[j * self.size + i];
        let a = at(i, j) + (at(i1, j) - at(i, j)) * tx;
        let b = at(i, j1) + (at(i1, j1) - at(i, j1)) * tx;
        a + (b - a) * ty
    }

    /// The signed stamp weight at alpha-space (x, y): v divided by `stretch`, repeated `tile` times, sampled, inverted, then split at `mid`; 0 outside the alpha.
    pub fn stamp(&self, x: f32, y: f32, stretch: f32, tile: u32, invert: bool, mid: f32) -> f32 {
        let (mut x, mut y) = (x, y / stretch);
        if !(-1.0..=1.0).contains(&x) || !(-1.0..=1.0).contains(&y) {
            return 0.0;
        }
        if tile > 1 {
            let n = tile as f32;
            x = ((x + 1.0) * 0.5 * n).rem_euclid(1.0) * 2.0 - 1.0;
            y = ((y + 1.0) * 0.5 * n).rem_euclid(1.0) * 2.0 - 1.0;
        }
        let mut a = self.sample(x, y);
        if invert {
            a = 1.0 - a;
        }
        if mid > 0.0 {
            a = (if a > mid { (a - mid) / (1.0 - mid) } else { (a - mid) / mid }).clamp(-1.0, 1.0);
        }
        a
    }

    /// An alpha from 8-bit grayscale (or the mean of RGB) pixels, resampled to a square of `size`.
    pub fn from_pixels(width: usize, height: usize, gray: &[u8], size: usize) -> Result<Alpha, String> {
        if width == 0 || height == 0 || gray.len() != width * height {
            return Err("the alpha image is empty or its size does not match its pixels".into());
        }
        let size = size.clamp(16, 512);
        Ok(Alpha::from_fn(size, |x, y| {
            let px = ((x + 1.0) * 0.5 * width as f32).clamp(0.0, width as f32 - 1.0) as usize;
            let py = ((1.0 - y) * 0.5 * height as f32).clamp(0.0, height as f32 - 1.0) as usize;
            gray[py * width + px] as f32 / 255.0
        }))
    }
}

/// Rotates (x, y) by `angle` radians: the alpha's own turn, from the stroke direction (roll) or a set angle.
pub fn turn(x: f32, y: f32, angle: f32) -> (f32, f32) {
    let (s, c) = (angle % TAU).sin_cos();
    (x * c - y * s, x * s + y * c)
}
