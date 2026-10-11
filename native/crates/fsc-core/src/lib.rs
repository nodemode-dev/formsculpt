pub mod bvh;
pub mod mesh;
pub mod primitives;
pub mod scene;
pub mod topology;

pub use bvh::{Bvh, Hit};
pub use topology::Topology;
pub use mesh::{Aabb, Mesh};
pub use primitives::Primitive;
pub use scene::{Object, ObjectId, Scene};

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn engine_version() -> &'static str {
    ENGINE_VERSION
}
