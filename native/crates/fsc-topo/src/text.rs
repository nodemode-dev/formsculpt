use fsc_core::{Aabb, Mesh};
use glam::{Vec2, Vec3};

use crate::voxel::implicit_mesh;

type Glyph = &'static [&'static [(f32, f32)]];

const O: &[(f32, f32)] = &[(1.0, 0.0), (0.0, 1.0), (0.0, 5.0), (1.0, 6.0), (3.0, 6.0), (4.0, 5.0), (4.0, 1.0), (3.0, 0.0), (1.0, 0.0)];
const P: &[(f32, f32)] = &[(0.0, 0.0), (0.0, 6.0), (3.0, 6.0), (4.0, 5.0), (4.0, 4.0), (3.0, 3.0), (0.0, 3.0)];
const DOT: &[(f32, f32)] = &[(2.0, 0.0), (2.0, 0.2)];

/// A single-stroke font on a grid 4 wide and 6 tall: each glyph is polylines swept by a round pen, so the letters come
/// out as one rounded solid. Letters are capitals (lower case is drawn as capitals), digits and common punctuation.
fn glyph(c: char) -> Option<Glyph> {
    Some(match c.to_ascii_uppercase() {
        'A' => &[&[(0.0, 0.0), (2.0, 6.0), (4.0, 0.0)], &[(0.8, 2.4), (3.2, 2.4)]],
        'B' => &[&[(0.0, 0.0), (0.0, 6.0), (3.0, 6.0), (4.0, 5.0), (4.0, 4.0), (3.0, 3.0), (0.0, 3.0)], &[(3.0, 3.0), (4.0, 2.0), (4.0, 1.0), (3.0, 0.0), (0.0, 0.0)]],
        'C' => &[&[(4.0, 5.0), (3.0, 6.0), (1.0, 6.0), (0.0, 5.0), (0.0, 1.0), (1.0, 0.0), (3.0, 0.0), (4.0, 1.0)]],
        'D' => &[&[(0.0, 0.0), (0.0, 6.0), (2.5, 6.0), (4.0, 4.5), (4.0, 1.5), (2.5, 0.0), (0.0, 0.0)]],
        'E' => &[&[(4.0, 6.0), (0.0, 6.0), (0.0, 0.0), (4.0, 0.0)], &[(0.0, 3.0), (3.0, 3.0)]],
        'F' => &[&[(4.0, 6.0), (0.0, 6.0), (0.0, 0.0)], &[(0.0, 3.0), (3.0, 3.0)]],
        'G' => &[&[(4.0, 5.0), (3.0, 6.0), (1.0, 6.0), (0.0, 5.0), (0.0, 1.0), (1.0, 0.0), (3.0, 0.0), (4.0, 1.0), (4.0, 3.0), (2.0, 3.0)]],
        'H' => &[&[(0.0, 0.0), (0.0, 6.0)], &[(4.0, 0.0), (4.0, 6.0)], &[(0.0, 3.0), (4.0, 3.0)]],
        'I' => &[&[(2.0, 0.0), (2.0, 6.0)], &[(1.0, 6.0), (3.0, 6.0)], &[(1.0, 0.0), (3.0, 0.0)]],
        'J' => &[&[(4.0, 6.0), (4.0, 1.0), (3.0, 0.0), (1.0, 0.0), (0.0, 1.0)]],
        'K' => &[&[(0.0, 0.0), (0.0, 6.0)], &[(4.0, 6.0), (0.0, 2.0)], &[(1.33, 3.33), (4.0, 0.0)]],
        'L' => &[&[(0.0, 6.0), (0.0, 0.0), (4.0, 0.0)]],
        'M' => &[&[(0.0, 0.0), (0.0, 6.0), (2.0, 3.0), (4.0, 6.0), (4.0, 0.0)]],
        'N' => &[&[(0.0, 0.0), (0.0, 6.0), (4.0, 0.0), (4.0, 6.0)]],
        'O' => &[O],
        'P' => &[P],
        'Q' => &[O, &[(2.5, 1.5), (4.2, -0.3)]],
        'R' => &[P, &[(2.0, 3.0), (4.0, 0.0)]],
        'S' => &[&[(4.0, 5.0), (3.0, 6.0), (1.0, 6.0), (0.0, 5.0), (0.0, 4.0), (1.0, 3.0), (3.0, 3.0), (4.0, 2.0), (4.0, 1.0), (3.0, 0.0), (1.0, 0.0), (0.0, 1.0)]],
        'T' => &[&[(0.0, 6.0), (4.0, 6.0)], &[(2.0, 6.0), (2.0, 0.0)]],
        'U' => &[&[(0.0, 6.0), (0.0, 1.0), (1.0, 0.0), (3.0, 0.0), (4.0, 1.0), (4.0, 6.0)]],
        'V' => &[&[(0.0, 6.0), (2.0, 0.0), (4.0, 6.0)]],
        'W' => &[&[(0.0, 6.0), (1.0, 0.0), (2.0, 4.0), (3.0, 0.0), (4.0, 6.0)]],
        'X' => &[&[(0.0, 0.0), (4.0, 6.0)], &[(0.0, 6.0), (4.0, 0.0)]],
        'Y' => &[&[(0.0, 6.0), (2.0, 3.0), (4.0, 6.0)], &[(2.0, 3.0), (2.0, 0.0)]],
        'Z' => &[&[(0.0, 6.0), (4.0, 6.0), (0.0, 0.0), (4.0, 0.0)]],
        '0' => &[O, &[(3.5, 5.0), (0.5, 1.0)]],
        '1' => &[&[(1.0, 5.0), (2.0, 6.0), (2.0, 0.0)], &[(1.0, 0.0), (3.0, 0.0)]],
        '2' => &[&[(0.0, 5.0), (1.0, 6.0), (3.0, 6.0), (4.0, 5.0), (4.0, 4.0), (0.0, 0.0), (4.0, 0.0)]],
        '3' => &[&[(0.0, 5.0), (1.0, 6.0), (3.0, 6.0), (4.0, 5.0), (4.0, 4.0), (3.0, 3.0), (1.5, 3.0)], &[(3.0, 3.0), (4.0, 2.0), (4.0, 1.0), (3.0, 0.0), (1.0, 0.0), (0.0, 1.0)]],
        '4' => &[&[(3.0, 0.0), (3.0, 6.0), (0.0, 2.0), (4.0, 2.0)]],
        '5' => &[&[(4.0, 6.0), (0.0, 6.0), (0.0, 3.5), (3.0, 3.5), (4.0, 2.5), (4.0, 1.0), (3.0, 0.0), (1.0, 0.0), (0.0, 1.0)]],
        '6' => &[&[(4.0, 5.0), (3.0, 6.0), (1.0, 6.0), (0.0, 5.0), (0.0, 1.0), (1.0, 0.0), (3.0, 0.0), (4.0, 1.0), (4.0, 2.0), (3.0, 3.0), (0.0, 3.0)]],
        '7' => &[&[(0.0, 6.0), (4.0, 6.0), (1.5, 0.0)]],
        '8' => &[&[(1.0, 3.0), (0.0, 4.0), (0.0, 5.0), (1.0, 6.0), (3.0, 6.0), (4.0, 5.0), (4.0, 4.0), (3.0, 3.0), (1.0, 3.0), (0.0, 2.0), (0.0, 1.0), (1.0, 0.0), (3.0, 0.0), (4.0, 1.0), (4.0, 2.0), (3.0, 3.0)]],
        '9' => &[&[(4.0, 3.0), (1.0, 3.0), (0.0, 4.0), (0.0, 5.0), (1.0, 6.0), (3.0, 6.0), (4.0, 5.0), (4.0, 1.0), (3.0, 0.0), (1.0, 0.0), (0.0, 1.0)]],
        '.' => &[DOT],
        ',' => &[&[(2.0, 0.3), (1.5, -0.8)]],
        ':' => &[DOT, &[(2.0, 3.8), (2.0, 4.0)]],
        '-' => &[&[(1.0, 3.0), (3.0, 3.0)]],
        '+' => &[&[(2.0, 1.5), (2.0, 4.5)], &[(0.5, 3.0), (3.5, 3.0)]],
        '!' => &[&[(2.0, 6.0), (2.0, 2.0)], DOT],
        '?' => &[&[(0.0, 5.0), (1.0, 6.0), (3.0, 6.0), (4.0, 5.0), (4.0, 4.0), (2.0, 3.0), (2.0, 2.0)], DOT],
        '\'' => &[&[(2.0, 6.0), (2.0, 4.5)]],
        '/' => &[&[(0.0, 0.0), (4.0, 6.0)]],
        ' ' => &[],
        _ => return None,
    })
}

