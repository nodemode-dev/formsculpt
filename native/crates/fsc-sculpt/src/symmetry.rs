use glam::{Mat3, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Symmetry {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub radial: u32,
    /// Local mode: mirror and turn about the object's centre (set by the command from the object's bounds) instead of
    /// the world origin.
    pub local: bool,
    /// Where the mirror planes and the radial axis meet: the world origin, or with local the object's centre.
    pub center: [f32; 3],
}

impl Symmetry {
    /// A point's copy under transform `m`, about the symmetry's centre.
    pub fn place(&self, m: Mat3, p: Vec3) -> Vec3 {
        let c = Vec3::from_array(self.center);
        c + m * (p - c)
    }

    pub fn transforms(&self) -> Vec<Mat3> {
        let mut mirrors = vec![Mat3::IDENTITY];
        for (on, axis) in [(self.x, 0), (self.y, 1), (self.z, 2)] {
            if !on {
                continue;
            }
            let mut flip = Vec3::ONE;
            flip[axis] = -1.0;
            let m = Mat3::from_diagonal(flip);
            let extra: Vec<Mat3> = mirrors.iter().map(|t| m * *t).collect();
            mirrors.extend(extra);
        }
        let count = self.radial.clamp(1, 150);
        let mut out = Vec::with_capacity(mirrors.len() * count as usize);
        for k in 0..count {
            let rot = Mat3::from_rotation_y(std::f32::consts::TAU * k as f32 / count as f32);
            for m in &mirrors {
                out.push(rot * *m);
            }
        }
        out
    }
}
