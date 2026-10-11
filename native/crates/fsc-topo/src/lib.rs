mod decimate;
mod deform;
mod dyntopo;
mod edges;
mod holes;
mod manifold;
mod nearest;
mod reconstruct;
mod subdivide;
mod transfer;
mod voxel;

pub use decimate::decimate;
pub use deform::{deform, merge, split_parts, Deformer};
pub use dyntopo::{collapse_sphere, compact, refine_sphere};
pub use edges::{boundary_edges, edge_faces, Edge};
pub use holes::fill_holes;
pub use manifold::split_nonmanifold;
pub use nearest::{closest_on_triangle, nearest_point};
pub use reconstruct::{reconstruct_lower, Reconstructed};
pub use subdivide::{loop_subdivide, Subdivided};
pub use transfer::transfer_paint;
pub use voxel::{voxel_boolean, voxel_remesh, BooleanOp};

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
