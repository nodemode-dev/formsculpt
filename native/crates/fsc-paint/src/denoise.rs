//! An edge-aware denoiser for the path render: the edge-avoiding a-trous wavelet filter (Dammertz et al., HPG 2010)
//! with the edge-stopping weights of spatiotemporal variance-guided filtering (Schied et al., HPG 2017), run spatially
//! on one frame. Five levels of a 5 x 5 B3-spline kernel whose taps spread 1, 2, 4, 8 and 16 pixels apart. Each tap is
//! weighted by how alike the two pixels' normals, depths and brightness are, the brightness by how noisy the pixel's own
//! samples were, so a clean pixel is left nearly alone and edges and textures stay sharp. The light is filtered with the
//! surface colour divided out and multiplied back in afterwards, so painted detail is not blurred.

use glam::Vec3;

/// What the filter is guided by, one entry per pixel. Pixels where no surface was hit are left as they are.
pub struct Guides<'a> {
    pub width: usize,
    pub height: usize,
    /// The first surface's colour, averaged over the pixel's samples.
    pub albedo: &'a [Vec3],
    /// The first surface's normal, averaged and normalised.
    pub normal: &'a [Vec3],
    /// Camera distance to the first surface.
    pub depth: &'a [f32],
    /// The variance of the pixel's mean brightness (the samples' variance over their count).
    pub variance: &'a [f32],
    pub hit: &'a [bool],
}

const H: [f32; 5] = [1.0 / 16.0, 1.0 / 4.0, 3.0 / 8.0, 1.0 / 4.0, 1.0 / 16.0];
const SIGMA_N: i32 = 128;
const SIGMA_L: f32 = 4.0;

fn luminance(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// Filters `color` (linear light, one per pixel) in place.
pub fn denoise(color: &mut [Vec3], g: &Guides) {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    if n == 0 || color.len() != n {
        return;
    }
    // The surface colour divided out; a floor keeps dark paint from amplifying noise.
    let floor = Vec3::splat(0.02);
    let mut light: Vec<Vec3> = (0..n).map(|i| if g.hit[i] { color[i] / g.albedo[i].max(floor) } else { color[i] }).collect();
    // The variance is of the demodulated light's brightness, so scale it the same way (by the brightness ratio squared).
    let mut var: Vec<f32> = (0..n)
        .map(|i| {
            if !g.hit[i] {
                return 0.0;
            }
            let k = 1.0 / luminance(g.albedo[i].max(floor)).max(0.02);
            g.variance[i] * k * k
        })
        .collect();
    // The depth gradient per pixel (the larger of the one-sided differences), so depth is compared against a plane.
    let at = |x: isize, y: isize| -> Option<usize> { (x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h).then(|| y as usize * w + x as usize) };
    let slope: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            if !g.hit[i] {
                return (0.0, 0.0);
            }
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            let d = |a: Option<usize>| a.filter(|&j| g.hit[j]).map(|j| (g.depth[j] - g.depth[i]).abs());
            let sx = d(at(x + 1, y)).into_iter().chain(d(at(x - 1, y))).fold(f32::INFINITY, f32::min);
            let sy = d(at(x, y + 1)).into_iter().chain(d(at(x, y - 1))).fold(f32::INFINITY, f32::min);
            (if sx.is_finite() { sx } else { 0.0 }, if sy.is_finite() { sy } else { 0.0 })
        })
        .collect();
    let eps = g.depth.iter().zip(g.hit).filter(|(_, h)| **h).map(|(d, _)| *d).fold(0.0f32, f32::max).max(1e-6) * 1e-3;
    for level in 0..5 {
        let step = 1isize << level;
        // A 3 x 3 Gaussian of the variance steers the brightness weight (it is not carried on).
        let blurred: Vec<f32> = (0..n)
            .map(|i| {
                if !g.hit[i] {
                    return 0.0;
                }
                let (x, y) = ((i % w) as isize, (i / w) as isize);
                let (mut s, mut t) = (0.0, 0.0);
                for dy in -1..=1isize {
                    for dx in -1..=1isize {
                        if let Some(j) = at(x + dx, y + dy).filter(|&j| g.hit[j]) {
                            let k = [0.25, 0.5, 0.25][(dx + 1) as usize] * [0.25, 0.5, 0.25][(dy + 1) as usize];
                            s += var[j] * k;
                            t += k;
                        }
                    }
                }
                s / t.max(1e-6)
            })
            .collect();
        let pass = |i: usize| -> (Vec3, f32) {
            if !g.hit[i] {
                return (light[i], var[i]);
            }
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            let (lp, np, zp, (gx, gy)) = (luminance(light[i]), g.normal[i], g.depth[i], slope[i]);
            let spread = SIGMA_L * blurred[i].max(0.0).sqrt() + 1e-6;
            let (mut sum, mut weights, mut vsum) = (Vec3::ZERO, 0.0f32, 0.0f32);
            for (ky, hy) in H.iter().enumerate() {
                for (kx, hx) in H.iter().enumerate() {
                    let (dx, dy) = ((kx as isize - 2) * step, (ky as isize - 2) * step);
                    let Some(j) = at(x + dx, y + dy).filter(|&j| g.hit[j]) else { continue };
                    let wz = (-(g.depth[j] - zp).abs() / (gx * dx.abs() as f32 + gy * dy.abs() as f32 + eps)).exp();
                    let wn = np.dot(g.normal[j]).max(0.0).powi(SIGMA_N);
                    let wl = (-(luminance(light[j]) - lp).abs() / spread).exp();
                    let k = hx * hy * wz * wn * wl;
                    sum += light[j] * k;
                    weights += k;
                    vsum += var[j] * k * k;
                }
            }
            if weights <= 1e-12 {
                return (light[i], var[i]);
            }
            (sum / weights, vsum / (weights * weights))
        };
        #[cfg(feature = "parallel")]
        let next: Vec<(Vec3, f32)> = {
            use rayon::prelude::*;
            (0..n).into_par_iter().map(pass).collect()
        };
        #[cfg(not(feature = "parallel"))]
        let next: Vec<(Vec3, f32)> = (0..n).map(pass).collect();
        for (i, (c, v)) in next.into_iter().enumerate() {
            light[i] = c;
            var[i] = v;
        }
    }
    for i in 0..n {
        if g.hit[i] {
            color[i] = light[i] * g.albedo[i].max(floor);
        }
    }
}
