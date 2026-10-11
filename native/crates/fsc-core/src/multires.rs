use glam::Vec3;

use crate::mesh::Mesh;
use crate::scene::Object;

#[derive(Debug, Clone, Default)]
pub struct Stencil {
    pub offsets: Vec<u32>,
    pub sources: Vec<u32>,
    pub weights: Vec<f32>,
}

impl Stencil {
    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, row: &[(u32, f32)]) {
        if self.offsets.is_empty() {
            self.offsets.push(0);
        }
        for &(s, w) in row {
            self.sources.push(s);
            self.weights.push(w);
        }
        self.offsets.push(self.sources.len() as u32);
    }

    fn row(&self, i: usize) -> impl Iterator<Item = (usize, f32)> + '_ {
        let (a, b) = (self.offsets[i] as usize, self.offsets[i + 1] as usize);
        self.sources[a..b].iter().zip(&self.weights[a..b]).map(|(&s, &w)| (s as usize, w))
    }

    pub fn apply3(&self, src: &[[f32; 3]]) -> Vec<[f32; 3]> {
        let one = |i: usize| self.row(i).fold(Vec3::ZERO, |acc, (s, w)| acc + Vec3::from_array(src[s]) * w).to_array();
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            (0..self.len()).into_par_iter().map(one).collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            (0..self.len()).map(one).collect()
        }
    }

    pub fn apply1(&self, src: &[f32]) -> Vec<f32> {
        (0..self.len()).map(|i| self.row(i).map(|(s, w)| src[s] * w).sum()).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Level {
    pub mesh: Mesh,
    pub stencil: Stencil,
    pub detail: Vec<[f32; 3]>,
    pub vertices: usize,
    pub triangles: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Multires {
    pub levels: Vec<Level>,
    pub current: usize,
}

impl Multires {
    pub fn top(&self) -> usize {
        self.levels.len().saturating_sub(1)
    }
}

struct Frames {
    normal: Vec<Vec3>,
    tangent: Vec<Vec3>,
}

fn frames(positions: &[[f32; 3]], indices: &[u32]) -> Frames {
    let n = positions.len();
    let p = |i: u32| Vec3::from_array(positions[i as usize]);
    let mut normal = vec![Vec3::ZERO; n];
    let mut reference = vec![u32::MAX; n];
    for tri in indices.chunks_exact(3) {
        let (a, b, c) = (tri[0], tri[1], tri[2]);
        let f = (p(b) - p(a)).cross(p(c) - p(a));
        for (v, next) in [(a, b), (b, c), (c, a)] {
            normal[v as usize] += f;
            if reference[v as usize] == u32::MAX {
                reference[v as usize] = next;
            }
        }
    }
    let mut tangent = Vec::with_capacity(n);
    for v in 0..n {
        let nv = normal[v].normalize_or(Vec3::Y);
        normal[v] = nv;
        let toward = if reference[v] == u32::MAX { Vec3::X } else { p(reference[v]) - p(v as u32) };
        let t = toward - nv * toward.dot(nv);
        tangent.push(t.try_normalize().unwrap_or_else(|| nv.any_orthonormal_vector()));
    }
    Frames { normal, tangent }
}

fn to_local(high: &[[f32; 3]], smooth: &[[f32; 3]], f: &Frames) -> Vec<[f32; 3]> {
    (0..high.len())
        .map(|v| {
            let d = Vec3::from_array(high[v]) - Vec3::from_array(smooth[v]);
            let (n, t) = (f.normal[v], f.tangent[v]);
            [d.dot(t), d.dot(n.cross(t)), d.dot(n)]
        })
        .collect()
}

fn from_local(smooth: &[[f32; 3]], detail: &[[f32; 3]], f: &Frames) -> Vec<[f32; 3]> {
    (0..smooth.len())
        .map(|v| {
            let s = Vec3::from_array(smooth[v]);
            let Some(d) = detail.get(v) else { return smooth[v] };
            let (n, t) = (f.normal[v], f.tangent[v]);
            (s + t * d[0] + n.cross(t) * d[1] + n * d[2]).to_array()
        })
        .collect()
}

impl Object {
    pub fn levels(&self) -> (usize, usize) {
        self.multires.as_ref().map(|m| (m.current, m.levels.len())).unwrap_or((0, 1))
    }

    pub fn divide(&mut self, upper: Mesh, stencil: Stencil) -> Result<(), String> {
        if stencil.len() != upper.vertex_count() {
            return Err("the subdivision stencil does not match the new level".into());
        }
        let multires = self.multires.get_or_insert_with(|| Multires {
            levels: vec![Level { vertices: self.mesh.vertex_count(), triangles: self.mesh.triangle_count(), ..Level::default() }],
            current: 0,
        });
        if multires.current != multires.top() {
            return Err(format!("step up to the highest level ({}) before dividing", multires.top()));
        }
        let mask_up = stencil.apply1(&self.mask);
        let cur = multires.current;
        multires.levels[cur].mesh = std::mem::replace(&mut self.mesh, upper);
        self.mask = mask_up;
        multires.levels.push(Level { vertices: self.mesh.vertex_count(), triangles: self.mesh.triangle_count(), stencil, ..Level::default() });
        multires.current += 1;
        self.rebuild();
        Ok(())
    }

    pub fn undivide(&mut self) -> Result<(Mesh, Stencil), String> {
        let multires = self.multires.as_mut().ok_or("the object has no subdivision levels")?;
        if multires.current != multires.top() || multires.current == 0 {
            return Err("only the highest level can be removed this way".into());
        }
        let top = multires.levels.pop().expect("at least two levels");
        multires.current -= 1;
        let below = &mut multires.levels[multires.current];
        let n = below.vertices;
        let upper = std::mem::replace(&mut self.mesh, std::mem::take(&mut below.mesh));
        self.mask.truncate(n);
        if multires.levels.len() == 1 {
            self.multires = None;
        }
        self.rebuild();
        Ok((upper, top.stencil))
    }

    pub fn set_level(&mut self, target: usize) -> Result<usize, String> {
        let multires = self.multires.as_mut().ok_or("the object has no subdivision levels; divide it first")?;
        if target > multires.top() {
            return Err(format!("level {target} does not exist; the highest is {}", multires.top()));
        }
        let from = multires.current;
        while multires.current > target {
            let cur = multires.current;
            let lower = std::mem::take(&mut multires.levels[cur - 1].mesh);
            let smooth = multires.levels[cur].stencil.apply3(&lower.positions);
            let f = frames(&smooth, &self.mesh.indices);
            multires.levels[cur].detail = to_local(&self.mesh.positions, &smooth, &f);
            let low_mask = self.mask[..lower.vertex_count()].to_vec();
            self.mask = low_mask;
            multires.levels[cur].mesh = std::mem::replace(&mut self.mesh, lower);
            multires.current -= 1;
        }
        while multires.current < target {
            let up = multires.current + 1;
            let mut upper = std::mem::take(&mut multires.levels[up].mesh);
            let smooth = multires.levels[up].stencil.apply3(&self.mesh.positions);
            let f = frames(&smooth, &upper.indices);
            upper.positions = from_local(&smooth, &multires.levels[up].detail, &f);
            upper.recompute_normals();
            multires.levels[up].detail = Vec::new();
            let mask_up = multires.levels[up].stencil.apply1(&self.mask);
            self.mask = mask_up;
            multires.levels[up - 1].mesh = std::mem::replace(&mut self.mesh, upper);
            multires.current = up;
        }
        if from != target {
            self.rebuild();
        }
        Ok(from)
    }

    pub fn delete_higher(&mut self) -> Result<usize, String> {
        let multires = self.multires.as_mut().ok_or("the object has no subdivision levels")?;
        let removed = multires.top() - multires.current;
        multires.levels.truncate(multires.current + 1);
        if multires.levels.len() == 1 {
            self.multires = None;
        }
        self.revision += 1;
        Ok(removed)
    }

    pub fn delete_lower(&mut self) -> Result<usize, String> {
        let multires = self.multires.as_mut().ok_or("the object has no subdivision levels")?;
        let removed = multires.current;
        multires.levels.drain(0..removed);
        multires.levels[0].stencil = Stencil::default();
        multires.levels[0].detail = Vec::new();
        multires.current = 0;
        if multires.levels.len() == 1 {
            self.multires = None;
        }
        self.revision += 1;
        Ok(removed)
    }
}
