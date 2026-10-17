use fsc_core::time::Instant;

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

/// A small, fast random stream (xorshift32) per pixel, so a path-traced picture is the same every time.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// A direction about `n` with cosine-weighted density.
fn cosine_dir(n: Vec3, rng: &mut Rng) -> Vec3 {
    let (u, v) = (rng.next(), rng.next());
    let r = u.sqrt();
    let phi = std::f32::consts::TAU * v;
    let t = n.any_orthonormal_vector();
    let b = n.cross(t);
    (t * (r * phi.cos()) + b * (r * phi.sin()) + n * (1.0 - u).max(0.0).sqrt()).normalize()
}

/// The surface at a hit: position, shading normal, and paint (colour, roughness, metalness, opacity, emissive).
fn surface(o: &RenderObject, h: &fsc_core::Hit) -> (Vec3, Vec3, [f32; fsc_core::PAINT_LEN]) {
    let [a, b, c] = o.mesh.triangle(h.triangle as usize);
    let w = [1.0 - h.u - h.v, h.u, h.v];
    let n: Vec3 = [a, b, c].iter().zip(w).map(|(&v, w)| Vec3::from(o.mesh.normals[v as usize]) * w).sum();
    let paint = if o.mesh.has_paint() {
        std::array::from_fn(|k| o.mesh.paint[a as usize][k] * w[0] + o.mesh.paint[b as usize][k] * w[1] + o.mesh.paint[c as usize][k] * w[2])
    } else {
        let mut p = fsc_core::DEFAULT_PAINT;
        p[..3].copy_from_slice(&if o.active { [0.86, 0.74, 0.66] } else { [0.72, 0.7, 0.68] });
        p
    };
    (h.point, n.normalize_or(Vec3::Y), paint)
}

/// A path-traced picture of the objects from the same camera as [`render`]: global illumination from a sky and a sun
/// (the window's key light) with soft shadows, diffuse and glossy bounces by paint roughness and metalness, painted
/// glow as a light, and painted opacity letting light through. `samples` paths per pixel, `bounces` deep.
/// What a path render writes: the lit image, or one of the compositing passes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pass {
    Beauty,
    /// Camera distance, near white to far black across the framed bounds.
    Depth,
    /// World normal as colour (n * 0.5 + 0.5).
    Normal,
    /// Ambient occlusion: the share of the hemisphere open within a fifth of the bounds.
    Occlusion,
}

/// A directional light added to the sun: the direction it shines from (world space) and its colour times intensity.
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub from: Vec3,
    pub color: Vec3,
}

#[derive(Clone, Debug)]
pub struct PathOptions {
    pub samples: u32,
    pub bounces: u32,
    pub lights: Vec<Light>,
    /// Keep the window's key light as the sun.
    pub sun: bool,
    /// Lens radius as a fraction of the framed size; 0 is a pinhole (everything sharp).
    pub aperture: f32,
    /// Distance from the camera that is sharp; the framed centre when None.
    pub focus: Option<f32>,
    pub pass: Pass,
    /// How the beauty pass's light is brought to the display, and the grade before it (passes are left linear).
    pub tone: crate::Tone,
    pub grade: crate::Grade,
}

impl PathOptions {
    pub fn new(samples: u32, bounces: u32) -> Self {
        PathOptions { samples, bounces, lights: Vec::new(), sun: true, aperture: 0.0, focus: None, pass: Pass::Beauty, tone: crate::Tone::Reinhard, grade: crate::Grade::default() }
    }
}

pub fn path_trace(objects: &[RenderObject], view: &View, samples: u32, bounces: u32) -> Result<Rendered, String> {
    path_trace_with(objects, view, &PathOptions::new(samples, bounces))
}

