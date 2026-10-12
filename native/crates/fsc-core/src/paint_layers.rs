//! Paint layers: vertex paint kept in a stack above a base, each layer with a blend mode, an opacity and a
//! visibility, composited into `Mesh.paint` (which every renderer, bake and exporter reads). A layer holds per vertex
//! its colour, roughness, metalness and a coverage (0 is untouched); a vertex a layer never covered shows what lies
//! below. The stack counts only while every array matches the vertex count; a topology change flattens it, and the
//! composite already in `Mesh.paint` carries on.

use crate::mesh::{paint_channel, Paint, DEFAULT_PAINT, PAINT_CHANNELS, PAINT_LEN};
use crate::scene::Object;

/// The paint floats (colour, roughness, metalness, opacity, emissive) and then the coverage.
pub type LayerPaint = [f32; PAINT_LEN + 1];
/// Where a layer's coverage sits in its [`LayerPaint`].
pub const COVER: usize = PAINT_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Add,
}

impl Blend {
    pub const NAMES: [&'static str; 5] = ["normal", "multiply", "screen", "overlay", "add"];

    pub fn parse(name: &str) -> Option<Blend> {
        Some(match name {
            "normal" => Blend::Normal,
            "multiply" => Blend::Multiply,
            "screen" => Blend::Screen,
            "overlay" => Blend::Overlay,
            "add" => Blend::Add,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        Blend::NAMES[self as usize]
    }

    fn apply(self, below: f32, over: f32) -> f32 {
        match self {
            Blend::Normal => over,
            Blend::Multiply => below * over,
            Blend::Screen => 1.0 - (1.0 - below) * (1.0 - over),
            Blend::Overlay => {
                if below < 0.5 {
                    2.0 * below * over
                } else {
                    1.0 - 2.0 * (1.0 - below) * (1.0 - over)
                }
            }
            Blend::Add => (below + over).min(1.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaintLayer {
    pub name: String,
    pub blend: Blend,
    pub opacity: f32,
    pub visible: bool,
    pub data: Vec<LayerPaint>,
}

#[derive(Debug, Clone, Default)]
pub struct PaintLayers {
    /// Bottom to top.
    pub list: Vec<PaintLayer>,
    /// The layer brushes and fills paint into; None paints the base.
    pub active: Option<usize>,
    /// The paint under every layer.
    pub base: Vec<Paint>,
}

impl PaintLayers {
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// True when the stack holds layers whose arrays all match `vertices`.
    pub fn valid(&self, vertices: usize) -> bool {
        !self.list.is_empty() && self.base.len() == vertices && self.list.iter().all(|l| l.data.len() == vertices)
    }

    /// The paint at `v` from the base through the layers below `upto` (all visible layers when None).
    pub fn below(&self, v: usize, upto: Option<usize>) -> Paint {
        let mut out = self.base.get(v).copied().unwrap_or(DEFAULT_PAINT);
        let end = upto.unwrap_or(self.list.len()).min(self.list.len());
        for layer in &self.list[..end] {
            if !layer.visible {
                continue;
            }
            let d = layer.data[v];
            let a = (d[COVER] * layer.opacity).clamp(0.0, 1.0);
            if a <= 0.0 {
                continue;
            }
            for c in 0..3 {
                out[c] += (layer.blend.apply(out[c], d[c]) - out[c]) * a;
            }
            // Roughness, metalness, opacity and emissive mix normally whatever the blend mode.
            for c in 3..PAINT_LEN {
                out[c] += (d[c] - out[c]) * a;
            }
        }
        out
    }
}

impl Object {
    /// True when the object has a paint stack that matches its mesh. A stack left over from before a topology
    /// change is ignored (paint goes straight to the mesh) but kept, so undoing the topology step brings it back.
    pub fn layered(&self) -> bool {
        self.paint_layers.valid(self.mesh.vertex_count())
    }

    /// Drops a stack that no longer matches the mesh; paint layer commands call it before changing the stack.
    pub fn check_paint_layers(&mut self) {
        if !self.paint_layers.list.is_empty() && !self.layered() {
            self.paint_layers = PaintLayers::default();
        }
    }

    /// Writes the composite of the whole stack into `Mesh.paint` for `verts` (every vertex when None).
    pub fn composite_paint(&mut self, verts: Option<&[u32]>) {
        if !self.layered() {
            return;
        }
        self.mesh.ensure_paint();
        match verts {
            Some(vs) => {
                for &v in vs {
                    self.mesh.paint[v as usize] = self.paint_layers.below(v as usize, None);
                }
            }
            None => {
                for v in 0..self.mesh.vertex_count() {
                    self.mesh.paint[v] = self.paint_layers.below(v, None);
                }
            }
        }
    }

    /// Paints toward `target` by `w` in the active paint target (the active layer, or the base), for the channels
    /// chosen (colour, roughness, metalness, opacity, emissive); `erase` lowers a layer's coverage instead. Without
    /// layers it paints `Mesh.paint` directly. The caller recomposites (or not, without layers).
    pub fn paint_vertex(&mut self, v: u32, target: Paint, channels: [bool; PAINT_CHANNELS], w: f32, erase: bool) {
        let i = v as usize;
        if !self.layered() {
            self.mesh.ensure_paint();
            let cur = &mut self.mesh.paint[i];
            for (c, value) in cur.iter_mut().enumerate() {
                if channels[paint_channel(c)] {
                    *value += (target[c] - *value) * w;
                }
            }
            return;
        }
        match self.paint_layers.active {
            None => {
                let cur = &mut self.paint_layers.base[i];
                for (c, value) in cur.iter_mut().enumerate() {
                    if channels[paint_channel(c)] {
                        *value += (target[c] - *value) * w;
                    }
                }
            }
            Some(at) => {
                let start = self.paint_layers.below(i, Some(at));
                let d = &mut self.paint_layers.list[at].data[i];
                if erase {
                    d[COVER] = (d[COVER] - w).max(0.0);
                    return;
                }
                if d[COVER] <= 1e-6 {
                    // An untouched vertex starts from what shows through, so painting one channel keeps the others.
                    d[..PAINT_LEN].copy_from_slice(&start);
                }
                for c in 0..PAINT_LEN {
                    if channels[paint_channel(c)] {
                        d[c] += (target[c] - d[c]) * w;
                    }
                }
                d[COVER] += (1.0 - d[COVER]) * w;
            }
        }
    }

    /// The active paint target's value at `v`, for undo (a layer's data, or the base with full coverage).
    pub fn paint_target(&self, v: u32) -> Option<LayerPaint> {
        let i = v as usize;
        if !self.layered() {
            return None;
        }
        Some(match self.paint_layers.active {
            None => {
                let mut out = [1.0; PAINT_LEN + 1];
                out[..PAINT_LEN].copy_from_slice(&self.paint_layers.base[i]);
                out
            }
            Some(at) => self.paint_layers.list[at].data[i],
        })
    }

    pub fn set_paint_target(&mut self, target: Option<usize>, v: u32, value: LayerPaint) {
        let i = v as usize;
        match target {
            None => {
                if let Some(b) = self.paint_layers.base.get_mut(i) {
                    b.copy_from_slice(&value[..PAINT_LEN]);
                }
            }
            Some(at) => {
                if let Some(d) = self.paint_layers.list.get_mut(at).and_then(|l| l.data.get_mut(i)) {
                    *d = value;
                }
            }
        }
    }
}
