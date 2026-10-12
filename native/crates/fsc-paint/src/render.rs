use std::time::Instant;

use fsc_core::{Aabb, Mesh};
use glam::Vec3;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// One mesh to draw: its mask darkens like the viewport's, `active` picks the warmer clay.
pub struct RenderObject<'a> {
    pub mesh: &'a Mesh,
    pub mask: &'a [f32],
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shading {
    Matcap { key: f32, spec: f32, sharp: f32, rim: f32, tint: [f32; 3] },
    Pbr,
    /// The clay matcap with each face group in its own colour (as the window's Face groups display).
    Groups,
}

/// The window's group colours (groupColor in gl-renderer.ts): hues spaced by the golden ratio, as pastel HSL.
pub fn group_color(g: u32) -> Vec3 {
    let h = (g as f32 * 0.618_034) % 1.0;
    let (s, l) = (0.45f32, 0.66f32);
    let a = s * l.min(1.0 - l);
    let f = |n: f32| {
        let k = (n + h * 12.0) % 12.0;
        l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    Vec3::new(f(0.0), f(8.0), f(4.0))
}

impl Shading {
    pub const NAMES: [&'static str; 7] = ["clay", "wax", "pearl", "bronze", "gray", "pbr", "groups"];

    /// The window's presets, so an agent sees what the person sees.
    pub fn named(name: &str) -> Option<Self> {
        let m = |key, spec, sharp, rim, tint| Some(Shading::Matcap { key, spec, sharp, rim, tint });
        match name {
            "clay" => m(0.75, 0.45, 6.0, 0.35, [1.0, 1.0, 1.0]),
            "wax" => m(0.85, 0.7, 10.0, 0.2, [1.0, 0.55, 0.5]),
            "pearl" => m(0.6, 0.6, 4.0, 0.7, [1.0, 0.97, 0.95]),
            "bronze" => m(0.45, 0.9, 14.0, 0.25, [0.85, 0.6, 0.38]),
            "gray" => m(0.8, 0.25, 4.0, 0.2, [0.78, 0.78, 0.82]),
            "pbr" => Some(Shading::Pbr),
            "groups" => Some(Shading::Groups),
            _ => None,
        }
    }
}

pub struct View {
    /// Radians about y from the front (+z) view, and above the horizon.
    pub yaw: f32,
    pub pitch: f32,
    pub width: usize,
    pub height: usize,
    pub shading: Shading,
    /// None leaves the background transparent.
    pub background: Option<[f32; 3]>,
    /// What to frame; the bounds of every object when None.
    pub frame: Option<Aabb>,
    /// Samples per pixel along each axis (1 to 3).
    pub supersample: usize,
}

pub struct Rendered {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 4]>,
    pub triangles: usize,
    pub covered: f32,
    pub ms: u128,
}

const FOV: f32 = 40.0 * std::f32::consts::PI / 180.0;
const KEY: Vec3 = Vec3::new(-0.45, 0.65, 0.6);
const FILL: Vec3 = Vec3::new(0.6, 0.1, 0.5);

struct Camera {
    eye: Vec3,
    right: Vec3,
    up: Vec3,
    fwd: Vec3,
    tan: f32,
    aspect: f32,
}

impl Camera {
    fn new(bounds: Aabb, yaw: f32, pitch: f32, aspect: f32) -> Self {
        let target = bounds.center();
        let radius = (bounds.extent().length() * 0.5).max(1e-4);
        let tan = (FOV * 0.5).tan();
        let fit = if aspect < 1.0 { (tan * aspect).atan() } else { FOV * 0.5 };
        let distance = radius / fit.sin() * 1.05;
        let limit = std::f32::consts::FRAC_PI_2 - 1e-3;
        let pitch = pitch.clamp(-limit, limit);
        let eye = target + Vec3::new(pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos()) * distance;
        let fwd = (target - eye).normalize();
        let right = fwd.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
        let up = right.cross(fwd);
        Camera { eye, right, up, fwd, tan, aspect }
    }

    /// Screen x, y in pixels and view depth.
    fn project(&self, p: Vec3, w: f32, h: f32) -> [f32; 3] {
        let q = p - self.eye;
        let z = q.dot(self.fwd).max(1e-6);
        let x = q.dot(self.right) / (z * self.tan * self.aspect);
        let y = q.dot(self.up) / (z * self.tan);
        [(x * 0.5 + 0.5) * w, (0.5 - y * 0.5) * h, z]
    }

    /// A world direction in view space, +z toward the viewer (as the viewport's shader sees it).
    fn to_view(&self, d: Vec3) -> Vec3 {
        Vec3::new(d.dot(self.right), d.dot(self.up), -d.dot(self.fwd))
    }
}

#[derive(Clone, Copy)]
struct Sample {
    depth: f32,
    object: u32,
    triangle: u32,
    bary: [f32; 3],
}

const EMPTY: Sample = Sample { depth: f32::INFINITY, object: u32::MAX, triangle: 0, bary: [0.0; 3] };

fn raster_band(band: &mut [Sample], y0: usize, w: usize, projected: &[Vec<[f32; 3]>], objects: &[RenderObject]) {
    let y1 = y0 + band.len() / w;
    for (oi, (o, pts)) in objects.iter().zip(projected).enumerate() {
        let hid = o.mesh.has_hidden();
        for (ti, t) in o.mesh.indices.chunks_exact(3).enumerate() {
            if hid && o.mesh.hidden[ti] {
                continue;
            }
            let (a, b, c) = (pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]);
            let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            if area.abs() < 1e-12 {
                continue;
            }
            let min_y = a[1].min(b[1]).min(c[1]).floor().max(y0 as f32) as usize;
            let max_y = (a[1].max(b[1]).max(c[1]).ceil() as isize).min(y1 as isize - 1);
            if max_y < min_y as isize {
                continue;
            }
            let min_x = a[0].min(b[0]).min(c[0]).floor().max(0.0) as usize;
            let max_x = (a[0].max(b[0]).max(c[0]).ceil() as isize).min(w as isize - 1);
            if max_x < min_x as isize {
                continue;
            }
            let inv = 1.0 / area;
            let (iza, izb, izc) = (1.0 / a[2], 1.0 / b[2], 1.0 / c[2]);
            for y in min_y..=max_y as usize {
                let py = y as f32 + 0.5;
                let row = (y - y0) * w;
                for x in min_x..=max_x as usize {
                    let px = x as f32 + 0.5;
                    let w0 = ((b[0] - px) * (c[1] - py) - (b[1] - py) * (c[0] - px)) * inv;
                    let w1 = ((c[0] - px) * (a[1] - py) - (c[1] - py) * (a[0] - px)) * inv;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let iz = w0 * iza + w1 * izb + w2 * izc;
                    let depth = 1.0 / iz;
                    let s = &mut band[row + x];
                    if depth < s.depth {
                        let k = depth;
                        *s = Sample { depth, object: oi as u32, triangle: ti as u32, bary: [w0 * iza * k, w1 * izb * k, w2 * izc * k] };
                    }
                }
            }
        }
    }
}

