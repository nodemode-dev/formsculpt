//! The head primitive: a simple base head on a quad sphere, with a second form for a layer that blends between a
//! masculine and a feminine head (as Nomad Sculpt's Head primitive is described). Each form is a smooth union of simple
//! shapes (cranium, face, jaw, chin, nose, brow, cheekbones, ears and neck, with eye sockets and a mouth line carved
//! out), and the quad sphere's points are moved along their direction from a centre point onto the last place each ray
//! leaves that surface. Both forms use the same rays, so their points correspond and the difference is a layer.
//! The face looks along +z; the head is about 2.4 tall with its neck, centred near the origin.

use glam::Vec3;

use crate::Mesh;

/// The settings that differ between the two forms.
#[derive(Clone, Copy)]
struct Form {
    cranium: Vec3,
    jaw_width: f32,
    jaw_drop: f32,
    chin: f32,
    nose: f32,
    brow: f32,
    cheek: f32,
    neck: f32,
    lips: f32,
}

const MASCULINE: Form = Form { cranium: Vec3::new(0.74, 0.84, 0.92), jaw_width: 0.6, jaw_drop: 0.0, chin: 0.2, nose: 0.1, brow: 0.11, cheek: 0.19, neck: 0.4, lips: 0.04 };
const FEMININE: Form = Form { cranium: Vec3::new(0.71, 0.84, 0.9), jaw_width: 0.5, jaw_drop: 0.05, chin: 0.15, nose: 0.075, brow: 0.06, cheek: 0.21, neck: 0.31, lips: 0.055 };

fn ellipsoid(p: Vec3, c: Vec3, r: Vec3) -> f32 {
    let q = (p - c) / r;
    (q.length() - 1.0) * r.min_element()
}

fn sphere(p: Vec3, c: Vec3, r: f32) -> f32 {
    (p - c).length() - r
}

fn capsule(p: Vec3, a: Vec3, b: Vec3, r: f32) -> f32 {
    let (pa, ba) = (p - a, b - a);
    let h = (pa.dot(ba) / ba.length_squared()).clamp(0.0, 1.0);
    (pa - ba * h).length() - r
}

/// An upright cylinder centred at `c`, radius `r`, half height `h`, its rims rounded by `round`.
fn rounded_cylinder(p: Vec3, c: Vec3, r: f32, h: f32, round: f32) -> f32 {
    let q = p - c;
    let dx = Vec3::new(q.x, 0.0, q.z).length() - r + round;
    let dy = q.y.abs() - h + round;
    dx.max(dy).min(0.0) + Vec3::new(dx.max(0.0), dy.max(0.0), 0.0).length() - round
}

/// Smooth union over a blend width `k` (the polynomial smooth minimum).
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

/// Smooth subtraction of `b` from `a`.
fn ssub(a: f32, b: f32, k: f32) -> f32 {
    -smin(-a, b, k)
}

/// Negative inside the head, positive outside.
fn field(p: Vec3, f: &Form) -> f32 {
    // Symmetric about x = 0: features on one side are mirrored by folding x.
    let m = Vec3::new(p.x.abs(), p.y, p.z);
    let mut d = ellipsoid(p, Vec3::new(0.0, 0.18, -0.05), f.cranium);
    // The face mass and the jaw under it.
    d = smin(d, ellipsoid(p, Vec3::new(0.0, -0.3, 0.22), Vec3::new(0.56, 0.55, 0.58)), 0.25);
    d = smin(d, capsule(m, Vec3::new(f.jaw_width * 0.8, -0.45 - f.jaw_drop, -0.05), Vec3::new(0.12, -0.78 - f.jaw_drop, 0.42), 0.14), 0.2);
    d = smin(d, sphere(p, Vec3::new(0.0, -0.82 - f.jaw_drop, 0.47), f.chin), 0.18);
    // Brow ridge, cheekbones, nose, lips.
    d = smin(d, capsule(m, Vec3::new(0.04, 0.24, 0.74), Vec3::new(0.38, 0.22, 0.66), f.brow), 0.12);
    d = smin(d, sphere(m, Vec3::new(0.42, -0.08, 0.52), f.cheek), 0.2);
    d = smin(d, capsule(p, Vec3::new(0.0, 0.12, 0.8), Vec3::new(0.0, -0.22, 0.97), f.nose), 0.1);
    d = smin(d, sphere(m, Vec3::new(0.07, -0.28, 0.95), f.nose * 0.9), 0.08);
    d = smin(d, capsule(m, Vec3::new(0.0, -0.47, 0.8), Vec3::new(0.15, -0.49, 0.74), f.lips), 0.1);
    d = smin(d, capsule(m, Vec3::new(0.0, -0.58, 0.78), Vec3::new(0.12, -0.58, 0.73), f.lips * 0.95), 0.1);
    // Ears (thick enough that the rays sample their rim cleanly), and the neck, rounded at its end: a flat cut met by
    // grazing rays left a ragged edge.
    d = smin(d, ellipsoid(m, Vec3::new(0.75, -0.05, -0.08), Vec3::new(0.1, 0.22, 0.14)), 0.1);
    d = smin(d, rounded_cylinder(p, Vec3::new(0.0, -0.95, -0.16), f.neck, 0.45, 0.15), 0.22);
    // Eye sockets and the line between the lips, carved out.
    d = ssub(d, sphere(m, Vec3::new(0.27, 0.07, 0.86), 0.13), 0.08);
    d = ssub(d, capsule(m, Vec3::new(0.0, -0.525, 0.86), Vec3::new(0.15, -0.53, 0.78), 0.012), 0.03);
    d
}

