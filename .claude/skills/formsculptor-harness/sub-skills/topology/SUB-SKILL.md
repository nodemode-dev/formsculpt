---
name: topology
description: "Covers fsc-topo in formsculptor: dynamic topology, multires, voxel remesh, QEM decimation, quad remesh, booleans and hole fill. Load when changing mesh resolution, remeshing, decimating, boolean ops or repairing holes."
---

# Topology

## Purpose

Own `fsc-topo`. Operations that change connectivity or vertex count. Hands back a topology change that rebuilds derived data and stays undoable.

## Preconditions

State at 0.4.0: built in `native/crates/fsc-topo`: `loop_subdivide` (Loop or flat, parents for mask transfer), `voxel_remesh` (narrow-band distance from BVH nearest-feature queries, signs from angle-weighted pseudo-normals, naive surface nets, rayon), `decimate` (quadric edge collapse with link and flip checks, border quadrics), `fill_holes` (boundary loops, centroid fans), `refine_sphere` (dynamic topology by longest-edge bisection with curved midpoints, called once per stroke sample, followed by tangential relaxation in `fsc-sculpt`), `edge_faces` and `boundary_edges`. Commands `subdivide`, `remesh_voxel`, `decimate`, `fill_holes`, `mesh_check`; every topology change goes through `Object::replace_mesh` and an `Edit::Topology` undo step that swaps the whole mesh and mask. At 0.4.1: `loop_subdivide` also returns a `Stencil` (CSR weights from the lower level), and `fsc_core::multires` keeps `Object.multires` (levels, current; the live level's mesh sits in `Object.mesh`, the others in their `Level`); stepping down stores the higher level as tangent-frame offsets from the subdivided lower level, stepping up rebuilds it from them; `Edit::Divide` parks the removed level for exact redo, `Edit::Level` replays the step, `Edit::Levels` swaps the stack for delete higher or lower; `multires_cmds::guard` blocks every whole-mesh change while levels exist. `collapse_sphere` (link condition, interior fans only, valence and flip checks, one-ring lock per pass) runs after each `refine_sphere` at 0.4 of the detail size; collapsed vertices stay as orphans until `compact` at stroke end. `reconstruct_lower` labels even and odd vertices through ordered rings, rebuilds the lower triangles from the all-odd centre triangles, inverts Loop's even rule by Jacobi sweeps and renumbers the stencil to the live vertex order. Not built: quad remesh, booleans.

## Procedure

1. Scope: dynamic topology (local split and collapse during strokes), multires (subdivision levels with detail kept per level), voxel remesh, QEM decimation, quad remesh, booleans, hole fill.
2. Connectivity changes invalidate adjacency, normals, BVH, masks, groups, colour and layers. Remap or rebuild each (see `sub-skills/engine-core/SUB-SKILL.md`).
3. Count changes mean the whole buffer set is replaced, so the viewport must do a full upload instead of a dirty-range one (`sub-skills/render-viewport/SUB-SKILL.md`).
4. Undo for these is larger than a region delta; record what is needed to restore, and keep it one undo step.
5. Long operations run as jobs when called through the backend (`sub-skills/engine-service/SUB-SKILL.md`).
6. Expose each operation as a `topo.*` command via `sub-skills/command-registry/SUB-SKILL.md`.
7. No GPL code; Blender is study only.

## Gate

Done when the result is manifold where the operation promises it, masks and groups survived or were deliberately cleared, and undo restores the prior mesh.

## Hands back

The operation, its command id, and the checks run.

## Owned resources

None bundled.
