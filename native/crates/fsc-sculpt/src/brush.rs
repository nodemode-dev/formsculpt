use crate::symmetry::Symmetry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushKind {
    Draw,
    Clay,
    Inflate,
    Pinch,
    Crease,
    Flatten,
    Smooth,
    Grab,
    Mask,
    Paint,
    Relax,
    Planar,
    Trim,
    SnakeHook,
    Noise,
    Smudge,
    /// A regularized Kelvinlet grab that drags a wide, smooth, volume-like neighbourhood.
    Elastic,
    /// Hard-surface polish: cuts what sticks out of the fitted plane and smooths without crossing creases.
    Polish,
    /// Builds a raised layer whose per-vertex offset over the stroke is capped at `height`.
    Layer,
    /// Slides vertices along the surface in the stroke direction without changing height.
    Nudge,
    /// A single draw dab at the first point of the stroke.
    Stamp,
    /// Scales down the active sculpt layer's contribution under the brush.
    LayerReset,
}

impl BrushKind {
    pub const NAMES: [&'static str; 22] = [
        "draw", "clay", "inflate", "pinch", "crease", "flatten", "smooth", "grab", "mask", "paint", "relax", "planar", "trim", "snake_hook", "noise", "smudge", "elastic", "polish", "layer", "nudge", "stamp",
        "layer_reset",
    ];

    pub fn moves_vertices(&self) -> bool {
        !matches!(self, BrushKind::Mask | BrushKind::Paint | BrushKind::Smudge)
    }

    /// Whether the brush captures the vertices under the first point and drags them by the pointer's offset.
    pub fn drags(&self) -> bool {
        matches!(self, BrushKind::Grab | BrushKind::Elastic)
    }

