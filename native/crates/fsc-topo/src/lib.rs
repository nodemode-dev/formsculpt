mod build;
mod decimate;
mod deform;
mod dyntopo;
mod edges;
mod exact;
mod holes;
mod loops;
mod manifold;
mod nearest;
mod quad;
mod region;
mod reconstruct;
mod subdivide;
mod sweep;
mod text;
mod transfer;
mod field;
mod patches;
mod voxel;
mod zmodel;

pub use build::{lathe, silhouette_mesh, skeleton_mesh, tube, Ball};
pub use decimate::{decimate, decimate_with, DecimateOptions};
pub use deform::{deform, lattice_box, lattice_deform, merge, split_parts, Deformer};
pub use dyntopo::{collapse_sphere, compact, refine_sphere, refine_sphere_protected};
pub use edges::{boundary_edges, edge_faces, Edge};
pub use holes::fill_holes;
pub use manifold::split_nonmanifold;
pub use nearest::{closest_on_triangle, nearest_feature, nearest_point};
pub use quad::{guide_frame, quad_remesh, quad_remesh_aligned, QuadRemesh};
pub use reconstruct::{reconstruct_lower, Reconstructed};
pub use region::{extract_patch, extract_shell, extrude_region, project_onto, region_faces, PatchMode};
pub use subdivide::{loop_subdivide, loop_subdivide_with, smooth_preview, Subdivided};
pub use sweep::{sweep, Cap, SweepOptions};
pub use text::{extrude_shape, text_mesh, TEXT_CHARACTERS};
pub use transfer::transfer_paint;
pub use exact::exact_boolean;
pub use loops::{border_loops, bridge_borders, insert_edge_loop};
pub use field::{field_remesh, field_remesh_guided, FieldOptions, FieldRemesh};
pub use patches::{patch, Patch, PATCH_KINDS};
pub use zmodel::{close_border, delete_face, edge_loop, extrude_edge, loop_moves, inset_face, insert_loop_at, poly_hit, quads_of, set_creases, PolyHit};
pub use voxel::{round_edges, voxel_boolean, voxel_boolean_blend, voxel_remesh, BooleanOp, RoundEdges};

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
