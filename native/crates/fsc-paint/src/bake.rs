use fsc_core::{Bvh, Mesh, Paint, DEFAULT_PAINT};
use glam::{Vec2, Vec3};

use crate::uv::Expanded;

pub struct Baked {
    pub size: usize,
    pub color: Vec<[u8; 4]>,
    pub mr: Vec<[u8; 4]>,
    pub normal: Vec<[u8; 4]>,
    pub displacement: Vec<f32>,
    pub covered: usize,
    pub hits: usize,
}

type Sample = ([u8; 4], [u8; 4], [u8; 4], f32, bool);

struct Texel {
    tri: u32,
    bary: [f32; 3],
}

fn to8(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn paint_at(mesh: &Mesh, tri: [u32; 3], w: [f32; 3]) -> Paint {
    if !mesh.has_paint() {
        return DEFAULT_PAINT;
    }
    std::array::from_fn(|k| mesh.paint[tri[0] as usize][k] * w[0] + mesh.paint[tri[1] as usize][k] * w[1] + mesh.paint[tri[2] as usize][k] * w[2])
}

pub fn bake(low: &Mesh, ex: &Expanded, source: &Mesh, source_bvh: &Bvh, size: usize, max_dist: f32, padding: usize) -> Baked {
    let mut owner: Vec<Option<Texel>> = (0..size * size).map(|_| None).collect();
    for t in 0..low.triangle_count() {
        let q: [Vec2; 3] = std::array::from_fn(|k| Vec2::from_array(low.uvs[3 * t + k]) * size as f32);
        let lo = q[0].min(q[1]).min(q[2]).floor().max(Vec2::ZERO);
        let hi = q[0].max(q[1]).max(q[2]).ceil().min(Vec2::splat(size as f32 - 1.0));
        let area = (q[1] - q[0]).perp_dot(q[2] - q[0]);
        if area.abs() < 1e-12 {
            continue;
        }
        for j in lo.y as usize..=hi.y as usize {
            for i in lo.x as usize..=hi.x as usize {
                let p = Vec2::new(i as f32 + 0.5, j as f32 + 0.5);
                let w1 = (q[2] - q[1]).perp_dot(p - q[1]) / area;
                let w2 = (q[0] - q[2]).perp_dot(p - q[2]) / area;
                let w0 = 1.0 - w1 - w2;
                if w1 >= -1e-4 && w2 >= -1e-4 && w0 >= -1e-4 {
                    owner[j * size + i] = Some(Texel { tri: t as u32, bary: [w1, w2, w0] });
                }
            }
        }
    }
    let texel = |at: usize| -> Option<Sample> {
        let Texel { tri, bary } = owner[at].as_ref()?;
        let t = *tri as usize;
        let ids = [low.indices[3 * t], low.indices[3 * t + 1], low.indices[3 * t + 2]];
        let cids = [ex.corner[3 * t], ex.corner[3 * t + 1], ex.corner[3 * t + 2]];
        let w = [bary[0], bary[1], bary[2]];
        let p: Vec3 = (0..3).map(|k| low.position(ids[k]) * w[k]).sum();
        let n: Vec3 = (0..3).map(|k| Vec3::from_array(low.normals[ids[k] as usize]) * w[k]).sum::<Vec3>().normalize_or(Vec3::Y);
        let tg: Vec3 = (0..3).map(|k| Vec3::from_slice(&ex.tangent[cids[k] as usize][..3]) * w[k]).sum();
        let sign = ex.tangent[cids[0] as usize][3];
        let tg = (tg - n * n.dot(tg)).try_normalize().unwrap_or_else(|| n.any_orthonormal_vector());
        let bt = n.cross(tg) * sign;
        let mut best: Option<(f32, fsc_core::Hit)> = None;
        for dir in [n, -n] {
            if let Some(h) = source_bvh.raycast(source, p - dir * 1e-5, dir) {
                let d = h.t - 1e-5;
                if d <= max_dist && best.as_ref().is_none_or_le(d) {
                    best = Some((if dir == n { d } else { -d }, h));
                }
            }
        }
        let (paint, hn, disp, hit) = match best {
            Some((d, h)) => {
                let st = source.triangle(h.triangle as usize);
                let sw = [1.0 - h.u - h.v, h.u, h.v];
                let hn: Vec3 = (0..3).map(|k| Vec3::from_array(source.normals[st[k] as usize]) * sw[k]).sum::<Vec3>().normalize_or(n);
                let paint = if source.has_paint() { paint_at(source, st, sw) } else { paint_at(low, ids, w) };
                (paint, hn, d, true)
            }
            None => (paint_at(low, ids, w), n, 0.0, false),
        };
        let local = Vec3::new(hn.dot(tg), hn.dot(bt), hn.dot(n)).normalize_or(Vec3::Z);
        Some((
            [to8(paint[0]), to8(paint[1]), to8(paint[2]), 255],
            [0, to8(paint[3]), to8(paint[4]), 255],
            [to8(local.x * 0.5 + 0.5), to8(local.y * 0.5 + 0.5), to8(local.z * 0.5 + 0.5), 255],
            disp,
            hit,
        ))
    };
    #[cfg(feature = "parallel")]
    let samples: Vec<_> = {
        use rayon::prelude::*;
        (0..size * size).into_par_iter().map(texel).collect()
    };
    #[cfg(not(feature = "parallel"))]
    let samples: Vec<_> = (0..size * size).map(texel).collect();
    let covered = samples.iter().filter(|s| s.is_some()).count();
    let hits = samples.iter().filter(|s| s.as_ref().is_some_and(|x| x.4)).count();
    let mut color: Vec<Option<[u8; 4]>> = samples.iter().map(|s| s.as_ref().map(|x| x.0)).collect();
    let mut mr: Vec<Option<[u8; 4]>> = samples.iter().map(|s| s.as_ref().map(|x| x.1)).collect();
    let mut normal: Vec<Option<[u8; 4]>> = samples.iter().map(|s| s.as_ref().map(|x| x.2)).collect();
    let mut disp: Vec<Option<f32>> = samples.iter().map(|s| s.as_ref().map(|x| x.3)).collect();
    for _ in 0..padding {
        let filled: Vec<bool> = color.iter().map(|c| c.is_some()).collect();
        for j in 0..size {
            for i in 0..size {
                let at = j * size + i;
                if filled[at] {
                    continue;
                }
                let near = [(i > 0).then(|| at - 1), (i + 1 < size).then(|| at + 1), (j > 0).then(|| at - size), (j + 1 < size).then(|| at + size)];
                if let Some(src) = near.into_iter().flatten().find(|&n| filled[n]) {
                    color[at] = color[src];
                    mr[at] = mr[src];
                    normal[at] = normal[src];
                    disp[at] = disp[src];
                }
            }
        }
    }
    let base = DEFAULT_PAINT;
    Baked {
        size,
        color: color.into_iter().map(|c| c.unwrap_or([to8(base[0]), to8(base[1]), to8(base[2]), 255])).collect(),
        mr: mr.into_iter().map(|c| c.unwrap_or([0, to8(base[3]), to8(base[4]), 255])).collect(),
        normal: normal.into_iter().map(|c| c.unwrap_or([128, 128, 255, 255])).collect(),
        displacement: disp.into_iter().map(|d| d.unwrap_or(0.0)).collect(),
        covered,
        hits,
    }
}

trait BestDistance {
    fn is_none_or_le(&self, d: f32) -> bool;
}

impl BestDistance for Option<&(f32, fsc_core::Hit)> {
    fn is_none_or_le(&self, d: f32) -> bool {
        self.map_or(true, |b| d < b.0.abs())
    }
}
