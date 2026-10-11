pub mod bvh;
pub mod layers;
pub mod mesh;
pub mod multires;
pub mod primitives;
pub mod scene;
pub mod topology;

pub use bvh::{Bvh, Hit};
pub use topology::Topology;
pub use layers::{Layer, Layers};
pub use mesh::{mix_paint, Aabb, Mesh, Paint, DEFAULT_PAINT};
pub use multires::{Level, Multires, Stencil};
pub use primitives::Primitive;
pub use scene::{Maps, Object, ObjectId, Scene};

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn engine_version() -> &'static str {
    ENGINE_VERSION
}
