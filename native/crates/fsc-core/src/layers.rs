use glam::Vec3;

use crate::mesh::Paint;
use crate::scene::Object;

#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub strength: f32,
    pub delta: Vec<[f32; 3]>,
    /// Per vertex, per paint float: the difference the layer made to the paint, when it recorded any.
    pub paint: Option<Vec<Paint>>,
    /// Per vertex: the difference the layer made to the mask, when it recorded any.
    pub mask: Option<Vec<f32>>,
    pub shape_factor: f32,
    pub paint_factor: f32,
    pub mask_factor: f32,
}

impl Default for Layer {
    fn default() -> Self {
        Layer { name: String::new(), strength: 0.0, delta: Vec::new(), paint: None, mask: None, shape_factor: 1.0, paint_factor: 1.0, mask_factor: 1.0 }
    }
}

impl Layer {
    /// Strength times the shape factor: how much of the shape delta shows.
    pub fn shape_scale(&self) -> f32 {
        self.strength * self.shape_factor
    }

    /// Strength times the paint factor: how much of the paint delta shows.
    pub fn paint_scale(&self) -> f32 {
        self.strength * self.paint_factor
    }

    /// Strength times the mask factor: how much of the mask delta shows.
    pub fn mask_scale(&self) -> f32 {
        self.strength * self.mask_factor
    }
}

#[derive(Debug, Clone, Default)]
pub struct Layers {
    pub list: Vec<Layer>,
    pub active: Option<usize>,
}

impl Object {
    /// Moves every vertex by `delta` times `by`, then refreshes normals and the BVH (a layer's strength change).
    pub fn shift(&mut self, delta: &[[f32; 3]], by: f32) {
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
        self.layers.list.push(Layer { name, strength: 1.0, delta: vec![[0.0; 3]; n], ..Layer::default() });
        let at = self.layers.list.len() - 1;
        self.layers.active = Some(at);
        self.revision += 1;
        at
    }

    /// Moves the paint by `delta` times `by`, held to 0..1 per float.
    pub fn shift_paint(&mut self, delta: &[Paint], by: f32) {
        if by == 0.0 || delta.len() != self.mesh.vertex_count() {
            return;
        }
        self.mesh.ensure_paint();
        for (p, d) in self.mesh.paint.iter_mut().zip(delta) {
            for k in 0..p.len() {
                p[k] = (p[k] + d[k] * by).clamp(0.0, 1.0);
            }
        }
        self.revision += 1;
    }

    /// Moves the mask by `delta` times `by`, held to 0..1.
    pub fn shift_mask(&mut self, delta: &[f32], by: f32) {
        if by == 0.0 || delta.len() != self.mask.len() {
            return;
        }
        for (m, d) in self.mask.iter_mut().zip(delta) {
            *m = (*m + d * by).clamp(0.0, 1.0);
        }
        self.revision += 1;
    }

    pub fn set_layer_strength(&mut self, at: usize, strength: f32) -> Result<f32, String> {
        let old = self.layers.list.get(at).ok_or("no layer at that index")?.strength;
        self.set_layer(at, Some(strength), None, None, None)?;
        Ok(old)
    }

    /// Changes a layer's strength and per-channel factors, moving shape, paint and mask by the change in what shows.
    pub fn set_layer(&mut self, at: usize, strength: Option<f32>, shape: Option<f32>, paint: Option<f32>, mask: Option<f32>) -> Result<(), String> {
        let layer = self.layers.list.get_mut(at).ok_or("no layer at that index")?;
        let (os, op, om) = (layer.shape_scale(), layer.paint_scale(), layer.mask_scale());
        layer.strength = strength.unwrap_or(layer.strength);
        layer.shape_factor = shape.unwrap_or(layer.shape_factor);
        layer.paint_factor = paint.unwrap_or(layer.paint_factor);
        layer.mask_factor = mask.unwrap_or(layer.mask_factor);
        let (ns, np, nm) = (layer.shape_scale(), layer.paint_scale(), layer.mask_scale());
        let delta = std::mem::take(&mut layer.delta);
        let pdelta = layer.paint.take();
        let mdelta = layer.mask.take();
        self.shift(&delta, ns - os);
        if let Some(p) = &pdelta {
            self.shift_paint(p, np - op);
        }
        if let Some(m) = &mdelta {
            self.shift_mask(m, nm - om);
        }
        let layer = &mut self.layers.list[at];
        layer.delta = delta;
        layer.paint = pdelta;
        layer.mask = mdelta;
        Ok(())
    }

    pub fn remove_layer(&mut self, at: usize, keep_shape: bool) -> Result<Layer, String> {
        if at >= self.layers.list.len() {
            return Err("no layer at that index".into());
        }
        let layer = self.layers.list.remove(at);
        if !keep_shape {
            self.shift(&layer.delta, -layer.shape_scale());
            if let Some(p) = &layer.paint {
                self.shift_paint(p, -layer.paint_scale());
            }
            if let Some(m) = &layer.mask {
                self.shift_mask(m, -layer.mask_scale());
            }
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
        let s = layer.shape_scale();
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

    /// Adds the paint change (after minus before, divided by the layer's paint scale) to the layer's paint delta; `sign` -1 takes it back.
    pub fn record_paint_into_layer(&mut self, at: usize, indices: &[u32], before: &[Paint], after: &[Paint], sign: f32) {
        let n = self.mesh.vertex_count();
        let Some(layer) = self.layers.list.get_mut(at) else { return };
        let s = layer.paint_scale();
        if s.abs() < 1e-4 || before.len() != indices.len() || after.len() != indices.len() || before == after {
            return;
        }
        let slots = layer.paint.get_or_insert_with(|| vec![[0.0; crate::mesh::PAINT_LEN]; n]);
        for (k, &v) in indices.iter().enumerate() {
            if let Some(slot) = slots.get_mut(v as usize) {
                for c in 0..slot.len() {
                    slot[c] += (after[k][c] - before[k][c]) * (sign / s);
                }
            }
        }
    }

    /// Adds the mask change (after minus before, divided by the layer's mask scale) to the layer's mask delta; `sign` -1 takes it back.
    pub fn record_mask_into_layer(&mut self, at: usize, indices: &[u32], before: &[f32], after: &[f32], sign: f32) {
        let n = self.mesh.vertex_count();
        let Some(layer) = self.layers.list.get_mut(at) else { return };
        let s = layer.mask_scale();
        if s.abs() < 1e-4 || before.len() != indices.len() || after.len() != indices.len() || before == after {
            return;
        }
        let slots = layer.mask.get_or_insert_with(|| vec![0.0; n]);
        for (k, &v) in indices.iter().enumerate() {
            if let Some(slot) = slots.get_mut(v as usize) {
                *slot += (after[k] - before[k]) * (sign / s);
            }
        }
    }
}