/// The last point where the ray from `c` along `d` leaves the surface (so ears and the nose are reached), found by
/// stepping out and then halving.
fn reach(c: Vec3, d: Vec3, f: &Form) -> Vec3 {
    const STEPS: usize = 160;
    const FAR: f32 = 2.6;
    let at = |t: f32| field(c + d * t, f);
    let mut last = None;
    let mut prev = at(0.0);
    for i in 1..=STEPS {
        let t = FAR * i as f32 / STEPS as f32;
        let v = at(t);
        if prev < 0.0 && v >= 0.0 {
            last = Some((FAR * (i - 1) as f32 / STEPS as f32, t));
        }
        prev = v;
    }
    let (mut lo, mut hi) = last.unwrap_or((0.0, 0.05));
    for _ in 0..24 {
        let mid = (lo + hi) * 0.5;
        if at(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    c + d * ((lo + hi) * 0.5)
}

/// The field's gradient at `p`, by central differences.
fn gradient(p: Vec3, f: &Form) -> Vec3 {
    const H: f32 = 1e-3;
    Vec3::new(
        field(p + Vec3::X * H, f) - field(p - Vec3::X * H, f),
        field(p + Vec3::Y * H, f) - field(p - Vec3::Y * H, f),
        field(p + Vec3::Z * H, f) - field(p - Vec3::Z * H, f),
    ) / (2.0 * H)
}

fn normal(p: Vec3, f: &Form) -> Vec3 {
    gradient(p, f).normalize_or(Vec3::Y)
}

/// Moves `p` onto the surface along the field's gradient (a few Newton steps; the field is close to a distance).
fn settle(mut p: Vec3, f: &Form) -> Vec3 {
    for _ in 0..6 {
        let v = field(p, f);
        let g = gradient(p, f);
        let gg = g.length_squared();
        if gg < 1e-8 || v.abs() < 1e-6 {
            break;
        }
        p -= g * (v / gg);
    }
    p
}

/// Below the jaw the rays from the centre meet the neck's end at a grazing angle, so its points bunch on one side of
/// the rounded rim and thin out on the other and the rim comes out uneven. There each point is pulled toward the
/// average of its neighbours and put back on the surface, a few times over, which spreads them evenly; the pull
/// fades in over the top of the neck so the face is untouched.
fn even_neck(points: &mut [Vec3], neighbours: &[Vec<u32>], f: &Form) {
    const TOP: f32 = -1.05;
    const FULL: f32 = -1.2;
    let weight = |y: f32| ((TOP - y) / (TOP - FULL)).clamp(0.0, 1.0);
    for _ in 0..60 {
        let before = points.to_vec();
        for (i, p) in points.iter_mut().enumerate() {
            let w = weight(before[i].y);
            if w <= 0.0 || neighbours[i].is_empty() {
                continue;
            }
            let mean = neighbours[i].iter().map(|&n| before[n as usize]).sum::<Vec3>() / neighbours[i].len() as f32;
            // Only the pull along the surface: the part across it would draw points off the curved rim.
            let n = normal(before[i], f);
            let pull = mean - before[i];
            *p = settle(before[i] + (pull - n * pull.dot(n)) * (0.8 * w), f);
        }
    }
}

/// The masculine head on a quad sphere of `divisions` quads a side (scaled by `size`) and, per point, how far each
/// moves to give the feminine head.
pub fn head(size: f32, divisions: u32) -> (Mesh, Vec<[f32; 3]>) {
    let mut mesh = crate::primitives::quad_sphere(1.0, divisions.clamp(4, 256));
    let centre = Vec3::new(0.0, -0.15, 0.05);
    let rays: Vec<Vec3> = mesh.positions.iter().map(|p| Vec3::from_array(*p).normalize_or(Vec3::Y)).collect();
    let mut a: Vec<Vec3> = rays.iter().map(|&d| reach(centre, d, &MASCULINE)).collect();
    let mut b: Vec<Vec3> = rays.iter().map(|&d| reach(centre, d, &FEMININE)).collect();
    // Both forms are evened over the same mesh, so their points still pair up for the blend.
    let mut neighbours = vec![Vec::new(); a.len()];
    for t in mesh.indices.chunks(3) {
        for k in 0..3 {
            let (u, v) = (t[k], t[(k + 1) % 3]);
            if !neighbours[u as usize].contains(&v) {
                neighbours[u as usize].push(v);
                neighbours[v as usize].push(u);
            }
        }
    }
    even_neck(&mut a, &neighbours, &MASCULINE);
    even_neck(&mut b, &neighbours, &FEMININE);
    let mut delta = Vec::with_capacity(a.len());
    for (i, p) in mesh.positions.iter_mut().enumerate() {
        *p = (a[i] * size).to_array();
        delta.push(((b[i] - a[i]) * size).to_array());
    }
    mesh.recompute_normals();
    (mesh, delta)
}
