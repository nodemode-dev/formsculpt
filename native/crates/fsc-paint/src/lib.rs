mod bake;
mod encode;
mod render;
mod uv;

pub use bake::{bake, Baked};
pub use encode::{apng_rgba, exr_float, png_gray, png_rgba, png_rgba_rect};
pub use render::{render, view_coordinates, RenderObject, Rendered, Shading, View};
pub use uv::{expand, unwrap, Expanded, Unwrap};

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
