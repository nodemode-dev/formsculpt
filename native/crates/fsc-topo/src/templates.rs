//! Whole-object retopology templates: quad cages that wrap an entire sculpt rather than a patch of it. A template is
//! built in a unit frame (x right, y up, z toward the viewer), fitted to the target's extent along the view's axes,
//! shrink-wrapped onto its outer surface by rays from its centre, then evened out by relaxing along the surface.

use std::collections::{HashMap, HashSet};

use fsc_core::{Bvh, Mesh};
use glam::Vec3;

use crate::nearest::nearest_point;
use crate::zmodel::{inset_face, PolyHit};

/// The template kinds, as named to agents and in the window.
pub const WRAP_KINDS: &[&str] = &["head", "sphere"];

/// A template laid on a target: points on its surface, the surface normal under each, and quads wound outward.
#[derive(Debug, Clone)]
pub struct Wrapped {
    pub points: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub quads: Vec<[u32; 4]>,
    /// Quads that came out facing into the surface (folded where the target is not star-shaped from its centre).
    pub folded: usize,
}

/// A cube's six faces split into `n` by `n` quads, pushed out onto the unit sphere: all quads, eight three-edge poles.
fn cube_sphere(n: usize) -> (Vec<Vec3>, Vec<[u32; 4]>) {
    let mut points: Vec<Vec3> = Vec::new();
    let mut index: HashMap<(i32, i32, i32), u32> = HashMap::new();
    let mut quads = Vec::new();
    let axes = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
    for normal in axes {
        let u = if normal.x.abs() > 0.5 { Vec3::Z } else { Vec3::X };
        let v = normal.cross(u);
        let mut id = |i: usize, j: usize| -> u32 {
            let s = -1.0 + 2.0 * i as f32 / n as f32;
            let t = -1.0 + 2.0 * j as f32 / n as f32;
            let p = normal + u * s + v * t;
            let k = ((p.x * 1000.0).round() as i32, (p.y * 1000.0).round() as i32, (p.z * 1000.0).round() as i32);
            *index.entry(k).or_insert_with(|| {
                points.push(p.normalize());
                points.len() as u32 - 1
            })
        };
        for j in 0..n {
            for i in 0..n {
                let q = [id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)];
                quads.push(q);
            }
        }
    }
    // Wind every quad outward.
    for q in quads.iter_mut() {
        let p = q.map(|k| points[k as usize]);
        if (p[2] - p[0]).cross(p[3] - p[1]).dot(p[0] + p[1] + p[2] + p[3]) < 0.0 {
            q.reverse();
        }
    }
    (points, quads)
}

/// Every quad split in four through its edge midpoints and centre, the new points pushed onto the unit sphere;
/// winding is kept.
fn split_on_sphere(points: &[Vec3], quads: &[[u32; 4]]) -> (Vec<Vec3>, Vec<[u32; 4]>) {
    let mut out_points = points.to_vec();
    let mut mids: HashMap<(u32, u32), u32> = HashMap::new();
    let mut out = Vec::with_capacity(quads.len() * 4);
    for q in quads {
        let mut mid = |a: u32, b: u32, pts: &mut Vec<Vec3>| -> u32 {
            *mids.entry((a.min(b), a.max(b))).or_insert_with(|| {
                pts.push((points[a as usize] + points[b as usize]).normalize_or(Vec3::Z));
                pts.len() as u32 - 1
            })
        };
        let m01 = mid(q[0], q[1], &mut out_points);
        let m12 = mid(q[1], q[2], &mut out_points);
        let m23 = mid(q[2], q[3], &mut out_points);
        let m30 = mid(q[3], q[0], &mut out_points);
        out_points.push(q.iter().map(|&k| points[k as usize]).sum::<Vec3>().normalize_or(Vec3::Z));
        let c = out_points.len() as u32 - 1;
        out.push([q[0], m01, c, m30]);
        out.push([m01, q[1], m12, c]);
        out.push([c, m12, q[2], m23]);
        out.push([m30, c, m23, q[3]]);
    }
    (out_points, out)
}

