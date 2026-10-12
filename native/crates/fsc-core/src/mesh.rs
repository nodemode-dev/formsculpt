use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb { min: Vec3::splat(f32::INFINITY), max: Vec3::splat(f32::NEG_INFINITY) };

    pub fn grow(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn merge(&self, other: &Aabb) -> Aabb {
        Aabb { min: self.min.min(other.min), max: self.max.max(other.max) }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn extent(&self) -> Vec3 {
        (self.max - self.min).max(Vec3::ZERO)
    }

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x
    }

    pub fn surface_area(&self) -> f32 {
        let e = self.extent();
        2.0 * (e.x * e.y + e.y * e.z + e.z * e.x)
    }

    pub fn ray_entry(&self, origin: Vec3, inv_dir: Vec3, t_max: f32) -> Option<f32> {
        let t0 = (self.min - origin) * inv_dir;
        let t1 = (self.max - origin) * inv_dir;
        let near = t0.min(t1).max_element().max(0.0);
        let far = t0.max(t1).min_element().min(t_max);
        (near <= far).then_some(near)
    }

    pub fn distance_sq_to(&self, p: Vec3) -> f32 {
        let d = (self.min - p).max(p - self.max).max(Vec3::ZERO);
        d.length_squared()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub paint: Vec<Paint>,
    pub uvs: Vec<[f32; 2]>,
    pub quads: bool,
    /// Face group per triangle (empty: none). Valid only while its length matches the triangle count; anything that
    /// changes the triangles in place without carrying it leaves it stale, and every reader then treats it as absent.
    pub groups: Vec<u32>,
    /// Hidden flag per triangle (empty: none), under the same rule as `groups`.
    pub hidden: Vec<bool>,
}

/// Floats per painted vertex: red, green, blue, roughness, metalness, opacity, emissive.
pub const PAINT_LEN: usize = 7;
/// Paint channels as painted: colour (the three colour floats together), roughness, metalness, opacity, emissive.
pub const PAINT_CHANNELS: usize = 5;
/// Paint per vertex in that order; projects from before 0.26.0 hold five floats and open opaque without glow.
pub type Paint = [f32; PAINT_LEN];

pub const DEFAULT_PAINT: Paint = [0.8, 0.8, 0.8, 0.5, 0.0, 1.0, 0.0];

/// The channel a paint float belongs to (0 colour, then roughness, metalness, opacity, emissive).
pub fn paint_channel(float: usize) -> usize {
    if float < 3 {
        0
    } else {
        float - 2
    }
}

/// Paint from its first five floats (colour, roughness, metalness), opaque and without glow.
pub fn paint_from5(p: &[f32]) -> Paint {
    let mut out = DEFAULT_PAINT;
    out[..5].copy_from_slice(&p[..5]);
    out
}

pub fn mix_paint(a: Paint, b: Paint, t: f32) -> Paint {
    let mut out = a;
    for k in 0..PAINT_LEN {
        out[k] = a[k] + (b[k] - a[k]) * t;
    }
    out
}

impl Mesh {
    pub fn new(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
        let mut mesh = Mesh { normals: vec![[0.0; 3]; positions.len()], positions, indices, paint: Vec::new(), uvs: Vec::new(), quads: false, groups: Vec::new(), hidden: Vec::new() };
        mesh.recompute_normals();
        mesh
    }

    pub fn quad_faces(&self) -> Option<Vec<[u32; 4]>> {
        if !self.quads || self.indices.len() % 6 != 0 {
            return None;
        }
        self.indices
            .chunks_exact(6)
            .map(|q| {
                let (t1, t2) = ([q[0], q[1], q[2]], [q[3], q[4], q[5]]);
                let [a, b, c] = t1;
                let d = t2.into_iter().find(|v| !t1.contains(v))?;
                let has = |x: u32, y: u32| t2.contains(&x) && t2.contains(&y);
                if has(a, b) {
                    Some([a, d, b, c])
                } else if has(b, c) {
                    Some([a, b, d, c])
                } else if has(c, a) {
                    Some([a, b, c, d])
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn has_paint(&self) -> bool {
        !self.paint.is_empty()
    }

    pub fn has_groups(&self) -> bool {
        !self.groups.is_empty() && self.groups.len() == self.triangle_count()
    }

    pub fn has_hidden(&self) -> bool {
        self.hidden.len() == self.triangle_count() && self.hidden.iter().any(|&h| h)
    }

    pub fn face_hidden(&self, t: usize) -> bool {
        self.hidden.len() == self.triangle_count() && self.hidden[t]
    }

    /// Per vertex: true when every triangle around it is hidden (such vertices are held still, like a full mask).
    pub fn hidden_vertices(&self) -> Vec<bool> {
        let mut seen = vec![false; self.vertex_count()];
        let mut shown = vec![false; self.vertex_count()];
        for t in 0..self.triangle_count() {
            let h = self.face_hidden(t);
            for v in self.triangle(t) {
                seen[v as usize] = true;
                if !h {
                    shown[v as usize] = true;
                }
            }
        }
        seen.iter().zip(&shown).map(|(&s, &v)| s && !v).collect()
    }

    pub fn groups_bytes(&self) -> Vec<u8> {
        if self.has_groups() { self.groups.iter().flat_map(|g| g.to_le_bytes()).collect() } else { Vec::new() }
    }

    pub fn hidden_bytes(&self) -> Vec<u8> {
        if self.has_hidden() { self.hidden.iter().map(|&h| h as u8).collect() } else { Vec::new() }
    }

    pub fn ensure_paint(&mut self) {
        if self.paint.len() != self.positions.len() {
            self.paint.resize(self.positions.len(), DEFAULT_PAINT);
        }
    }

    pub fn paint_bytes(&self) -> Vec<u8> {
        self.paint.iter().flat_map(|p| p.iter().flat_map(|c| c.to_le_bytes())).collect()
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn position(&self, i: u32) -> Vec3 {
        Vec3::from_array(self.positions[i as usize])
    }

    pub fn triangle(&self, t: usize) -> [u32; 3] {
        [self.indices[3 * t], self.indices[3 * t + 1], self.indices[3 * t + 2]]
    }

    pub fn bounds(&self) -> Aabb {
        let mut b = Aabb::EMPTY;
        for p in &self.positions {
            b.grow(Vec3::from_array(*p));
        }
        b
    }

    pub fn face_normals(&self) -> Vec<Vec3> {
        let face = |t: usize| {
            let [a, b, c] = self.triangle(t);
            let (pa, pb, pc) = (self.position(a), self.position(b), self.position(c));
            (pb - pa).cross(pc - pa)
        };
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            (0..self.triangle_count()).into_par_iter().map(face).collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            (0..self.triangle_count()).map(face).collect()
        }
    }

    pub fn recompute_normals(&mut self) {
        let faces = self.face_normals();
        let mut acc = vec![Vec3::ZERO; self.positions.len()];
        for (t, n) in faces.iter().enumerate() {
            for v in self.triangle(t) {
                acc[v as usize] += *n;
            }
        }
        self.normals = acc.into_iter().map(|n| n.normalize_or(Vec3::Y).to_array()).collect();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.indices.len() % 3 != 0 {
            return Err(format!("index count {} is not a multiple of 3", self.indices.len()));
        }
        let n = self.positions.len() as u32;
        if let Some(bad) = self.indices.iter().find(|&&i| i >= n) {
            return Err(format!("index {bad} out of range for {n} vertices"));
        }
        if self.normals.len() != self.positions.len() {
            return Err("normal count differs from vertex count".into());
        }
        if !self.paint.is_empty() && self.paint.len() != self.positions.len() {
            return Err("paint count differs from vertex count".into());
        }
        if !self.uvs.is_empty() && self.uvs.len() != self.indices.len() {
            return Err("UV count differs from corner count".into());
        }
        Ok(())
    }

    pub fn positions_bytes(&self) -> Vec<u8> {
        self.positions.iter().flat_map(|p| p.iter().flat_map(|c| c.to_le_bytes())).collect()
    }

    pub fn normals_bytes(&self) -> Vec<u8> {
        self.normals.iter().flat_map(|p| p.iter().flat_map(|c| c.to_le_bytes())).collect()
    }

    pub fn indices_bytes(&self) -> Vec<u8> {
        self.indices.iter().flat_map(|i| i.to_le_bytes()).collect()
    }
}
