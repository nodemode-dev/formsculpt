use glam::Vec3;

use crate::mesh::Mesh;

#[derive(Debug, Clone, Default)]
pub struct Topology {
    pub face_offsets: Vec<u32>,
    pub faces: Vec<u32>,
    pub neighbor_offsets: Vec<u32>,
    pub neighbors: Vec<u32>,
}

impl Topology {
    pub fn build(mesh: &Mesh) -> Topology {
        let n = mesh.vertex_count();
        let mut counts = vec![0u32; n + 1];
        for &i in &mesh.indices {
            counts[i as usize + 1] += 1;
        }
        for i in 0..n {
            counts[i + 1] += counts[i];
        }
        let face_offsets = counts.clone();
        let mut cursor = counts;
        let mut faces = vec![0u32; mesh.indices.len()];
        for t in 0..mesh.triangle_count() {
            for v in mesh.triangle(t) {
                faces[cursor[v as usize] as usize] = t as u32;
                cursor[v as usize] += 1;
            }
        }
        let mut neighbor_offsets = Vec::with_capacity(n + 1);
        let mut neighbors = Vec::with_capacity(mesh.indices.len());
        neighbor_offsets.push(0);
        let mut scratch: Vec<u32> = Vec::with_capacity(16);
        for v in 0..n {
            scratch.clear();
            for &t in &faces[face_offsets[v] as usize..face_offsets[v + 1] as usize] {
                for w in mesh.triangle(t as usize) {
                    if w as usize != v {
                        scratch.push(w);
                    }
                }
            }
            scratch.sort_unstable();
            scratch.dedup();
            neighbors.extend_from_slice(&scratch);
            neighbor_offsets.push(neighbors.len() as u32);
        }
        Topology { face_offsets, faces, neighbor_offsets, neighbors }
    }

    pub fn faces_of(&self, v: u32) -> &[u32] {
        &self.faces[self.face_offsets[v as usize] as usize..self.face_offsets[v as usize + 1] as usize]
    }

    pub fn neighbors_of(&self, v: u32) -> &[u32] {
        &self.neighbors[self.neighbor_offsets[v as usize] as usize..self.neighbor_offsets[v as usize + 1] as usize]
    }

    pub fn update_normals(&self, mesh: &mut Mesh, verts: &[u32]) {
        for &v in verts {
            let mut acc = Vec3::ZERO;
            for &t in self.faces_of(v) {
                let [a, b, c] = mesh.triangle(t as usize);
                let (pa, pb, pc) = (mesh.position(a), mesh.position(b), mesh.position(c));
                acc += (pb - pa).cross(pc - pa);
            }
            mesh.normals[v as usize] = acc.normalize_or(Vec3::Y).to_array();
        }
    }

    pub fn ring_vertices(&self, verts: &[u32], out: &mut Vec<u32>) {
        out.clear();
        out.extend_from_slice(verts);
        for &v in verts {
            out.extend_from_slice(self.neighbors_of(v));
        }
        out.sort_unstable();
        out.dedup();
    }
}