/// The quad whose centre points most nearly along `dir`.
fn quad_toward(points: &[Vec3], quads: &[[u32; 4]], dir: Vec3) -> usize {
    let d = dir.normalize();
    (0..quads.len())
        .max_by(|&a, &b| {
            let c = |q: usize| quads[q].iter().map(|&k| points[k as usize]).sum::<Vec3>().normalize_or_zero().dot(d);
            c(a).total_cmp(&c(b))
        })
        .unwrap_or(0)
}

/// Rings of quads around one face: the face inset `rings` times, each new inner face inset again.
fn loops_around(mesh: Mesh, face: usize, rings: usize) -> Result<Mesh, String> {
    let mut mesh = mesh;
    let mut face = face;
    // Ring widths are even, down to an inner face 0.45 of the outer side: split in four afterwards, that makes the
    // centre's quads and the rings' quads about the same size.
    let side = |k: usize| 1.0 - k as f32 * (1.0 - 0.45) / rings.max(1) as f32;
    for k in 1..=rings {
        let quads = mesh.quad_faces().ok_or("the template lost its quads")?;
        let corners = quads[face];
        let hit = PolyHit { quad: face, corners, edge: (corners[0], corners[1]), edge_index: 0, vertex: corners[0], border: false };
        mesh = inset_face(&mesh, &hit, 1.0 - side(k) / side(k - 1), 0.0, false)?;
        // inset_face puts the new inner face last.
        face = mesh.triangle_count() / 2 - 1;
    }
    Ok(mesh)
}

/// The unit-frame template: a quad sphere of `res` quads per cube edge, and for the head two eye loops and a mouth
/// loop of `rings` rings each on its front (+z).
fn template(kind: &str, res: usize, rings: usize) -> Result<(Vec<Vec3>, Vec<[u32; 4]>), String> {
    let res = res.clamp(2, 32);
    let (points, quads) = cube_sphere(res);
    match kind {
        "sphere" => Ok((points, quads)),
        "head" => {
            if res < 4 {
                return Err("the head template needs at least 4 quads per side for its eye and mouth loops".into());
            }
            // The loops are made at half resolution, each inset inside one coarse quad (two by two final quads), and
            // then every quad is split in four: each coarse ring becomes two rings of well-shaped quads, where rings
            // inset inside a single final quad would be thin slivers round a tiny centre.
            // An odd coarse count puts a middle column under the mouth and one column between the eyes.
            let (points, quads) = cube_sphere((res / 2).max(3) | 1);
            let mut mesh = Mesh::new(points.iter().map(|p| p.to_array()).collect(), quads.iter().flat_map(|q| [q[0], q[1], q[2], q[0], q[2], q[3]]).collect());
            mesh.quads = true;
            let coarse_rings = rings.clamp(1, 6).div_ceil(2);
            for dir in [Vec3::new(-0.38, 0.22, 0.9), Vec3::new(0.38, 0.22, 0.9), Vec3::new(0.0, -0.42, 0.9)] {
                let qs = mesh.quad_faces().ok_or("the template lost its quads")?;
                let ps: Vec<Vec3> = mesh.positions.iter().map(|&p| Vec3::from(p)).collect();
                let face = quad_toward(&ps, &qs, dir);
                mesh = loops_around(mesh, face, coarse_rings)?;
            }
            let qs = mesh.quad_faces().ok_or("the template lost its quads")?;
            let ps: Vec<Vec3> = mesh.positions.iter().map(|&p| Vec3::from(p).normalize_or(Vec3::Z)).collect();
            Ok(split_on_sphere(&ps, &qs))
        }
        other => Err(format!("unknown template {other}; use {}", WRAP_KINDS.join(" or "))),
    }
}