fn shade(cam: &Camera, s: &Sample, objects: &[RenderObject], shading_in: Shading) -> Option<Vec3> {
    if s.object == u32::MAX {
        return None;
    }
    let o = &objects[s.object as usize];
    let t = &o.mesh.indices[s.triangle as usize * 3..s.triangle as usize * 3 + 3];
    let mut n = Vec3::ZERO;
    let mut p = Vec3::ZERO;
    let mut paint = [0.0f32; fsc_core::PAINT_LEN];
    let mut mask = 0.0;
    for (k, &v) in t.iter().enumerate() {
        let v = v as usize;
        let w = s.bary[k];
        n += Vec3::from(o.mesh.normals[v]) * w;
        p += Vec3::from(o.mesh.positions[v]) * w;
        if let Some(c) = o.mesh.paint.get(v) {
            for (i, x) in c.iter().enumerate() {
                paint[i] += x * w;
            }
        }
        mask += o.mask.get(v).copied().unwrap_or(0.0) * w;
    }
    let mut n = cam.to_view(n.normalize_or(-cam.fwd));
    let v = cam.to_view((cam.eye - p).normalize_or(-cam.fwd));
    if n.dot(v) < 0.0 {
        n = -n;
    }
    let painted = !o.mesh.paint.is_empty();
    let shading = match shading_in {
        Shading::Groups => Shading::named("clay").expect("clay exists"),
        other => other,
    };
    let albedo = if matches!(shading_in, Shading::Groups) && o.mesh.has_groups() {
        group_color(o.mesh.groups[s.triangle as usize])
    } else if painted {
        Vec3::new(paint[0], paint[1], paint[2])
    } else if o.active {
        Vec3::new(0.86, 0.74, 0.66)
    } else {
        Vec3::new(0.72, 0.7, 0.68)
    };
    let mut col = match shading {
        Shading::Matcap { key, spec, sharp, rim, tint } => {
            let tint = Vec3::from(tint);
            let r = (-v) - 2.0 * (-v).dot(n) * n;
            let m = Vec3::new(r.x, r.y, 0.0) / (2.0 * (r.x * r.x + r.y * r.y + (r.z + 1.0) * (r.z + 1.0)).sqrt().max(1e-6)) + Vec3::new(0.5, 0.5, 0.0);
            let k = n.dot(KEY.normalize()).clamp(0.0, 1.0);
            let fill = n.dot(FILL.normalize()).clamp(0.0, 1.0) * 0.25;
            let rim = (1.0 - n.dot(v).clamp(0.0, 1.0)).powi(3) * rim;
            let sp = (1.0 - Vec3::new(m.x - 0.34, m.y - 0.68, 0.0).length() * 2.2).clamp(0.0, 1.0).powf(sharp) * spec;
            let t = ((n.y * 0.5 + 0.5 + 0.2) / 1.1).clamp(0.0, 1.0);
            let cavity = 0.55 + 0.45 * t * t * (3.0 - 2.0 * t);
            let base = albedo * tint;
            let c = base * (0.18 + k * key + fill) * cavity + (Vec3::splat(rim) + sp * Vec3::ONE.lerp(tint, 0.6));
            c.max(Vec3::ZERO).powf(1.0 / 1.1)
        }
        // Groups was turned into the clay matcap above; it is listed only to keep the match exhaustive.
        Shading::Pbr | Shading::Groups => {
            let rough = paint_or(painted, paint[3], 0.5).clamp(0.05, 1.0);
            let metal = paint_or(painted, paint[4], 0.0).clamp(0.0, 1.0);
            let mut c = light(n, v, KEY.normalize(), Vec3::new(2.6, 2.45, 2.25), albedo, rough, metal);
            c += light(n, v, FILL.normalize(), Vec3::new(0.55, 0.62, 0.75), albedo, rough, metal);
            c += light(n, v, Vec3::new(0.0, 0.35, -1.0).normalize(), Vec3::splat(0.9), albedo, rough, metal);
            let sky = Vec3::new(0.16, 0.15, 0.14).lerp(Vec3::new(0.42, 0.45, 0.5), n.y * 0.5 + 0.5);
            let f0 = Vec3::splat(0.04).lerp(albedo, metal);
            c += sky * (albedo * (1.0 - metal) + f0 * (1.0 - rough * 0.7));
            (c / (c + Vec3::ONE)).powf(1.0 / 2.2)
        }
    };
    // Emissive paint glows in its own colour whatever the light.
    if painted && paint[6] > 0.0 {
        col = col.lerp(albedo.powf(1.0 / 2.2) * 1.15, paint[6].clamp(0.0, 1.0));
    }
    col = col.lerp(col * Vec3::new(0.32, 0.34, 0.4), mask.clamp(0.0, 1.0) * 0.85);
    Some(col)
}

