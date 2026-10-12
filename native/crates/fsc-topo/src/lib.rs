mod build;
mod decimate;
mod deform;
mod dyntopo;
mod edges;
mod holes;
mod manifold;
mod nearest;
mod quad;
mod region;
mod reconstruct;
mod subdivide;
mod text;
mod transfer;
mod voxel;

pub use build::{lathe, silhouette_mesh, skeleton_mesh, tube, Ball};
pub use decimate::decimate;
pub use deform::{deform, lattice_box, lattice_deform, merge, split_parts, Deformer};
pub use dyntopo::{collapse_sphere, compact, refine_sphere};
pub use edges::{boundary_edges, edge_faces, Edge};
pub use holes::fill_holes;
pub use manifold::split_nonmanifold;
pub use nearest::{closest_on_triangle, nearest_feature, nearest_point};
pub use quad::{quad_remesh, QuadRemesh};
pub use reconstruct::{reconstruct_lower, Reconstructed};
pub use region::{extract_shell, extrude_region, project_onto, region_faces};
pub use subdivide::{loop_subdivide, loop_subdivide_with, smooth_preview, Subdivided};
pub use text::{extrude_shape, text_mesh, TEXT_CHARACTERS};
pub use transfer::transfer_paint;
pub use voxel::{round_edges, voxel_boolean, voxel_boolean_blend, voxel_remesh, BooleanOp, RoundEdges};

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
