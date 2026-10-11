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
}

impl BrushKind {
    pub const NAMES: [&'static str; 16] = ["draw", "clay", "inflate", "pinch", "crease", "flatten", "smooth", "grab", "mask", "paint", "relax", "planar", "trim", "snake_hook", "noise", "smudge"];

    pub fn moves_vertices(&self) -> bool {
        !matches!(self, BrushKind::Mask | BrushKind::Paint | BrushKind::Smudge)
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
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Falloff {
    Smooth,
    Sphere,
    Linear,
    Sharp,
}

impl Falloff {
    pub const NAMES: [&'static str; 4] = ["smooth", "sphere", "linear", "sharp"];

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
    pub paint: [f32; 5],
    pub paint_channels: [bool; 3],
    /// Noise brush: feature size in world units (0 means a fifth of the radius).
    pub noise_scale: f32,
    /// Alpha orientation: the world direction the alpha's top points toward (the view's up in the window), turned by
    /// `alpha_angle` radians; with `roll` the alpha's top follows the stroke's direction instead.
    pub alpha_up: [f32; 3],
    pub alpha_angle: f32,
    pub roll: bool,
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
            paint: [0.8, 0.2, 0.2, 0.5, 0.0],
            paint_channels: [true, false, false],
            noise_scale: 0.0,
            alpha_up: [0.0, 1.0, 0.0],
            alpha_angle: 0.0,
            roll: false,
        }
    }
}