/// The characters the stroke font draws.
pub const TEXT_CHARACTERS: &str = "A-Z, 0-9, space and . , : - + ! ? ' /";

fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let e = b - a;
    let t = ((p - a).dot(e) / e.length_squared().max(1e-12)).clamp(0.0, 1.0);
    (p - (a + e * t)).length()
}

/// The field of a 2D shape (negative inside) pushed out to `depth` along z through the middle, with its edges rounded
/// by `round` (never more than half the depth).
fn extruded(d2: f32, z: f32, depth: f32, round: f32) -> f32 {
    let r = round.min(depth * 0.5).max(0.0);
    let w = Vec2::new(d2 + r, z.abs() - depth * 0.5 + r);
    w.x.max(w.y).min(0.0) + w.max(Vec2::ZERO).length() - r
}

/// Cells along the longest side, so the finest feature still spans about six cells.
fn cells_for(longest: f32, finest: f32, resolution: Option<u32>) -> u32 {
    resolution.unwrap_or_else(|| (longest / (finest / 6.0)).ceil() as u32).clamp(48, 512)
}

/// Solid 3D text: `text` in the stroke font with capitals `size` tall, strokes `weight` of the size wide, `depth` deep
/// along z, centred on the origin and facing +z. Lines break at newlines. Watertight, rounded at every edge.
pub fn text_mesh(text: &str, size: f32, weight: f32, depth: f32, resolution: Option<u32>) -> Result<Mesh, String> {
    let s = size / 6.0;
    let mut segments: Vec<(Vec2, Vec2)> = Vec::new();
    for (line, row) in text.lines().enumerate() {
        for (col, c) in row.chars().enumerate() {
            let g = glyph(c).ok_or_else(|| format!("the font has no {c:?}; it draws {TEXT_CHARACTERS}"))?;
            let origin = Vec2::new(col as f32 * 5.6, -(line as f32) * 9.0);
            for stroke in g {
                for w in stroke.windows(2) {
                    segments.push(((origin + Vec2::new(w[0].0, w[0].1)) * s, (origin + Vec2::new(w[1].0, w[1].1)) * s));
                }
            }
        }
    }
    if segments.is_empty() {
        return Err("the text has nothing to draw".into());
    }
    let half_w = weight * size * 0.5;
    let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
    for (a, b) in &segments {
        lo = lo.min(a.min(*b));
        hi = hi.max(a.max(*b));
    }
    let centre = (lo + hi) * 0.5;
    for (a, b) in segments.iter_mut() {
        *a -= centre;
        *b -= centre;
    }
    let ext = (hi - lo) * 0.5 + Vec2::splat(half_w * 1.5);
    let mut bounds = Aabb::EMPTY;
    bounds.grow(Vec3::new(-ext.x, -ext.y, -depth * 0.75));
    bounds.grow(Vec3::new(ext.x, ext.y, depth * 0.75));
    let longest = (ext.x * 2.0).max(ext.y * 2.0).max(depth);
    let cells = cells_for(longest, (half_w * 2.0).min(depth), resolution);
    let field = |p: Vec3| -> f32 {
        let q = Vec2::new(p.x, p.y);
        let d = segments.iter().map(|(a, b)| segment_distance(q, *a, *b)).fold(f32::INFINITY, f32::min) - half_w;
        extruded(d, p.z, depth, half_w * 0.5)
    };
    let (mut mesh, _) = crate::manifold::split_nonmanifold(&implicit_mesh(bounds, cells, field)?);
    if mesh.indices.is_empty() {
        return Err("the text came out empty; raise weight or resolution".into());
    }
    mesh.quads = true;
    Ok(mesh)
}

