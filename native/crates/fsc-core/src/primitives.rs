use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use glam::Vec3;

use crate::mesh::Mesh;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    Sphere { radius: f32, subdivisions: u32 },
    UvSphere { radius: f32, segments: u32, rings: u32 },
    Cube { size: f32, divisions: u32 },
    Cylinder { radius: f32, height: f32, segments: u32, rings: u32 },
    Cone { radius: f32, height: f32, segments: u32 },
    Torus { major: f32, minor: f32, segments: u32, sides: u32 },
    Plane { size: f32, divisions: u32 },
}

impl Primitive {
    pub const KINDS: [&'static str; 7] = ["sphere", "uv_sphere", "cube", "cylinder", "cone", "torus", "plane"];

    pub fn build(&self) -> Mesh {
        match *self {
            Primitive::Sphere { radius, subdivisions } => icosphere(radius, subdivisions.min(9)),
            Primitive::UvSphere { radius, segments, rings } => uv_sphere(radius, segments.clamp(3, 2048), rings.clamp(2, 2048)),
            Primitive::Cube { size, divisions } => cube(size, divisions.clamp(1, 1024)),
            Primitive::Cylinder { radius, height, segments, rings } => cylinder(radius, height, segments.clamp(3, 2048), rings.clamp(1, 2048)),
            Primitive::Cone { radius, height, segments } => cone(radius, height, segments.clamp(3, 2048)),
            Primitive::Torus { major, minor, segments, sides } => torus(major, minor, segments.clamp(3, 2048), sides.clamp(3, 2048)),
            Primitive::Plane { size, divisions } => plane(size, divisions.clamp(1, 2048)),
        }
    }
}

fn icosphere(radius: f32, subdivisions: u32) -> Mesh {
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let mut verts: Vec<Vec3> = [
        [-1.0, t, 0.0], [1.0, t, 0.0], [-1.0, -t, 0.0], [1.0, -t, 0.0],
        [0.0, -1.0, t], [0.0, 1.0, t], [0.0, -1.0, -t], [0.0, 1.0, -t],
        [t, 0.0, -1.0], [t, 0.0, 1.0], [-t, 0.0, -1.0], [-t, 0.0, 1.0],
    ]
    .iter()
    .map(|p| Vec3::from_array(*p).normalize())
    .collect();
    let mut faces: Vec<[u32; 3]> = vec![
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ];
    for _ in 0..subdivisions {
        let mut cache: HashMap<(u32, u32), u32> = HashMap::with_capacity(faces.len() * 3 / 2);
        let mut mid = |a: u32, b: u32, verts: &mut Vec<Vec3>| -> u32 {
            let key = if a < b { (a, b) } else { (b, a) };
            *cache.entry(key).or_insert_with(|| {
                verts.push(((verts[a as usize] + verts[b as usize]) * 0.5).normalize());
                (verts.len() - 1) as u32
            })
        };
        let mut next = Vec::with_capacity(faces.len() * 4);
        for [a, b, c] in faces {
            let ab = mid(a, b, &mut verts);
            let bc = mid(b, c, &mut verts);
            let ca = mid(c, a, &mut verts);
            next.extend_from_slice(&[[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        faces = next;
    }
    let positions = verts.iter().map(|v| (*v * radius).to_array()).collect();
    Mesh::new(positions, faces.into_iter().flatten().collect())
}

fn grid_indices(cols: u32, rows: u32, wrap_cols: bool, base: u32, out: &mut Vec<u32>) {
    let stride = if wrap_cols { cols } else { cols + 1 };
    for r in 0..rows {
        for c in 0..cols {
            let c1 = if wrap_cols { (c + 1) % cols } else { c + 1 };
            let a = base + r * stride + c;
            let b = base + r * stride + c1;
            let d = base + (r + 1) * stride + c;
            let e = base + (r + 1) * stride + c1;
            out.extend_from_slice(&[a, d, b, b, d, e]);
        }
    }
}

fn uv_sphere(radius: f32, segments: u32, rings: u32) -> Mesh {
    let mut positions = vec![[0.0, radius, 0.0]];
    for r in 1..rings {
        let phi = PI * r as f32 / rings as f32;
        for s in 0..segments {
            let theta = TAU * s as f32 / segments as f32;
            positions.push([radius * phi.sin() * theta.cos(), radius * phi.cos(), -radius * phi.sin() * theta.sin()]);
        }
    }
    positions.push([0.0, -radius, 0.0]);
    let bottom = (positions.len() - 1) as u32;
    let mut indices = Vec::new();
    for s in 0..segments {
        indices.extend_from_slice(&[0, 1 + s, 1 + (s + 1) % segments]);
    }
    grid_indices(segments, rings - 2, true, 1, &mut indices);
    let last = 1 + (rings - 2) * segments;
    for s in 0..segments {
        indices.extend_from_slice(&[last + s, bottom, last + (s + 1) % segments]);
    }
    Mesh::new(positions, indices)
}

fn plane_face(origin: Vec3, u: Vec3, v: Vec3, n: u32, positions: &mut Vec<[f32; 3]>, indices: &mut Vec<u32>) {
    let base = positions.len() as u32;
    for j in 0..=n {
        for i in 0..=n {
            let p = origin + u * (i as f32 / n as f32) + v * (j as f32 / n as f32);
            positions.push(p.to_array());
        }
    }
    let start = indices.len();
    grid_indices(n, n, false, base, indices);
    // grid_indices winds its cells to face v x u; these faces must face u x v (out of a cube, up from a plane).
    for t in indices[start..].chunks_exact_mut(3) {
        t.swap(1, 2);
    }
}

fn plane(size: f32, n: u32) -> Mesh {
    let h = size / 2.0;
    let (mut positions, mut indices) = (Vec::new(), Vec::new());
    plane_face(Vec3::new(-h, 0.0, h), Vec3::new(size, 0.0, 0.0), Vec3::new(0.0, 0.0, -size), n, &mut positions, &mut indices);
    Mesh::new(positions, indices)
}

fn cube(size: f32, n: u32) -> Mesh {
    let h = size / 2.0;
    let (mut positions, mut indices) = (Vec::new(), Vec::new());
    let faces = [
        (Vec3::new(-h, -h, h), Vec3::X * size, Vec3::Y * size),
        (Vec3::new(h, -h, -h), -Vec3::X * size, Vec3::Y * size),
        (Vec3::new(h, -h, h), -Vec3::Z * size, Vec3::Y * size),
        (Vec3::new(-h, -h, -h), Vec3::Z * size, Vec3::Y * size),
        (Vec3::new(-h, h, h), Vec3::X * size, -Vec3::Z * size),
        (Vec3::new(-h, -h, -h), Vec3::X * size, Vec3::Z * size),
    ];
    for (o, u, v) in faces {
        plane_face(o, u, v, n, &mut positions, &mut indices);
    }
    weld(positions, indices, size * 1e-5)
}

fn weld(positions: Vec<[f32; 3]>, indices: Vec<u32>, eps: f32) -> Mesh {
    let mut map: HashMap<[i64; 3], u32> = HashMap::with_capacity(positions.len());
    let mut remap = Vec::with_capacity(positions.len());
    let mut out = Vec::with_capacity(positions.len());
    for p in &positions {
        let key = [(p[0] / eps).round() as i64, (p[1] / eps).round() as i64, (p[2] / eps).round() as i64];
        let id = *map.entry(key).or_insert_with(|| {
            out.push(*p);
            (out.len() - 1) as u32
        });
        remap.push(id);
    }
    Mesh::new(out, indices.into_iter().map(|i| remap[i as usize]).collect())
}

fn cylinder(radius: f32, height: f32, segments: u32, rings: u32) -> Mesh {
    let h = height / 2.0;
    let mut positions = Vec::new();
    for r in 0..=rings {
        let y = h - height * r as f32 / rings as f32;
        for s in 0..segments {
            let theta = TAU * s as f32 / segments as f32;
            positions.push([radius * theta.cos(), y, -radius * theta.sin()]);
        }
    }
    let mut indices = Vec::new();
    grid_indices(segments, rings, true, 0, &mut indices);
    let top = positions.len() as u32;
    positions.push([0.0, h, 0.0]);
    let bottom = positions.len() as u32;
    positions.push([0.0, -h, 0.0]);
    let last = rings * segments;
    for s in 0..segments {
        let s1 = (s + 1) % segments;
        indices.extend_from_slice(&[top, s, s1]);
        indices.extend_from_slice(&[bottom, last + s1, last + s]);
    }
    Mesh::new(positions, indices)
}

fn cone(radius: f32, height: f32, segments: u32) -> Mesh {
    let h = height / 2.0;
    let mut positions = vec![[0.0, h, 0.0]];
    for s in 0..segments {
        let theta = TAU * s as f32 / segments as f32;
        positions.push([radius * theta.cos(), -h, -radius * theta.sin()]);
    }
    let bottom = positions.len() as u32;
    positions.push([0.0, -h, 0.0]);
    let mut indices = Vec::new();
    for s in 0..segments {
        let a = 1 + s;
        let b = 1 + (s + 1) % segments;
        indices.extend_from_slice(&[0, a, b, bottom, b, a]);
    }
    Mesh::new(positions, indices)
}

fn torus(major: f32, minor: f32, segments: u32, sides: u32) -> Mesh {
    let mut positions = Vec::with_capacity((segments * sides) as usize);
    for i in 0..segments {
        let u = TAU * i as f32 / segments as f32;
        for j in 0..sides {
            let v = TAU * j as f32 / sides as f32;
            let r = major + minor * v.cos();
            positions.push([r * u.cos(), minor * v.sin(), -r * u.sin()]);
        }
    }
    let mut indices = Vec::new();
    for i in 0..segments {
        let i1 = (i + 1) % segments;
        for j in 0..sides {
            let j1 = (j + 1) % sides;
            let (a, b, c, d) = (i * sides + j, i1 * sides + j, i1 * sides + j1, i * sides + j1);
            indices.extend_from_slice(&[a, b, d, b, c, d]);
        }
    }
    Mesh::new(positions, indices)
}
