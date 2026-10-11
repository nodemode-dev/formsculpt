use glam::Vec3;

use crate::scene::Object;

#[derive(Debug, Clone, Default)]
pub struct Layer {
    pub name: String,
    pub strength: f32,
    pub delta: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, Default)]
pub struct Layers {
    pub list: Vec<Layer>,
    pub active: Option<usize>,
}

impl Object {
    fn shift(&mut self, delta: &[[f32; 3]], by: f32) {
        if by == 0.0 {
            return;
        }
        for (p, d) in self.mesh.positions.iter_mut().zip(delta) {
            *p = (Vec3::from_array(*p) + Vec3::from_array(*d) * by).to_array();
        }
        self.mesh.recompute_normals();
        self.bvh.refit(&self.mesh);
        self.revision += 1;
    }

    pub fn add_layer(&mut self, name: String) -> usize {
        let n = self.mesh.vertex_count();
        self.layers.list.push(Layer { name, strength: 1.0, delta: vec![[0.0; 3]; n] });
        let at = self.layers.list.len() - 1;
        self.layers.active = Some(at);
        self.revision += 1;
        at
    }

    pub fn set_layer_strength(&mut self, at: usize, strength: f32) -> Result<f32, String> {
        let layer = self.layers.list.get_mut(at).ok_or("no layer at that index")?;
        let old = layer.strength;
        layer.strength = strength;
        let delta = std::mem::take(&mut layer.delta);
        self.shift(&delta, strength - old);
        self.layers.list[at].delta = delta;
        Ok(old)
    }

    pub fn remove_layer(&mut self, at: usize, keep_shape: bool) -> Result<Layer, String> {
        if at >= self.layers.list.len() {
            return Err("no layer at that index".into());
        }
        let layer = self.layers.list.remove(at);
        if !keep_shape {
            self.shift(&layer.delta, -layer.strength);
        } else {
            self.revision += 1;
        }
        self.layers.active = match self.layers.active {
            Some(a) if a == at => None,
            Some(a) if a > at => Some(a - 1),
            other => other,
        };
        Ok(layer)
    }

    pub fn record_into_layer(&mut self, at: usize, indices: &[u32], before: &[[f32; 3]], after: &[[f32; 3]], sign: f32) {
        let Some(layer) = self.layers.list.get_mut(at) else { return };
        let s = layer.strength;
        if s.abs() < 1e-4 {
            return;
        }
        for (k, &v) in indices.iter().enumerate() {
            let d = (Vec3::from_array(after[k]) - Vec3::from_array(before[k])) * (sign / s);
            if let Some(slot) = layer.delta.get_mut(v as usize) {
                *slot = (Vec3::from_array(*slot) + d).to_array();
            }
        }
    }
}
