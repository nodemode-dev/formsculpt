use glam::Vec3;

/// A vector displacement map: an offset in three directions for every point of a square stamp, so a stamp can lift a
/// surface sideways and back over itself (horns that curl, mushroom caps, ears), which a gray alpha cannot. Offsets are
/// (across, out, along) in units of the stamp's half-width, on a square grid of nodes from -1 to 1 on both axes.
#[derive(Debug, Clone, PartialEq)]
pub struct Vdm {
    pub size: usize,
    pub data: Vec<[f32; 3]>,
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Vdm {
    pub const BUILTIN: [&'static str; 3] = ["horn", "hook", "mushroom"];

    pub fn from_fn(size: usize, f: impl Fn(f32, f32) -> [f32; 3]) -> Vdm {
        let mut data = Vec::with_capacity(size * size);
        for j in 0..size {
            for i in 0..size {
                let x = i as f32 / (size - 1) as f32 * 2.0 - 1.0;
                let y = j as f32 / (size - 1) as f32 * 2.0 - 1.0;
                data.push(f(x, y));
            }
        }
        Vdm { size, data }
    }

    /// The built-in maps, generated rather than shipped. Each one overhangs, which is what sets a vector map apart.
    pub fn builtin(name: &str) -> Option<Vdm> {
        // A cone of `base` radius rising to `height` whose tip leans across by `lean` (its square keeps the base
        // upright and bends the tip, so the tip passes over the base's edge).
        let cone = |base: f32, height: f32, lean: f32, power: f32| {
            move |x: f32, y: f32| {
                let r = (x * x + y * y).sqrt();
                if r >= base {
                    return [0.0; 3];
                }
                let t = 1.0 - r / base;
                let h = height * t.powf(1.5);
                let s = 0.6 * t;
                let bend = lean * (h / height).powf(power);
                [-x * s + bend, h, -y * s]
            }
        };
        Some(match name {
            "horn" => Vdm::from_fn(129, cone(0.6, 1.6, 0.7, 2.0)),
            "hook" => Vdm::from_fn(129, cone(0.55, 1.4, 1.3, 2.5)),
            "mushroom" => Vdm::from_fn(129, |x, y| {
                let r = (x * x + y * y).sqrt();
                if r >= 1.0 {
                    return [0.0; 3];
                }
                // The flat disc maps outward from the middle onto the cap's top, round its rim, back in under it, down
                // the stem and out over the ground, so the cap overhangs the stem.
                let (big_r, h) = if r < 0.45 {
                    let t = r / 0.45;
                    (0.75 * t, 0.95 - 0.25 * t * t)
                } else if r < 0.6 {
                    let t = (r - 0.45) / 0.15;
                    (0.75 - 0.05 * t, 0.7 - 0.15 * t)
                } else if r < 0.72 {
                    let t = (r - 0.6) / 0.12;
                    (0.7 - 0.52 * t, 0.55)
                } else if r < 0.88 {
                    let t = (r - 0.72) / 0.16;
                    (0.18, 0.55 * (1.0 - t))
                } else {
                    let t = (r - 0.88) / 0.12;
                    (0.18 + 0.82 * t, 0.0)
                };
                let (dx, dy) = if r > 1e-6 { (x / r, y / r) } else { (0.0, 0.0) };
                [dx * (big_r - r), h, dy * (big_r - r)]
            }),
            _ => return None,
        })
    }

    /// Captures a map from a plane made with `divisions` cells of `size` (the plane primitive's grid: x across, y up,
    /// rows running toward -z) and then sculpted. Each vertex's move from its flat place becomes the offset; the mean move
    /// of the four corners is taken off first, so a plane that was moved as a whole still captures only its sculpting.
    pub fn capture(positions: &[[f32; 3]], divisions: usize, size: f32) -> Result<Vdm, String> {
        let n = divisions;
        if n < 2 || positions.len() != (n + 1) * (n + 1) {
            return Err("capture needs a plane from add_primitive whose vertices were moved but not added or removed".into());
        }
        let h = size / 2.0;
        let flat = |i: usize, j: usize| Vec3::new(-h + size * i as f32 / n as f32, 0.0, h - size * j as f32 / n as f32);
        let moved = |i: usize, j: usize| Vec3::from_array(positions[j * (n + 1) + i]) - flat(i, j);
        let shift = (moved(0, 0) + moved(n, 0) + moved(0, n) + moved(n, n)) / 4.0;
        let scale = 2.0 / size;
        let mut data = Vec::with_capacity((n + 1) * (n + 1));
        for j in 0..=n {
            for i in 0..=n {
                let d = (moved(i, j) - shift) * scale;
                data.push([d.x, d.y, -d.z]);
            }
        }
        Ok(Vdm { size: n + 1, data })
    }

    /// Bilinear sample at (x, y) from -1 to 1; zero outside.
    pub fn sample(&self, x: f32, y: f32) -> Vec3 {
        if !(-1.0..=1.0).contains(&x) || !(-1.0..=1.0).contains(&y) {
            return Vec3::ZERO;
        }
        let m = (self.size - 1) as f32;
        let fx = (x + 1.0) * 0.5 * m;
        let fy = (y + 1.0) * 0.5 * m;
        let (i, j) = ((fx.floor() as usize).min(self.size - 2), (fy.floor() as usize).min(self.size - 2));
        let (tx, ty) = (fx - i as f32, fy - j as f32);
        let at = |i: usize, j: usize| Vec3::from_array(self.data[j * self.size + i]);
        let a = at(i, j).lerp(at(i + 1, j), tx);
        let b = at(i, j + 1).lerp(at(i + 1, j + 1), tx);
        a.lerp(b, ty)
    }

    /// The largest offset in the map, in half-widths.
    pub fn reach(&self) -> f32 {
        self.data.iter().map(|d| Vec3::from_array(*d).length()).fold(0.0, f32::max)
    }
}

/// Where a stamp sits: its centre on the surface, the surface normal there, its half-width and its turn (radians)
/// about the normal.
#[derive(Debug, Clone, Copy)]
pub struct VdmPlacement {
    pub center: Vec3,
    pub normal: Vec3,
    pub radius: f32,
    pub angle: f32,
    pub strength: f32,
}

/// Moves every vertex under the stamp's square by the map's offset in the stamp's frame (across, out along the normal,
/// along), held back by the mask and faded over the outer tenth of the square so a captured map's edges blend. Only
/// vertices within half a radius of the surface plane are moved, so a thin part's far side is left alone. Returns the
/// new positions and how many vertices moved.
pub fn vdm_stamp(positions: &[[f32; 3]], mask: &[f32], vdm: &Vdm, at: VdmPlacement) -> (Vec<[f32; 3]>, usize) {
    let n = at.normal.normalize_or(Vec3::Y);
    let seed = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
    let u0 = (seed - n * seed.dot(n)).normalize();
    let v0 = n.cross(u0);
    let (sin, cos) = at.angle.sin_cos();
    let u = u0 * cos + v0 * sin;
    let v = n.cross(u);
    let mut out = positions.to_vec();
    let mut moved = 0;
    for (k, p) in positions.iter().enumerate() {
        let local = Vec3::from_array(*p) - at.center;
        let (a, b, h) = (local.dot(u) / at.radius, local.dot(v) / at.radius, local.dot(n) / at.radius);
        if a.abs() > 1.0 || b.abs() > 1.0 || h.abs() > 0.5 {
            continue;
        }
        let keep = 1.0 - mask.get(k).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        let fade = 1.0 - smooth(0.9, 1.0, a.abs().max(b.abs()));
        let d = vdm.sample(a, b) * (at.radius * at.strength * keep * fade);
        if d.length_squared() > 0.0 {
            out[k] = (Vec3::from_array(*p) + u * d.x + n * d.y + v * d.z).to_array();
            moved += 1;
        }
    }
    (out, moved)
}
