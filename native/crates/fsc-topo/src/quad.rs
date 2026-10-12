use fsc_core::{Bvh, Mesh, Topology};
use glam::Vec3;

use crate::nearest::nearest_point;
use crate::voxel::voxel_remesh;

pub struct QuadRemesh {
    pub mesh: Mesh,
    pub resolution: u32,
}

/// The frame quads should line up with: the guide curves' mean direction as the first axis (each tangent's sign
/// matched to the first), and the world axis least along it (made square) as the third. None without usable guides.
pub fn guide_frame(guides: &[Vec<Vec3>]) -> Option<glam::Mat3> {
    let mut sum = Vec3::ZERO;
    let mut first: Option<Vec3> = None;
    for g in guides {
        for w in g.windows(2) {
            let t = w[1] - w[0];
            if t.length_squared() < 1e-20 {
                continue;
            }
            let f = *first.get_or_insert(t.normalize());
            sum += if t.dot(f) < 0.0 { -t } else { t };
        }
    }
    let u = sum.try_normalize()?;
    // The third axis is the world axis least along u, made square to it, so flat sides square to the world (a box's
    // front stays a face of the grid) while the loops turn to the guides.
    let axis = [Vec3::X, Vec3::Y, Vec3::Z].into_iter().min_by(|a, b| a.dot(u).abs().total_cmp(&b.dot(u).abs())).unwrap_or(Vec3::Z);
    let w = (axis - u * axis.dot(u)).try_normalize()?;
    let v = w.cross(u);
    Some(glam::Mat3::from_cols(u, v, w))
}

/// Quad remesh with the grid turned to a frame (from guide_frame), so edge loops run along the guides' direction.
pub fn quad_remesh_aligned(mesh: &Mesh, frame: glam::Mat3, target_faces: usize, relax: u32) -> Result<QuadRemesh, String> {
    let inverse = frame.transpose();
    let mut local = mesh.clone();
    for p in local.positions.iter_mut() {
        *p = (inverse * Vec3::from(*p)).to_array();
    }
    local.recompute_normals();
    let local_bvh = Bvh::build(&local);
    let mut out = quad_remesh(&local, &local_bvh, target_faces, relax)?;
    for p in out.mesh.positions.iter_mut() {
        *p = (frame * Vec3::from(*p)).to_array();
    }
    out.mesh.recompute_normals();
    Ok(out)
}

pub fn quad_remesh(mesh: &Mesh, bvh: &Bvh, target_faces: usize, relax: u32) -> Result<QuadRemesh, String> {
    let area: f32 = mesh
        .face_normals()
        .iter()
        .map(|n| n.length() * 0.5)
        .sum();
    if area <= 0.0 {
        return Err("the mesh has no area".into());
    }
    let cell = (area / target_faces.max(8) as f32).sqrt();
    let longest = mesh.bounds().extent().max_element();
    let mut resolution = ((longest / cell).round() as u32).clamp(8, 320);
    let mut out = voxel_remesh(mesh, bvh, resolution)?;
    let got = out.triangle_count() as f32 / 2.0;
    let ratio = got / target_faces.max(8) as f32;
    if !(0.87..=1.15).contains(&ratio) {
        let corrected = ((resolution as f32 / ratio.sqrt()).round() as u32).clamp(8, 320);
        if corrected != resolution {
            resolution = corrected;
            out = voxel_remesh(mesh, bvh, resolution)?;
        }
    }
    if out.indices.len() % 6 != 0 {
        return Err("the remesh did not come out as quads".into());
    }
    let reach = longest * 0.1;
    let topo = Topology::build(&out);
    for _ in 0..relax {
        let targets: Vec<Vec3> = (0..out.vertex_count() as u32)
            .map(|v| {
                let p = out.position(v);
                let nb = topo.neighbors_of(v);
                if nb.is_empty() {
                    return p;
                }
                let avg = nb.iter().map(|&w| out.position(w)).sum::<Vec3>() / nb.len() as f32;
                let n = Vec3::from_array(out.normals[v as usize]);
                let d = avg - p;
                let moved = p + (d - n * d.dot(n)) * 0.6;
                nearest_point(bvh, mesh, moved, reach).map(|(q, _, _)| q).unwrap_or(moved)
            })
            .collect();
        for (v, t) in targets.into_iter().enumerate() {
            out.positions[v] = t.to_array();
        }
        out.recompute_normals();
    }
    out.quads = true;
    Ok(QuadRemesh { mesh: out, resolution })
}