/// A vector shape: closed outlines in the x, y plane (any inside another is a hole, by the even-odd rule) pushed out to
/// `depth` along z through the middle, with edges rounded by `round` world units. Watertight.
pub fn extrude_shape(outlines: &[Vec<Vec2>], depth: f32, round: f32, resolution: Option<u32>) -> Result<Mesh, String> {
    if outlines.is_empty() || outlines.iter().any(|o| o.len() < 3) {
        return Err("give at least one outline of three or more points".into());
    }
    let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
    for p in outlines.iter().flatten() {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let span = (hi - lo).max_element().max(1e-4);
    let pad = span * 0.04 + round;
    let mut bounds = Aabb::EMPTY;
    bounds.grow(Vec3::new(lo.x - pad, lo.y - pad, -depth * 0.75));
    bounds.grow(Vec3::new(hi.x + pad, hi.y + pad, depth * 0.75));
    let cells = cells_for(span.max(depth), (span / 40.0).min(depth), resolution);
    let field = |p: Vec3| -> f32 {
        let q = Vec2::new(p.x, p.y);
        let (mut d, mut inside) = (f32::INFINITY, false);
        for poly in outlines {
            let n = poly.len();
            for i in 0..n {
                let (a, b) = (poly[i], poly[(i + 1) % n]);
                d = d.min(segment_distance(q, a, b));
                if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                    inside = !inside;
                }
            }
        }
        extruded(if inside { -d } else { d }, p.z, depth, round)
    };
    let (mut mesh, _) = crate::manifold::split_nonmanifold(&implicit_mesh(bounds, cells, field)?);
    if mesh.indices.is_empty() {
        return Err("the outlines enclose nothing".into());
    }
    mesh.quads = true;
    Ok(mesh)
}
