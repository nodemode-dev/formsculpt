//! Per-texel paint in UV space (tile 1001), seven floats a texel in the order of vertex paint.

use glam::Vec2;

use crate::mesh::{Mesh, Paint, DEFAULT_PAINT, PAINT_LEN};

/// The smallest and largest texture sides.
pub const TEXTURE_MIN: usize = 256;
pub const TEXTURE_MAX: usize = 4096;
/// Texels of padding around each UV triangle that painting and creation fill, so filtering at seams never reads unpainted texels.
pub const PAD: f32 = 2.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// Texels per side.
    pub size: usize,
    /// size * size paint values, row j (v) then column i (u), v 0 first like the baked maps.
    pub texels: Vec<Paint>,
    /// True where a texel centre lies inside some UV triangle (padding texels are false).
    pub interior: Vec<bool>,
    /// Raised by every change, so a viewer knows when to fetch the picture again.
    pub revision: u64,
}

/// A texel a UV triangle reaches: its index, the triangle, barycentric weights in vertex order and whether its centre is inside.
#[derive(Debug, Clone, Copy)]
pub struct Texel {
    pub at: u32,
    pub tri: u32,
    pub bary: [f32; 3],
    pub inside: bool,
}

/// Appends the texels UV triangle `t` covers, plus padding texels within `margin` of its edges (weights clamped onto it), kept inside `window` (texel corners) when given.
pub fn rasterize(mesh: &Mesh, size: usize, t: usize, margin: f32, window: Option<(Vec2, Vec2)>, out: &mut Vec<Texel>) {
    let s = size as f32;
    let q: [Vec2; 3] = std::array::from_fn(|k| Vec2::from_array(mesh.uvs[3 * t + k]) * s);
    let area = (q[1] - q[0]).perp_dot(q[2] - q[0]);
    if !area.is_finite() || area.abs() < 1e-9 {
        return;
    }
    let lo = (q[0].min(q[1]).min(q[2]) - Vec2::splat(margin)).floor();
    let hi = (q[0].max(q[1]).max(q[2]) + Vec2::splat(margin)).ceil();
    if hi.x < 0.0 || hi.y < 0.0 || lo.x > s - 1.0 || lo.y > s - 1.0 {
        return;
    }
    let (mut lo, mut hi) = (lo.max(Vec2::ZERO), hi.min(Vec2::splat(s - 1.0)));
    if let Some((a, b)) = window {
        (lo, hi) = (lo.max(a.floor()), hi.min(b.ceil()));
    }
    if lo.x > hi.x || lo.y > hi.y {
        return;
    }
    let len = [(q[2] - q[1]).length(), (q[0] - q[2]).length(), (q[1] - q[0]).length()];
    for j in lo.y as usize..=hi.y as usize {
        for i in lo.x as usize..=hi.x as usize {
            let p = Vec2::new(i as f32 + 0.5, j as f32 + 0.5);
            let b0 = (q[2] - q[1]).perp_dot(p - q[1]) / area;
            let b1 = (q[0] - q[2]).perp_dot(p - q[2]) / area;
            let w = [b0, b1, 1.0 - b0 - b1];
            let at = (j * size + i) as u32;
            if w.iter().all(|&x| x >= -1e-4) {
                out.push(Texel { at, tri: t as u32, bary: w, inside: true });
            } else if margin > 0.0 && (0..3).all(|k| w[k] * area.abs() / len[k].max(1e-12) >= -margin) {
                let c = w.map(|x| x.max(0.0));
                let sum = c[0] + c[1] + c[2];
                if sum > 1e-9 {
                    out.push(Texel { at, tri: t as u32, bary: c.map(|x| x / sum), inside: false });
                }
            }
        }
    }
}

