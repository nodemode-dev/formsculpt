//! The retopology patch library: flat quad layouts in a unit frame (x right, y up, counter-clockwise faces), laid onto
//! a surface by the caller. Every patch is all quads.

/// The patch kinds, as named to agents and in the window.
pub const PATCH_KINDS: &[&str] = &["grid", "strip", "eye_loop", "mouth_loop", "disc"];

/// A flat patch: points in the unit frame and quads (counter-clockwise seen from +z) over them.
#[derive(Debug, Clone, PartialEq)]
pub struct Patch {
    pub points: Vec<[f32; 2]>,
    pub quads: Vec<[u32; 4]>,
}

/// A grid of `cols` by `rows` quads over the square -1..1.
fn grid(cols: usize, rows: usize) -> Patch {
    let mut points = Vec::with_capacity((cols + 1) * (rows + 1));
    for j in 0..=rows {
        for i in 0..=cols {
            points.push([-1.0 + 2.0 * i as f32 / cols as f32, -1.0 + 2.0 * j as f32 / rows as f32]);
        }
    }
    let id = |i: usize, j: usize| (j * (cols + 1) + i) as u32;
    let mut quads = Vec::with_capacity(cols * rows);
    for j in 0..rows {
        for i in 0..cols {
            quads.push([id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)]);
        }
    }
    Patch { points, quads }
}

/// Concentric rings around the centre: ring k of `segments` points on an ellipse of half-height `aspect` times its
/// half-width, from `inner` (as a share of the outer size) out to 1, with quads between rings. With `fill` the
/// innermost ring is closed by a fan of quads around a centre point (`segments` must be even; the centre then joins
/// segments / 2 quads, a pole of that many edges).
fn rings(segments: usize, count: usize, inner: f32, aspect: f32, fill: bool) -> Patch {
    let mut points = Vec::with_capacity(segments * count + 1);
    for k in 0..count {
        let t = if count == 1 { 1.0 } else { k as f32 / (count - 1) as f32 };
        let r = inner + (1.0 - inner) * t;
        // Inner rings follow the opening's shape; the outer ring rounds toward a circle so the patch meets a grid.
        let a = aspect + (1.0 - aspect) * t * t;
        for s in 0..segments {
            let ang = s as f32 / segments as f32 * std::f32::consts::TAU;
            points.push([r * ang.cos(), r * a * ang.sin()]);
        }
    }
    let id = |k: usize, s: usize| (k * segments + s % segments) as u32;
    let mut quads = Vec::with_capacity(segments * count);
    for k in 0..count.saturating_sub(1) {
        for s in 0..segments {
            quads.push([id(k, s), id(k + 1, s), id(k + 1, s + 1), id(k, s + 1)]);
        }
    }
    if fill {
        let c = points.len() as u32;
        points.push([0.0, 0.0]);
        for s in (0..segments).step_by(2) {
            quads.push([c, id(0, s), id(0, s + 1), id(0, s + 2)]);
        }
    }
    Patch { points, quads }
}

/// One patch from the library. `a` and `b` are its two counts (grid: columns and rows; strip: length, with b ignored;
/// loops and disc: points around and rings), clamped to sensible ranges; loops and the disc round `a` up to even.
pub fn patch(kind: &str, a: usize, b: usize) -> Result<Patch, String> {
    let around = |n: usize| { let n = n.clamp(4, 64); n + n % 2 };
    match kind {
        "grid" => Ok(grid(a.clamp(1, 64), b.clamp(1, 64))),
        "strip" => {
            let mut p = grid(a.clamp(1, 64), 1);
            for q in &mut p.points {
                q[1] *= 0.25;
            }
            Ok(p)
        }
        "eye_loop" => Ok(rings(around(a), b.clamp(2, 12), 0.45, 0.55, false)),
        "mouth_loop" => Ok(rings(around(a), b.clamp(2, 12), 0.5, 0.35, false)),
        "disc" => Ok(rings(around(a), b.clamp(1, 12), 0.35, 1.0, true)),
        other => Err(format!("unknown patch {other}; use one of {}", PATCH_KINDS.join(", "))),
    }
}
