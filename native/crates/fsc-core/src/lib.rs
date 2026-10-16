pub mod bvh;
pub mod curve;
pub mod layers;
pub mod mesh;
pub mod multires;
pub mod paint_layers;
pub mod primitives;
pub mod scene;
pub mod time;
pub mod topology;

pub use bvh::{Bvh, Hit};
pub use curve::{Curve, CurvePoint, CurveSample, Interp};
pub use topology::Topology;
pub use layers::{Layer, Layers};
pub use mesh::{mix_paint, paint_channel, paint_from5, Aabb, Mesh, Paint, DEFAULT_PAINT, PAINT_CHANNELS, PAINT_LEN};
pub use multires::{Level, Multires, Stencil};
pub use paint_layers::{Blend, LayerPaint, PaintLayer, PaintLayers, COVER};
pub use primitives::Primitive;
pub use scene::{Instance, Maps, Material, Object, ObjectId, Scene};

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn engine_version() -> &'static str {
    ENGINE_VERSION
}
