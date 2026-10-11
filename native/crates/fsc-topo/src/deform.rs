use fsc_core::Mesh;
use glam::{EulerRot, Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Deformer {
    Bend { axis: usize, toward: usize, angle: f32 },
    Twist { axis: usize, angle: f32 },
    Taper { axis: usize, amount: f32 },
    Transform { translate: Vec3, rotate_deg: Vec3, scale: Vec3 },
}

pub fn deform(mesh: &Mesh, mask: &[f32], deformer: Deformer) -> Vec<[f32; 3]> {
    let bounds = mesh.bounds();
    let center = bounds.center();
    let extent = bounds.extent();
    let one = |p: Vec3| -> Vec3 {
        let d = p - center;
        match deformer {
            Deformer::Bend { axis, toward, angle } => {
                let len = extent[axis].max(1e-6);
                if angle.abs() < 1e-6 {
                    return p;
                }
                let radius = len / angle;
                let phi = angle * d[axis] / len;
                let r = radius - d[toward];
                let mut q = d;
                q[toward] = radius - r * phi.cos();
                q[axis] = r * phi.sin();
                center + q
            }
            Deformer::Twist { axis, angle } => {
                let len = extent[axis].max(1e-6);
                let mut unit = Vec3::ZERO;
                unit[axis] = 1.0;
                center + Quat::from_axis_angle(unit, angle * d[axis] / len) * d
            }
            Deformer::Taper { axis, amount } => {
                let len = extent[axis].max(1e-6);
                let f = (1.0 + amount * d[axis] / len).max(0.0);
                let mut q = d * f;
                q[axis] = d[axis];
                center + q
            }
            Deformer::Transform { translate, rotate_deg, scale } => {
                let r = Quat::from_euler(EulerRot::XYZ, rotate_deg.x.to_radians(), rotate_deg.y.to_radians(), rotate_deg.z.to_radians());
                center + translate + r * (d * scale)
            }
        }
    };
    mesh.positions
        .iter()
        .enumerate()
        .map(|(v, p)| {
            let p = Vec3::from_array(*p);
            let w = 1.0 - mask.get(v).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            p.lerp(one(p), w).to_array()
        })
        .collect()
}

pub fn split_parts(mesh: &Mesh) -> Vec<Mesh> {
    let n = mesh.vertex_count();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(parent: &mut [u32], mut v: u32) -> u32 {
        while parent[v as usize] != v {
            parent[v as usize] = parent[parent[v as usize] as usize];
            v = parent[v as usize];
        }
        v
    }
    for t in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(t);
        for (x, y) in [(a, b), (b, c)] {
            let (rx, ry) = (find(&mut parent, x), find(&mut parent, y));
            if rx != ry {
                parent[rx as usize] = ry;
            }
        }
    }
    let mut part_of_root: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut parts: Vec<(Vec<[f32; 3]>, Vec<u32>)> = Vec::new();
    let mut local = vec![u32::MAX; n];
    for t in 0..mesh.triangle_count() {
        let tri = mesh.triangle(t);
        let root = find(&mut parent, tri[0]);
        let pi = *part_of_root.entry(root).or_insert_with(|| {
            parts.push((Vec::new(), Vec::new()));
            parts.len() - 1
        });
        for v in tri {
            if local[v as usize] == u32::MAX {
                local[v as usize] = parts[pi].0.len() as u32;
                parts[pi].0.push(mesh.positions[v as usize]);
            }
            parts[pi].1.push(local[v as usize]);
        }
    }
    parts.into_iter().map(|(p, i)| Mesh::new(p, i)).collect()
}

pub fn merge(meshes: &[&Mesh]) -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for m in meshes {
        let base = positions.len() as u32;
        positions.extend_from_slice(&m.positions);
        indices.extend(m.indices.iter().map(|i| i + base));
    }
    Mesh::new(positions, indices)
}
