mod decimate;
mod dyntopo;
mod edges;
mod holes;
mod nearest;
mod reconstruct;
mod subdivide;
mod voxel;

pub use decimate::decimate;
pub use dyntopo::{collapse_sphere, compact, refine_sphere};
pub use edges::{boundary_edges, edge_faces, Edge};
pub use holes::fill_holes;
pub use nearest::{closest_on_triangle, nearest_point};
pub use reconstruct::{reconstruct_lower, Reconstructed};
pub use subdivide::{loop_subdivide, Subdivided};
pub use voxel::voxel_remesh;

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
