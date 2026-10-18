//! The Triplanar primitive, as Nomad Sculpt's manual describes it: three planes whose masks fill a voxel grid that is
//! then polygonized. Here the planes are the front (x, y; facing +z), the side (z, y; facing +x) and the top (x, z;
//! facing +y), each an n by n grid in its own face group, made in that order so each plane's grid can be read back from
//! the vertex order (and so still after the object is moved or turned). The solid is built by fsc-topo's
//! triplanar_solid.

use glam::Vec3;

use crate::Mesh;

/// The planes' axes: (across, up) for front, side and top.
pub const AXES: [(Vec3, Vec3); 3] = [(Vec3::X, Vec3::Y), (Vec3::Z, Vec3::Y), (Vec3::X, Vec3::Z)];

/// Three crossing square planes of half size `size` and `n` cells a side, and a starting mask: a disc on each plane
/// (radius 0.6 of the half size, with a soft rim), so the first solid is the rounded shape the three discs carve.
pub fn planes(size: f32, n: u32) -> (Mesh, Vec<f32>) {
    let n = n.clamp(8, 512);
    let side = n + 1;
    let mut positions = Vec::with_capacity((side * side * 3) as usize);
    let mut indices = Vec::with_capacity((n * n * 18) as usize);
    let mut groups = Vec::with_capacity((n * n * 6) as usize);
    let mut mask = Vec::with_capacity((side * side * 3) as usize);
    for (k, (u, v)) in AXES.iter().enumerate() {
        let base = positions.len() as u32;
        for j in 0..side {
            for i in 0..side {
                let (a, b) = (-1.0 + 2.0 * i as f32 / n as f32, -1.0 + 2.0 * j as f32 / n as f32);
                positions.push(((*u * a + *v * b) * size).to_array());
                let r = (a * a + b * b).sqrt();
                let t = ((0.66 - r) / 0.12).clamp(0.0, 1.0);
                mask.push(t * t * (3.0 - 2.0 * t));
            }
        }
        for j in 0..n {
            for i in 0..n {
                let a = base + j * side + i;
                let (b, c, d) = (a + 1, a + side, a + side + 1);
                indices.extend_from_slice(&[a, b, d, a, d, c]);
                groups.extend_from_slice(&[k as u32, k as u32]);
            }
        }
    }
    let mut mesh = Mesh::new(positions, indices);
    mesh.groups = groups;
    (mesh, mask)
}

/// The cells a side of a triplanar mesh, read from its vertex count (three grids of (n + 1)^2 points), or None when the
/// mesh is not three such grids (its topology changed).
pub fn divisions(mesh: &Mesh) -> Option<u32> {
    let v = mesh.vertex_count();
    if v == 0 || !v.is_multiple_of(3) {
        return None;
    }
    let side = ((v / 3) as f64).sqrt().round() as usize;
    (side >= 2 && side * side * 3 == v && mesh.triangle_count() == (side - 1) * (side - 1) * 6).then(|| (side - 1) as u32)
}
