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
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    LinearBurn,
    LinearLight,
    VividLight,
    PinLight,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Subtract,
    Divide,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

/// The paint channels a per-channel blend can name, in channel order.
pub const CHANNEL_NAMES: [&str; PAINT_CHANNELS] = ["color", "roughness", "metalness", "opacity", "emissive"];

/// A blend override per paint channel; None uses the layer's own rule.
pub type ChannelBlend = [Option<Blend>; PAINT_CHANNELS];

fn dodge(b: f32, s: f32) -> f32 {
    if b <= 0.0 {
        0.0
    } else if s >= 1.0 {
        1.0
    } else {
        (b / (1.0 - s)).min(1.0)
    }
}

fn burn(b: f32, s: f32) -> f32 {
    if b >= 1.0 {
        1.0
    } else if s <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - b) / s).min(1.0)
    }
}

fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 {
        for v in out.iter_mut() {
            *v = l + (*v - l) * l / (l - n);
        }
    }
    if x > 1.0 {
        for v in out.iter_mut() {
            *v = l + (*v - l) * (1.0 - l) / (x - l);
        }
    }
    out
}

fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

fn sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|&a, &b| c[a].total_cmp(&c[b]));
    let (lo, mid, hi) = (idx[0], idx[1], idx[2]);
    let mut out = [0.0; 3];
    if c[hi] > c[lo] {
        out[mid] = (c[mid] - c[lo]) * s / (c[hi] - c[lo]);
        out[hi] = s;
    }
    out
}

impl Blend {
    pub const NAMES: [&'static str; 23] = [
        "normal",
        "multiply",
        "screen",
        "overlay",
        "add",
        "darken",
        "lighten",
        "color_dodge",
        "color_burn",
        "linear_burn",
        "linear_light",
        "vivid_light",
        "pin_light",
        "hard_light",
        "soft_light",
        "difference",
        "exclusion",
        "subtract",
        "divide",
        "hue",
        "saturation",
        "color",
        "luminosity",
    ];

    pub fn parse(name: &str) -> Option<Blend> {
        Some(match name {
            "normal" => Blend::Normal,
            "multiply" => Blend::Multiply,
            "screen" => Blend::Screen,
            "overlay" => Blend::Overlay,
            "add" => Blend::Add,
            "darken" => Blend::Darken,
            "lighten" => Blend::Lighten,
            "color_dodge" => Blend::ColorDodge,
            "color_burn" => Blend::ColorBurn,
            "linear_burn" => Blend::LinearBurn,
            "linear_light" => Blend::LinearLight,
            "vivid_light" => Blend::VividLight,
            "pin_light" => Blend::PinLight,
            "hard_light" => Blend::HardLight,
            "soft_light" => Blend::SoftLight,
            "difference" => Blend::Difference,
            "exclusion" => Blend::Exclusion,
            "subtract" => Blend::Subtract,
            "divide" => Blend::Divide,
            "hue" => Blend::Hue,
            "saturation" => Blend::Saturation,
            "color" => Blend::Color,
            "luminosity" => Blend::Luminosity,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        Blend::NAMES[self as usize]
    }

    /// True for the modes that act on the three colour floats together (hue, saturation, color, luminosity).
    pub fn non_separable(self) -> bool {
        matches!(self, Blend::Hue | Blend::Saturation | Blend::Color | Blend::Luminosity)
    }

    /// One scalar channel; the non-separable modes fall back to normal.
    pub fn apply(self, below: f32, over: f32) -> f32 {
        let (b, s) = (below, over);
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
            Blend::Darken => b.min(s),
            Blend::Lighten => b.max(s),
            Blend::ColorDodge => dodge(b, s),
            Blend::ColorBurn => burn(b, s),
            Blend::LinearBurn => (b + s - 1.0).clamp(0.0, 1.0),
            Blend::LinearLight => (b + 2.0 * s - 1.0).clamp(0.0, 1.0),
            Blend::VividLight => {
                if s <= 0.5 {
                    burn(b, 2.0 * s)
                } else {
                    dodge(b, 2.0 * (s - 0.5))
                }
            }
            Blend::PinLight => {
                if s < 0.5 {
                    b.min(2.0 * s)
                } else {
                    b.max(2.0 * (s - 0.5))
                }
            }
            Blend::HardLight => {
                if s <= 0.5 {
                    b * 2.0 * s
                } else {
                    let t = 2.0 * s - 1.0;
                    (b + t - b * t).clamp(0.0, 1.0)
                }
            }
            Blend::SoftLight => {
                let r = if s <= 0.5 {
                    b - (1.0 - 2.0 * s) * b * (1.0 - b)
                } else {
                    let d = if b <= 0.25 { ((16.0 * b - 12.0) * b + 4.0) * b } else { b.sqrt() };
                    b + (2.0 * s - 1.0) * (d - b)
                };
                r.clamp(0.0, 1.0)
            }
            Blend::Difference => (b - s).abs(),
            Blend::Exclusion => (b + s - 2.0 * b * s).clamp(0.0, 1.0),
            Blend::Subtract => (b - s).max(0.0),
            Blend::Divide => {
                if s <= 1e-6 {
                    1.0
                } else {
                    (b / s).min(1.0)
                }
            }
            Blend::Hue | Blend::Saturation | Blend::Color | Blend::Luminosity => over,
        }
    }

    /// The three colour floats together; the separable modes act on each float alone.
    pub fn apply_rgb(self, below: [f32; 3], over: [f32; 3]) -> [f32; 3] {
        let out = match self {
            Blend::Hue => set_lum(set_sat(over, sat(below)), lum(below)),
            Blend::Saturation => set_lum(set_sat(below, sat(over)), lum(below)),
            Blend::Color => set_lum(over, lum(below)),
            Blend::Luminosity => set_lum(below, lum(over)),
            _ => return [self.apply(below[0], over[0]), self.apply(below[1], over[1]), self.apply(below[2], over[2])],
        };
        [out[0].clamp(0.0, 1.0), out[1].clamp(0.0, 1.0), out[2].clamp(0.0, 1.0)]
    }
}

#[derive(Debug, Clone)]
pub struct PaintLayer {
    pub name: String,
    pub blend: Blend,
    /// Per-channel blend overrides; a colour override replaces `blend` for colour, and a scalar channel mixes normally unless overridden.
    pub channel_blend: ChannelBlend,
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
            let mixed = layer.channel_blend[0].unwrap_or(layer.blend).apply_rgb([out[0], out[1], out[2]], [d[0], d[1], d[2]]);
            for c in 0..3 {
                out[c] += (mixed[c] - out[c]) * a;
            }
            // Roughness, metalness, opacity and emissive mix normally unless the layer overrides that channel.
            for c in 3..PAINT_LEN {
                let mode = layer.channel_blend[paint_channel(c)].unwrap_or(Blend::Normal);
                out[c] += (mode.apply(out[c], d[c]) - out[c]) * a;
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