fn to8(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Colour with opacity as alpha.
pub fn color_pixels(texels: &[Paint]) -> Vec<[u8; 4]> {
    texels.iter().map(|p| [to8(p[0]), to8(p[1]), to8(p[2]), to8(p[5])]).collect()
}

/// Roughness in green and metalness in blue, as the baked map has them.
pub fn mr_pixels(texels: &[Paint]) -> Vec<[u8; 4]> {
    texels.iter().map(|p| [0, to8(p[3]), to8(p[4]), 255]).collect()
}

/// Glow colour (paint colour times emissive) and whether any texel glows.
pub fn emissive_pixels(texels: &[Paint]) -> (Vec<[u8; 4]>, bool) {
    let px: Vec<[u8; 4]> = texels.iter().map(|p| [to8(p[0] * p[6]), to8(p[1] * p[6]), to8(p[2] * p[6]), 255]).collect();
    let glows = px.iter().any(|g| g[0] > 0 || g[1] > 0 || g[2] > 0);
    (px, glows)
}

impl Texture {
    /// A texture of default paint with no texel marked inside; call [`Texture::mark_interior`] with the mesh.
    pub fn blank(size: usize) -> Texture {
        Texture { size, texels: vec![DEFAULT_PAINT; size * size], interior: vec![false; size * size], revision: 1 }
    }

    /// The vertex paint of `mesh` baked into texels: each UV triangle is rasterised with its corner paint interpolated, and `PAD` texels beyond its edges are filled.
    pub fn from_vertex(mesh: &Mesh, size: usize) -> Texture {
        let mut tex = Texture::blank(size);
        let mut hits = Vec::new();
        for t in 0..mesh.triangle_count() {
            hits.clear();
            rasterize(mesh, size, t, PAD, None, &mut hits);
            let ids = mesh.triangle(t);
            for h in &hits {
                let at = h.at as usize;
                if !h.inside && tex.interior[at] {
                    continue;
                }
                tex.texels[at] = if mesh.has_paint() {
                    std::array::from_fn(|k| (0..3).map(|c| mesh.paint[ids[c] as usize][k] * h.bary[c]).sum())
                } else {
                    DEFAULT_PAINT
                };
                if h.inside {
                    tex.interior[at] = true;
                }
            }
        }
        tex
    }

    /// Marks the texels whose centres lie inside a UV triangle of `mesh`.
    pub fn mark_interior(&mut self, mesh: &Mesh) {
        let mut hits = Vec::new();
        for t in 0..mesh.triangle_count() {
            hits.clear();
            rasterize(mesh, self.size, t, 0.0, None, &mut hits);
            for h in &hits {
                self.interior[h.at as usize] = true;
            }
        }
    }

    fn clamp_index(&self, uv: [f32; 2]) -> (usize, usize) {
        let s = self.size as f32;
        ((uv[0] * s).floor().clamp(0.0, s - 1.0) as usize, (uv[1] * s).floor().clamp(0.0, s - 1.0) as usize)
    }

    /// The texel under a UV point.
    pub fn at(&self, uv: [f32; 2]) -> Paint {
        let (i, j) = self.clamp_index(uv);
        self.texels[j * self.size + i]
    }

    /// The texture sampled bilinearly at a UV point.
    pub fn sample(&self, uv: [f32; 2]) -> Paint {
        let s = self.size as f32;
        let (x, y) = ((uv[0] * s - 0.5).clamp(0.0, s - 1.0), (uv[1] * s - 0.5).clamp(0.0, s - 1.0));
        let (i0, j0) = (x.floor() as usize, y.floor() as usize);
        let (i1, j1) = ((i0 + 1).min(self.size - 1), (j0 + 1).min(self.size - 1));
        let (fx, fy) = (x - i0 as f32, y - j0 as f32);
        let px = |i: usize, j: usize| self.texels[j * self.size + i];
        let (a, b, c, d) = (px(i0, j0), px(i1, j0), px(i0, j1), px(i1, j1));
        std::array::from_fn(|k| (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy)
    }

    /// The paint for texel `at` of a map `size` texels wide: the texel itself when the sizes match, else the texture sampled at its centre.
    pub fn lookup(&self, at: usize, size: usize) -> Paint {
        if size == self.size {
            return self.texels[at];
        }
        self.sample([((at % size) as f32 + 0.5) / size as f32, ((at / size) as f32 + 0.5) / size as f32])
    }

    /// The texture reduced to `size` texels a side by averaging blocks (at most the texture's own size).
    pub fn downsampled(&self, size: usize) -> Vec<Paint> {
        let size = size.clamp(1, self.size);
        if size == self.size {
            return self.texels.clone();
        }
        let n = self.size;
        let span = |i: usize| {
            let a = i * n / size;
            (a, ((i + 1) * n / size).max(a + 1))
        };
        let mut out = Vec::with_capacity(size * size);
        for j in 0..size {
            let (j0, j1) = span(j);
            for i in 0..size {
                let (i0, i1) = span(i);
                let mut sum = [0.0f32; PAINT_LEN];
                for y in j0..j1 {
                    for x in i0..i1 {
                        for (s, v) in sum.iter_mut().zip(&self.texels[y * n + x]) {
                            *s += v;
                        }
                    }
                }
                let count = ((j1 - j0) * (i1 - i0)) as f32;
                out.push(sum.map(|s| s / count));
            }
        }
        out
    }

    /// The texels as 16-bit values, one plane per paint float, for the project file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.texels.len() * PAINT_LEN * 2);
        for k in 0..PAINT_LEN {
            for p in &self.texels {
                out.extend_from_slice(&((p[k].clamp(0.0, 1.0) * 65535.0).round() as u16).to_le_bytes());
            }
        }
        out
    }

    /// Reads [`Texture::to_bytes`]; call [`Texture::mark_interior`] afterwards.
    pub fn from_bytes(size: usize, bytes: &[u8]) -> Result<Texture, String> {
        if !(TEXTURE_MIN..=TEXTURE_MAX).contains(&size) || bytes.len() != size * size * PAINT_LEN * 2 {
            return Err("a texture buffer does not match its size".into());
        }
        let mut tex = Texture::blank(size);
        let n = size * size;
        for k in 0..PAINT_LEN {
            for (i, c) in bytes[k * n * 2..(k + 1) * n * 2].as_chunks::<2>().0.iter().enumerate() {
                tex.texels[i][k] = u16::from_le_bytes(*c) as f32 / 65535.0;
            }
        }
        Ok(tex)
    }
}

/// The texels one stroke changed, with their values before and after, for undo and redo.
#[derive(Debug, Clone, Default)]
pub struct TextureEdit {
    pub texels: Vec<u32>,
    pub before: Vec<Paint>,
    pub after: Vec<Paint>,
}

impl TextureEdit {
    pub fn is_empty(&self) -> bool {
        self.texels.is_empty()
    }

    /// Writes the after values (forward) or the before values into the texture.
    pub fn apply(&self, texture: &mut Texture, forward: bool) {
        let values = if forward { &self.after } else { &self.before };
        for (k, &at) in self.texels.iter().enumerate() {
            if let Some(slot) = texture.texels.get_mut(at as usize) {
                *slot = values[k];
            }
        }
        texture.revision += 1;
    }
}