pub fn path_trace_with(objects: &[RenderObject], view: &View, options: &PathOptions) -> Result<Rendered, String> {
    let (samples, bounces) = (options.samples, options.bounces);
    let started = Instant::now();
    if view.width == 0 || view.height == 0 || view.width > 2048 || view.height > 2048 {
        return Err("width and height must be 1 to 2048".into());
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
    let cam = Camera::new(bounds, view.yaw, view.pitch, view.width as f32 / view.height as f32);
    let bvhs: Vec<fsc_core::Bvh> = objects.iter().map(|o| fsc_core::Bvh::build(o.mesh)).collect();
    let eps = bounds.extent().length().max(1e-3) * 1e-4;
    let hit = |p: Vec3, d: Vec3| -> Option<(usize, fsc_core::Hit)> {
        let mut best: Option<(usize, fsc_core::Hit)> = None;
        for (i, (o, b)) in objects.iter().zip(&bvhs).enumerate() {
            if let Some(h) = b.raycast(o.mesh, p, d) {
                if h.t > eps && best.as_ref().is_none_or(|(_, x)| h.t < x.t) {
                    best = Some((i, h));
                }
            }
        }
        best
    };
    // The window's key light as a sun of small angular size, in world space (the key follows the camera there).
    let sun_dir = (cam.right * KEY.x + cam.up * KEY.y - cam.fwd * KEY.z).normalize();
    let sun = Vec3::new(2.6, 2.45, 2.25);
    let sky = |d: Vec3| Vec3::new(0.16, 0.15, 0.14).lerp(Vec3::new(0.42, 0.45, 0.5), d.y * 0.5 + 0.5) * 1.2;
    let (samples, bounces) = (samples.clamp(1, 1024), bounces.clamp(1, 8));
    let size = bounds.extent().length().max(1e-3);
    let lens = options.aperture.clamp(0.0, 1.0) * size * 0.5;
    let focus = options.focus.unwrap_or_else(|| (bounds.center() - cam.eye).dot(cam.fwd)).max(1e-4);
    let mut lights: Vec<(Vec3, Vec3)> = options.lights.iter().filter_map(|l| Some((l.from.try_normalize()?, l.color.max(Vec3::ZERO)))).collect();
    if options.sun {
        lights.push((sun_dir, sun));
    }
    let near = (bounds.center() - cam.eye).length() - size * 0.5;
    let reach = size * 0.2;
    let trace = |i: usize| -> [u8; 4] {
        let (x, y) = (i % view.width, i / view.width);
        let mut rng = Rng((i as u32).wrapping_mul(2_654_435_761) ^ 0x9e37_79b9 | 1);
        let mut total = Vec3::ZERO;
        let mut alpha = 0.0f32;
        for _ in 0..samples {
            let sx = ((x as f32 + rng.next()) / view.width as f32) * 2.0 - 1.0;
            let sy = 1.0 - ((y as f32 + rng.next()) / view.height as f32) * 2.0;
            let mut d = (cam.fwd + cam.right * (sx * cam.tan * cam.aspect) + cam.up * (sy * cam.tan)).normalize();
            let mut p = cam.eye;
            if lens > 0.0 {
                // Thin lens: start from a point on the aperture disc, aimed at where the pinhole ray meets the focal plane.
                let focal = cam.eye + d * (focus / d.dot(cam.fwd));
                let (r, a) = (rng.next().sqrt() * lens, rng.next() * std::f32::consts::TAU);
                p = cam.eye + cam.right * (r * a.cos()) + cam.up * (r * a.sin());
                d = (focal - p).normalize();
            }
            if options.pass != Pass::Beauty {
                if let Some((oi, h)) = hit(p, d) {
                    alpha += 1.0;
                    let (q, mut n, _) = surface(&objects[oi], &h);
                    total += match options.pass {
                        Pass::Depth => Vec3::splat(1.0 - ((q - cam.eye).length() - near) / size),
                        Pass::Normal => n * 0.5 + Vec3::splat(0.5),
                        _ => {
                            if n.dot(d) > 0.0 {
                                n = -n;
                            }
                            let ray = cosine_dir(n, &mut rng);
                            let open = hit(q + n * eps * 4.0, ray).is_none_or(|(_, o)| o.t > reach);
                            Vec3::splat(if open { 1.0 } else { 0.0 })
                        }
                    }
                    .clamp(Vec3::ZERO, Vec3::ONE);
                } else if let Some(bg) = view.background {
                    total += Vec3::from(bg);
                }
                continue;
            }
            let mut through = Vec3::ONE;
            let mut light = Vec3::ZERO;
            let mut first = true;
            for _ in 0..bounces {
                let Some((oi, h)) = hit(p, d) else {
                    if first {
                        light += match view.background {
                            Some(bg) => Vec3::from(bg).powf(2.2),
                            None => Vec3::ZERO,
                        };
                    } else {
                        light += through * sky(d);
                    }
                    break;
                };
                if first {
                    alpha += 1.0;
                    first = false;
                }
                let (q, mut n, paint) = surface(&objects[oi], &h);
                if n.dot(d) > 0.0 {
                    n = -n;
                }
                let albedo = Vec3::new(paint[0], paint[1], paint[2]).clamp(Vec3::ZERO, Vec3::ONE);
                let (rough, metal, opacity, glow) = (paint[3].clamp(0.05, 1.0), paint[4].clamp(0.0, 1.0), paint[5].clamp(0.0, 1.0), paint[6].clamp(0.0, 1.0));
                if glow > 0.0 {
                    light += through * albedo * glow * 4.0;
                }
                // See-through paint: the path continues straight on with the chance it is not opaque.
                if rng.next() > opacity {
                    p = q + d * eps * 4.0;
                    continue;
                }
                let origin = q + n * eps * 4.0;
                // The sun and any added lights, each sampled directly with a soft shadow ray.
                for &(from, color) in &lights {
                    let s = (from + Vec3::new(rng.next() - 0.5, rng.next() - 0.5, rng.next() - 0.5) * 0.06).normalize();
                    let ndl = n.dot(s);
                    if ndl > 0.0 && hit(origin, s).is_none() {
                        let half = (s - d).normalize();
                        let spec = n.dot(half).max(0.0).powf(2.0 / (rough * rough * rough * rough).max(1e-4)) * (1.0 - rough) * 0.5;
                        let diffuse = albedo * (1.0 - metal) / std::f32::consts::PI;
                        let f0 = Vec3::splat(0.04).lerp(albedo, metal);
                        light += through * color * ndl * (diffuse + f0 * spec);
                    }
                }
                // Next bounce: glossy reflection by metalness (and smoothness), otherwise diffuse.
                let glossy = metal.max((1.0 - rough) * 0.25);
                if rng.next() < glossy {
                    let r = d - n * 2.0 * d.dot(n);
                    d = (r + cosine_dir(n, &mut rng) * rough * rough).normalize();
                    if d.dot(n) <= 0.0 {
                        break;
                    }
                    through *= Vec3::splat(0.04).lerp(albedo, metal).max(Vec3::splat(0.04)) / glossy.max(0.04) * glossy;
                } else {
                    d = cosine_dir(n, &mut rng);
                    through *= albedo * (1.0 - metal);
                }
                p = origin;
                if through.max_element() < 0.01 {
                    break;
                }
            }
            total += light;
        }
        let c = total / samples as f32;
        let c = if options.pass == Pass::Beauty { crate::tone::display(c, options.tone, &options.grade) } else { c };
        let a = if view.background.is_some() { 1.0 } else { alpha / samples as f32 };
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        [q(c.x), q(c.y), q(c.z), q(a)]
    };
    #[cfg(feature = "parallel")]
    let pixels: Vec<[u8; 4]> = (0..view.width * view.height).into_par_iter().map(trace).collect();
    #[cfg(not(feature = "parallel"))]
    let pixels: Vec<[u8; 4]> = (0..view.width * view.height).map(trace).collect();
    let covered = pixels.iter().filter(|p| p[3] > 0).count() as f32 / pixels.len() as f32;
    let triangles = objects.iter().map(|o| o.mesh.triangle_count()).sum();
    Ok(Rendered { width: view.width, height: view.height, pixels, triangles, covered, ms: started.elapsed().as_millis() })
}
