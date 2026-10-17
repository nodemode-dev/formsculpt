mod alpha;
mod brush;
mod noise;
mod stroke;
mod symmetry;
mod vdm;

pub use alpha::{turn, Alpha};
pub use brush::{BrushKind, BrushSettings, DyntopoMethod, DyntopoMode, Falloff, GroupList};
pub use noise::{noise_at, NoisePattern, NoiseSettings};
pub use stroke::{fbm, value_noise, Stroke, StrokeEdit, VertexEdit};
pub use symmetry::Symmetry;
pub use vdm::{vdm_stamp, Vdm, VdmPlacement};

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
