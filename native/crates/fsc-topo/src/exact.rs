//! Exact mesh booleans: the two surfaces are cut along their true intersection and the kept pieces joined, so detail
//! finer than any voxel survives and flat faces stay flat. Built on boolmesh (after Manifold's algorithm: symbolic
//! perturbation keeps it robust on coplanar and touching input). Both meshes must be closed and manifold; paint rides
//! through as values on each triangle corner, interpolated along the cut.

use std::ops::{Add, Mul};

use boolmesh::prelude::{compute_boolean, Manifold, OpType};
use fsc_core::{Mesh, Paint, PAINT_LEN};

use crate::voxel::BooleanOp;

/// Paint carried on each triangle corner through the boolean.
#[derive(Clone, Debug, PartialEq, Default)]
struct Corner([f64; PAINT_LEN]);

impl Add for Corner {
    type Output = Corner;
    fn add(self, o: Corner) -> Corner {
        Corner(std::array::from_fn(|k| self.0[k] + o.0[k]))
    }
}

impl Mul<f64> for Corner {
    type Output = Corner;
    fn mul(self, s: f64) -> Corner {
        Corner(self.0.map(|v| v * s))
    }
}

fn manifold(mesh: &Mesh, painted: bool, which: &str) -> Result<Manifold<Corner>, String> {
    let pos: Vec<f64> = mesh.positions.iter().flat_map(|p| p.map(f64::from)).collect();
    let idx: Vec<usize> = mesh.indices.iter().map(|&i| i as usize).collect();
    let vals: Vec<Corner> = if painted {
        let paint = |v: u32| mesh.paint.get(v as usize).copied().unwrap_or(fsc_core::DEFAULT_PAINT);
        mesh.indices.iter().map(|&v| Corner(paint(v).map(f64::from))).collect()
    } else {
        Vec::new()
    };
    Manifold::new_with_values(&pos, &idx, &vals).map_err(|e| format!("{which} is not a closed manifold surface ({e}); repair_mesh it, or use the voxel boolean"))
}

/// The exact union, difference (a minus b) or intersection of two closed meshes.
pub fn exact_boolean(a: &Mesh, b: &Mesh, op: BooleanOp) -> Result<Mesh, String> {
    let painted = a.has_paint() || b.has_paint();
    let (ma, mb) = (manifold(a, painted, "a")?, manifold(b, painted, "b")?);
    let kind = match op {
        BooleanOp::Union => OpType::Add,
        BooleanOp::Difference => OpType::Subtract,
        BooleanOp::Intersection => OpType::Intersect,
    };
    const NOTHING: &str = "the boolean left nothing (the objects do not overlap for an intersection, or b covers a)";
    let out = compute_boolean(&ma, &mb, kind).map_err(|e| if e.contains("empty") { NOTHING.to_string() } else { e })?;
    let positions: Vec<[f32; 3]> = out.ps.iter().map(|p| [p.x as f32, p.y as f32, p.z as f32]).collect();
    let indices: Vec<u32> = out.hs.iter().map(|h| h.tail as u32).collect();
    if indices.is_empty() {
        return Err(NOTHING.into());
    }
    let mut mesh = Mesh::new(positions, indices);
    // Paint: corner values averaged per vertex (corners of one vertex differ only across a paint seam).
    if painted && out.vals.len() == mesh.indices.len() {
        let mut sum = vec![[0.0f64; PAINT_LEN]; mesh.vertex_count()];
        let mut count = vec![0u32; mesh.vertex_count()];
        for (c, &v) in mesh.indices.iter().enumerate() {
            for (total, value) in sum[v as usize].iter_mut().zip(&out.vals[c].0) {
                *total += value;
            }
            count[v as usize] += 1;
        }
        mesh.paint = sum
            .iter()
            .zip(&count)
            .map(|(s, &n)| if n == 0 { fsc_core::DEFAULT_PAINT } else { std::array::from_fn(|k| (s[k] / f64::from(n)) as f32) as Paint })
            .collect();
    }
    Ok(mesh)
}
