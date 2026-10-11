use fsc_core::{Bvh, Mesh, Paint};
use glam::Vec3;

use crate::nearest::nearest_feature;

fn sample(src: &Mesh, bvh: &Bvh, p: Vec3, reach: f32) -> Option<(Paint, f32)> {
    let (_, t, bary, d) = nearest_feature(bvh, src, p, reach)?;
    let [a, b, c] = src.triangle(t as usize);
    let (pa, pb, pc) = (src.paint[a as usize], src.paint[b as usize], src.paint[c as usize]);
    let out: Paint = std::array::from_fn(|k| pa[k] * bary[0] + pb[k] * bary[1] + pc[k] * bary[2]);
    Some((out, d))
}

pub fn transfer_paint(sources: &[(&Mesh, &Bvh)], dst: &mut Mesh) {
    let painted: Vec<&(&Mesh, &Bvh)> = sources.iter().filter(|(m, _)| m.has_paint()).collect();
    if painted.is_empty() {
        dst.paint.clear();
        return;
    }
    let reach = dst.bounds().extent().max_element().max(1e-6);
    let one = |p: &[f32; 3]| -> Paint {
        let p = Vec3::from_array(*p);
        let mut best: Option<(Paint, f32)> = None;
        for (m, bvh) in &painted {
            if let Some((v, d)) = sample(m, bvh, p, reach) {
                if best.map_or(true, |b| d < b.1) {
                    best = Some((v, d));
                }
            }
        }
        best.map(|b| b.0).unwrap_or(fsc_core::DEFAULT_PAINT)
    };
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        dst.paint = dst.positions.par_iter().map(one).collect();
    }
    #[cfg(not(feature = "parallel"))]
    {
        dst.paint = dst.positions.iter().map(one).collect();
    }
}