fn paint_or(painted: bool, v: f32, default: f32) -> f32 {
    if painted {
        v
    } else {
        default
    }
}

fn light(n: Vec3, v: Vec3, l: Vec3, radiance: Vec3, albedo: Vec3, rough: f32, metal: f32) -> Vec3 {
    let h = (l + v).normalize();
    let ndl = n.dot(l).max(0.0);
    let ndv = n.dot(v).max(1e-3);
    let ndh = n.dot(h).max(0.0);
    let a2 = (rough * rough).powi(2);
    let d = a2 / (std::f32::consts::PI * (ndh * ndh * (a2 - 1.0) + 1.0).powi(2));
    let k = (rough + 1.0).powi(2) / 8.0;
    let g = (ndl / (ndl * (1.0 - k) + k)) * (ndv / (ndv * (1.0 - k) + k));
    let f0 = Vec3::splat(0.04).lerp(albedo, metal);
    let f = f0 + (Vec3::ONE - f0) * (1.0 - h.dot(v).max(0.0)).powi(5);
    let spec = d * g * f / (4.0 * ndl * ndv + 1e-4);
    let kd = (Vec3::ONE - f) * (1.0 - metal);
    (kd * albedo / std::f32::consts::PI + spec) * radiance * ndl
}

/// Where points land in a get_view picture of `frame` (all drawn objects' bounds by default): x from 0 at the left to
/// 1 at the right, y from 0 at the top to 1 at the bottom, for a picture of the given aspect (width over height).
pub fn view_coordinates(frame: Aabb, yaw: f32, pitch: f32, aspect: f32, points: &[[f32; 3]]) -> Vec<[f32; 2]> {
    let cam = Camera::new(frame, yaw, pitch, aspect);
    points.iter().map(|&p| { let s = cam.project(Vec3::from(p), 1.0, 1.0); [s[0], s[1]] }).collect()
}

