---
name: topology
description: "Covers fsc-topo in formsculpt: dynamic topology, multires, voxel remesh, QEM decimation, quad remesh, booleans and hole fill. Load when changing mesh resolution, remeshing, decimating, boolean ops or repairing holes."
---

# Topology

## Purpose

Own `fsc-topo`. Operations that change connectivity or vertex count. Hands back a topology change that rebuilds derived data and stays undoable.

## Preconditions

State at 0.4.0: built in `native/crates/fsc-topo`: `loop_subdivide` (Loop or flat, parents for mask transfer), `voxel_remesh` (narrow-band distance from BVH nearest-feature queries, signs from angle-weighted pseudo-normals, naive surface nets, rayon), `decimate` (quadric edge collapse with link and flip checks, border quadrics), `fill_holes` (boundary loops, centroid fans), `refine_sphere` (dynamic topology by longest-edge bisection with curved midpoints, called once per stroke sample, followed by tangential relaxation in `fsc-sculpt`), `edge_faces` and `boundary_edges`. Commands `subdivide`, `remesh_voxel`, `decimate`, `fill_holes`, `mesh_check`; every topology change goes through `Object::replace_mesh` and an `Edit::Topology` undo step that swaps the whole mesh and mask. At 0.4.1: `loop_subdivide` also returns a `Stencil` (CSR weights from the lower level), and `fsc_core::multires` keeps `Object.multires` (levels, current; the live level's mesh sits in `Object.mesh`, the others in their `Level`); stepping down stores the higher level as tangent-frame offsets from the subdivided lower level, stepping up rebuilds it from them; `Edit::Divide` parks the removed level for exact redo, `Edit::Level` replays the step, `Edit::Levels` swaps the stack for delete higher or lower; `multires_cmds::guard` blocks every whole-mesh change while levels exist. `collapse_sphere` (link condition, interior fans only, valence and flip checks, one-ring lock per pass) runs after each `refine_sphere` at 0.4 of the detail size; collapsed vertices stay as orphans until `compact` at stroke end. `reconstruct_lower` labels even and odd vertices through ordered rings, rebuilds the lower triangles from the all-odd centre triangles, inverts Loop's even rule by Jacobi sweeps and renumbers the stencil to the live vertex order. At 0.5.0: `voxel.rs` is split into `Grid`, `narrow_band`, `fill_signs` and `surface_nets`; `voxel_boolean` combines two filled fields (min, max, max with negation) and runs `manifold::split_nonmanifold`, which pairs faces around four-face edges by angle and inside gap and splits vertices per fan, so boolean results are watertight. `deform.rs` holds bend, twist, taper and transform (mask-weighted), `split_parts` and `merge`. At 0.8.0: `quad.rs` (quad_remesh: surface nets at a resolution from area and target, one corrective rerun, tangential relaxation snapped to the original; sets `Mesh.quads`, which `Mesh::quad_faces` turns into quads for OBJ) and `region.rs` (mask regions: extract_shell, extrude_region for extrude and inset, project_onto). At 0.8.1: `voxel::implicit_mesh` samples any distance function on the grid in parallel and runs surface nets; `build.rs` uses it for `skeleton_mesh` (round cones per link, spheres for unlinked balls, polynomial smooth min, then `split_nonmanifold`, because thin limbs within one cell left 2 to 4 non-manifold edges) and holds `tube` (Catmull-Rom resampling, parallel-transport rings, fan caps or a closed ring) and `lathe` (rings around y, poles at zero radius, flat caps); both flip winding by signed volume. Commands live in `fsc-commands/src/build_cmds.rs`. At 0.20.0 `text.rs` holds the stroke font, `text_mesh` and `extrude_shape` (implicit fields through `implicit_mesh`, resolution picked from the finest feature), and `voxel_boolean_blend` fillets booleans by a smooth min or max with the narrow band widened to the blend; `voxel_boolean` is its blend-0 case, and `cut_shape` (topo_cmds) is an extruded outline taken away by it. Any new grid face builder must check its winding with `print_check` on the result: `plane_face` was wound inward from 0.2.0 to 0.19.0. Not built: field-aligned quad remesh with guides, bevel, edge loops, bridge, exact (mesh-mesh) booleans.

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