    pub fn parse(name: &str) -> Option<BrushKind> {
        Some(match name {
            "draw" => BrushKind::Draw,
            "clay" => BrushKind::Clay,
            "inflate" => BrushKind::Inflate,
            "pinch" => BrushKind::Pinch,
            "crease" => BrushKind::Crease,
            "flatten" => BrushKind::Flatten,
            "smooth" => BrushKind::Smooth,
            "grab" => BrushKind::Grab,
            "mask" => BrushKind::Mask,
            "paint" => BrushKind::Paint,
            "relax" => BrushKind::Relax,
            "planar" => BrushKind::Planar,
            "trim" => BrushKind::Trim,
            "snake_hook" => BrushKind::SnakeHook,
            "noise" => BrushKind::Noise,
            "smudge" => BrushKind::Smudge,
            "elastic" => BrushKind::Elastic,
            "polish" => BrushKind::Polish,
            "layer" => BrushKind::Layer,
            "nudge" => BrushKind::Nudge,
            "stamp" => BrushKind::Stamp,
            "layer_reset" => BrushKind::LayerReset,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Falloff {
    Smooth,
    Sphere,
    Linear,
    Sharp,
    /// A drawn curve: the weight at 17 evenly spaced distances from the centre (0) to the rim (1), read linearly.
    Curve([f32; Falloff::CURVE_SAMPLES]),
}

impl Falloff {
    pub const NAMES: [&'static str; 4] = ["smooth", "sphere", "linear", "sharp"];
    pub const CURVE_SAMPLES: usize = 17;

    /// A curve falloff from 2 or more weights evenly spaced from the centre to the rim (each clamped to 0..1),
    /// resampled linearly to the fixed table.
    pub fn curve(weights: &[f32]) -> Option<Falloff> {
        if weights.len() < 2 || weights.iter().any(|w| !w.is_finite()) {
            return None;
        }
        let last = (weights.len() - 1) as f32;
        let mut table = [0.0; Falloff::CURVE_SAMPLES];
        for (i, slot) in table.iter_mut().enumerate() {
            let x = i as f32 / (Falloff::CURVE_SAMPLES - 1) as f32 * last;
            let k = (x.floor() as usize).min(weights.len() - 2);
            let f = x - k as f32;
            *slot = (weights[k] * (1.0 - f) + weights[k + 1] * f).clamp(0.0, 1.0);
        }
        Some(Falloff::Curve(table))
    }

    pub fn parse(name: &str) -> Option<Falloff> {
        Some(match name {
            "smooth" => Falloff::Smooth,
            "sphere" => Falloff::Sphere,
            "linear" => Falloff::Linear,
            "sharp" => Falloff::Sharp,
            _ => return None,
        })
    }

    pub fn weight(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Falloff::Smooth => 1.0 - t * t * (3.0 - 2.0 * t),
            Falloff::Sphere => (1.0 - t * t).max(0.0).sqrt(),
            Falloff::Linear => 1.0 - t,
            Falloff::Sharp => (1.0 - t) * (1.0 - t),
            Falloff::Curve(table) => {
                let x = t * (Falloff::CURVE_SAMPLES - 1) as f32;
                let k = (x.floor() as usize).min(Falloff::CURVE_SAMPLES - 2);
                let f = x - k as f32;
                table[k] * (1.0 - f) + table[k + 1] * f
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushSettings {
    pub kind: BrushKind,
    pub radius: f32,
    pub strength: f32,
    pub invert: bool,
    pub falloff: Falloff,
    pub spacing: f32,
    pub lazy: f32,
    pub symmetry: Symmetry,
    pub dyntopo: Option<f32>,
    pub dyntopo_size: Option<f32>,
    pub paint: fsc_core::Paint,
    /// Which channels the paint brush paints: colour, roughness, metalness, opacity, emissive.
    pub paint_channels: [bool; fsc_core::PAINT_CHANNELS],
    /// Noise brush: feature size in world units (0 means a fifth of the radius).
    pub noise_scale: f32,
    /// Alpha orientation: the world direction the alpha's top points toward (the view's up in the window), turned by
    /// `alpha_angle` radians; with `roll` the alpha's top follows the stroke's direction instead.
    pub alpha_up: [f32; 3],
    pub alpha_angle: f32,
    pub roll: bool,
    /// Spray: each dab lands up to `spray` radii from the path, turned at random and at a random share of the
    /// strength (0 is an ordinary stroke).
    pub spray: f32,
    /// Pen pressure: raised to `pressure_gamma` (above 1 needs a firmer press), then scaling strength and or radius.
    pub pressure_gamma: f32,
    pub pressure_strength: bool,
    pub pressure_size: bool,
    /// Layer brush: the cap on each vertex's offset over one stroke, in world units (0 means a tenth of the radius).
    pub height: f32,
    /// Multiplies each dab's strength by spacing over the default spacing (at most 4), so the deposit per length holds across spacings.
    pub spacing_compensation: bool,
    /// Lifts each dab's centre along its area normal by this many radii (positive touches peaks, negative reaches hollows).
    pub offset: f32,
    /// Jitter: each dab moves along the surface by up to this many radii in a random direction.
    pub jitter_position: f32,
    /// Jitter: each dab's radius is scaled by 1 plus or minus up to this share.
    pub jitter_radius: f32,
    /// Jitter: each dab's strength is scaled by 1 plus or minus up to this share.
    pub jitter_strength: f32,
    /// Jitter: the seed of the generator, so a stroke repeats exactly.
    pub jitter_seed: u64,
    /// Each dab's radius is rounded to a multiple of this in world units, at least one step (0 is off).
    pub snap_radius: f32,
    /// Only vertices whose normal faces the viewer are affected, with a soft ramp over the first 0.2 of facing.
    pub front_facing: bool,
    /// The direction the viewer looks along (None: the first point's surface normal, reversed, stands in).
    pub view_dir: Option<[f32; 3]>,
    /// Only vertices joined by mesh edges to the vertex nearest the dab centre, inside the brush sphere, are affected.
    pub connected_only: bool,
    /// Only faces in the face group under each copy's first dab are affected (borders at half weight).
    pub protect_group: bool,
    /// Vertices whose faces all lie in these face groups are untouched.
    pub protect_groups: GroupList,
    /// Vertices on an edge sharper than this many degrees are held, easing back over their one-ring (0 is off).
    pub keep_sharp: f32,
    /// When false, a vertex's displacement over the stroke is capped at the largest single-dab move it received.
    pub accumulate: bool,
    /// The radius, as a share of the brush radius, used for the dab's area normal and plane.
    pub area_sample: f32,
    /// Vertices more than this far below the dab's area plane are untouched, easing over the last 20% (0 is off).
    pub depth: f32,
}

/// Up to 16 face group ids.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GroupList {
    pub ids: [u32; 16],
    pub len: usize,
}

impl GroupList {
    pub fn contains(&self, id: u32) -> bool {
        self.ids[..self.len].contains(&id)
    }
}

impl Default for BrushSettings {
    fn default() -> Self {
        BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.2,
            strength: 0.5,
            invert: false,
            falloff: Falloff::Smooth,
            spacing: 0.15,
            lazy: 0.0,
            symmetry: Symmetry::default(),
            dyntopo: None,
            dyntopo_size: None,
            paint: [0.8, 0.2, 0.2, 0.5, 0.0, 1.0, 0.0],
            paint_channels: [true, false, false, false, false],
            noise_scale: 0.0,
            alpha_up: [0.0, 1.0, 0.0],
            alpha_angle: 0.0,
            roll: false,
            spray: 0.0,
            pressure_gamma: 1.0,
            pressure_strength: true,
            pressure_size: false,
            height: 0.0,
            spacing_compensation: false,
            offset: 0.0,
            jitter_position: 0.0,
            jitter_radius: 0.0,
            jitter_strength: 0.0,
            jitter_seed: 1,
            snap_radius: 0.0,
            front_facing: false,
            view_dir: None,
            connected_only: false,
            protect_group: false,
            protect_groups: GroupList::default(),
            keep_sharp: 0.0,
            accumulate: true,
            area_sample: 1.0,
            depth: 0.0,
        }
    }
}