/// Lays a whole-object template onto `target`: its front faces `front` (toward the viewer), its top `up`; it is
/// scaled to the target's extent along those axes, each point is put on the outermost surface along the ray from the
/// target's middle through it, and `relax` passes even the quads out along the surface.
#[allow(clippy::too_many_arguments)]
pub fn wrap_template(kind: &str, res: usize, rings: usize, target: &Mesh, bvh: &Bvh, up: Vec3, front: Vec3, relax: u32) -> Result<Wrapped, String> {
    if target.vertex_count() == 0 {
        return Err("the target has no surface".into());
    }
    let (unit, quads) = template(kind, res, rings)?;
    let f = front.try_normalize().ok_or("front must not be zero")?;
    let u = (up - f * up.dot(f)).try_normalize().ok_or("up must not run along front")?;
    let r = u.cross(f);
    // The target's extent along the three axes, and the middle of that box.
    let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for &p in &target.positions {
        let p = Vec3::from(p);
        let q = Vec3::new(p.dot(r), p.dot(u), p.dot(f));
        lo = lo.min(q);
        hi = hi.max(q);
    }
    let mid = (lo + hi) * 0.5;
    let half = ((hi - lo) * 0.5).max(Vec3::splat(1e-6));
    let centre = r * mid.x + u * mid.y + f * mid.z;
    let reach = (hi - lo).length() * 2.0 + 1e-3;
    let place = |p: Vec3| centre + r * (p.x * half.x) + u * (p.y * half.y) + f * (p.z * half.z);
    // The outermost surface point on the ray from the middle through q, else the nearest surface point.
    let drop = |q: Vec3| -> Option<(Vec3, Vec3)> {
        let dir = (q - centre).try_normalize().unwrap_or(u);
        // A ray aimed exactly through a vertex or along an edge (a cube's corners lie on template rays) can slip
        // between the triangles there and hit the far side; such a hit is behind the middle, so the ray is nudged
        // and cast again.
        let side = dir.any_orthonormal_vector();
        for nudge in [0.0, 1e-4, -1e-4, 1e-3] {
            let d = (dir + side * nudge + side.cross(dir) * nudge * 0.7).normalize();
            if let Some(h) = bvh.raycast(target, centre + d * reach, -d) {
                if (h.point - centre).dot(d) > 0.0 {
                    let [a, b, c] = target.triangle(h.triangle as usize).map(|v| target.position(v));
                    return Some((h.point, (b - a).cross(c - a).normalize_or_zero()));
                }
            }
        }
        nearest_point(bvh, target, q, reach).map(|(p, n, _)| (p, n.normalize_or_zero()))
    };
    let mut points = Vec::with_capacity(unit.len());
    let mut normals = Vec::with_capacity(unit.len());
    for &p in &unit {
        let (q, n) = drop(place(p)).ok_or("the template reaches past the target's surface")?;
        points.push(q);
        normals.push(n);
    }
    // Relax: each point toward the mean of its quad neighbours, along the surface, then back onto it.
    let mut nb: Vec<HashSet<u32>> = vec![HashSet::new(); points.len()];
    for q in &quads {
        for k in 0..4 {
            nb[q[k] as usize].insert(q[(k + 1) % 4]);
            nb[q[(k + 1) % 4] as usize].insert(q[k]);
        }
    }
    let nb: Vec<Vec<u32>> = nb.into_iter().map(|s| { let mut v: Vec<u32> = s.into_iter().collect(); v.sort_unstable(); v }).collect();
    for _ in 0..relax.min(50) {
        let moved: Vec<Vec3> = (0..points.len())
            .map(|i| {
                if nb[i].is_empty() {
                    return points[i];
                }
                let avg = nb[i].iter().map(|&j| points[j as usize]).sum::<Vec3>() / nb[i].len() as f32;
                let d = avg - points[i];
                points[i] + (d - normals[i] * d.dot(normals[i])) * 0.5
            })
            .collect();
        // Back onto the surface along the same rays from the middle, so a point never jumps across an edge to a
        // nearer face the way a nearest-point projection can.
        for (i, q) in moved.into_iter().enumerate() {
            if let Some((p, n)) = drop(q) {
                points[i] = p;
                if n.length_squared() > 0.0 {
                    normals[i] = n;
                }
            }
        }
    }
    let folded = quads
        .iter()
        .filter(|q| {
            let p = q.map(|k| points[k as usize]);
            let s: Vec3 = q.iter().map(|&k| normals[k as usize]).sum();
            (p[2] - p[0]).cross(p[3] - p[1]).dot(s) < 0.0
        })
        .count();
    Ok(Wrapped { points, normals, quads, folded })
}
