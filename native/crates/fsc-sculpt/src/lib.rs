mod brush;
mod stroke;
mod symmetry;

pub use brush::{BrushKind, BrushSettings, Falloff};
pub use stroke::{fbm, value_noise, Stroke, StrokeEdit, VertexEdit};
pub use symmetry::Symmetry;

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
