mod bake;
mod charts;
mod encode;
mod render;
mod uv;

pub use bake::{bake, Baked};
pub use charts::{segment, Segmented};
pub use encode::{apng_rgba, exr_float, png_gray, png_rgba, png_rgba_decode, png_rgba_rect};
pub use render::{path_trace, path_trace_with, render, view_coordinates, Light, Pass, PathOptions, RenderObject, Rendered, Shading, View};
pub use uv::{expand, island_at, islands, layout_image, pack, stretch, unwrap, unwrap_charts, unwrap_keyed, unwrap_with, Charts, Expanded, Flatten, Unwrap};

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
