use fsc_core::{Bvh, Mesh, Topology};
use glam::Vec3;

use crate::nearest::nearest_point;
use crate::voxel::voxel_remesh;

pub struct QuadRemesh {
    pub mesh: Mesh,
    pub resolution: u32,
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