/// Draws the objects with a z-buffer on the CPU, shaded as the window shades them. No GPU or window is needed, so an
/// agent can look at the model with the app closed.
pub fn render(objects: &[RenderObject], view: &View) -> Result<Rendered, String> {
    let started = Instant::now();
    if view.width == 0 || view.height == 0 || view.width > 4096 || view.height > 4096 {
        return Err("width and height must be 1 to 4096".into());
    }
    let mut bounds = Aabb::EMPTY;
    if let Some(f) = view.frame {
        bounds = f;
    } else {
        for o in objects {
            for &p in &o.mesh.positions {
                bounds.grow(Vec3::from(p));
            }
        }
    }
    if bounds.min.x > bounds.max.x {
        bounds = Aabb { min: Vec3::splat(-1.0), max: Vec3::splat(1.0) };
    }
    let ss = view.supersample.clamp(1, 3);
    let (w, h) = (view.width * ss, view.height * ss);
    let cam = Camera::new(bounds, view.yaw, view.pitch, view.width as f32 / view.height as f32);
    let project = |o: &RenderObject| -> Vec<[f32; 3]> { o.mesh.positions.iter().map(|&p| cam.project(Vec3::from(p), w as f32, h as f32)).collect() };
    #[cfg(feature = "parallel")]
    let projected: Vec<Vec<[f32; 3]>> = objects.par_iter().map(project).collect();
    #[cfg(not(feature = "parallel"))]
    let projected: Vec<Vec<[f32; 3]>> = objects.iter().map(project).collect();

    let mut buffer = vec![EMPTY; w * h];
    let rows = 16usize.max(h / 64);
    #[cfg(feature = "parallel")]
    buffer.par_chunks_mut(rows * w).enumerate().for_each(|(i, band)| raster_band(band, i * rows, w, &projected, objects));
    #[cfg(not(feature = "parallel"))]
    buffer.chunks_mut(rows * w).enumerate().for_each(|(i, band)| raster_band(band, i * rows, w, &projected, objects));

    let out_px = |i: usize| -> [u8; 4] {
        let (x, y) = (i % view.width, i / view.width);
        let mut rgb = Vec3::ZERO;
        let mut alpha = 0.0;
        for sy in 0..ss {
            for sx in 0..ss {
                let s = &buffer[(y * ss + sy) * w + x * ss + sx];
                match shade(&cam, s, objects, view.shading) {
                    Some(c) => {
                        rgb += c.clamp(Vec3::ZERO, Vec3::ONE);
                        alpha += 1.0;
                    }
                    None => {
                        if let Some(bg) = view.background {
                            rgb += Vec3::from(bg);
                        }
                    }
                }
            }
        }
        let n = (ss * ss) as f32;
        let a = if view.background.is_some() { 1.0 } else { alpha / n };
        let c = if view.background.is_some() { rgb / n } else if alpha > 0.0 { rgb / alpha } else { Vec3::ZERO };
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        [q(c.x), q(c.y), q(c.z), q(a)]
    };
    #[cfg(feature = "parallel")]
    let pixels: Vec<[u8; 4]> = (0..view.width * view.height).into_par_iter().map(out_px).collect();
    #[cfg(not(feature = "parallel"))]
    let pixels: Vec<[u8; 4]> = (0..view.width * view.height).map(out_px).collect();
    let covered = buffer.iter().filter(|s| s.object != u32::MAX).count() as f32 / buffer.len() as f32;
    let triangles = objects.iter().map(|o| o.mesh.triangle_count()).sum();
    Ok(Rendered { width: view.width, height: view.height, pixels, triangles, covered, ms: started.elapsed().as_millis() })
}
