---
name: engine-core
description: "Covers fsc-core, the formsculptor mesh and scene foundation in native/: SoA mesh buffers, adjacency, normals, BVH, scene, masks, face groups, vertex colour, sculpt layers and region delta undo. Load when editing mesh data layout, picking or raycast, undo, masks, groups or layers."
---

# Engine Core

## Purpose

Own `fsc-core`. Every other crate reads and writes its types, so a change here has the widest blast radius. Hands back a core change that keeps buffer layout, dirty tracking and undo coherent.

## Preconditions

State at 0.5.0: built: `Mesh` (positions, normals, indices), primitives, a binned-SAH `Bvh` with partial refit (`refit_triangles`, `Object::refit_vertices`), `Topology` (vertex faces and neighbours), `Scene` and `Object` (mask, revision, `multires`, `layers`), `multires.rs` (level stack, stencils, tangent-frame detail) and `layers.rs` (per-vertex offsets with a strength; `record_into_layer` for strokes). Undo lives in `fsc-commands/src/history.rs`, not here. Since then `Mesh` gained `paint` (0.7.0), `uvs` and `quads` (0.7.1, 0.8.0), and at 0.11.0 `groups` (u32 per triangle) and `hidden` (bool per triangle); all per-triangle and per-corner arrays are valid only while their length matches, so a tool that changes triangles in place must carry them (as `loop_subdivide_with` does) or clear them (as dyntopo does), and readers use `has_groups`/`has_hidden`. `Bvh::raycast` skips hidden triangles. `Object.folder` (0.11.0) files objects for the scene list. Not built: dirty-range tracking for WASM. At 0.14.0 `paint_layers.rs` adds `Object.paint_layers` (a base and layers of colour, roughness, metalness and coverage with a `Blend` and opacity) composited into `Mesh.paint`; every paint write goes through `Object::paint_vertex` and `composite_paint`, and `Object::layered()` (the stack matches the vertex count) decides whether layers are in play. Never write `Mesh.paint` directly in new paint code while `layered()` is true, or the next recomposite overwrites it. At 0.26.0 `Paint` is `PAINT_LEN` (7) floats: colour, roughness, metalness, opacity, emissive; channels are indexed through `paint_channel` and a layer's coverage sits at `COVER`. Never write a literal 5 or 6 for paint again; readers of older projects go through `paint_from5`. At 0.27.0 the engine also builds for wasm32-unknown-unknown (fsc-wasm): never call `std::time::Instant` (it panics there; use `fsc_core::time::Instant`), never spawn threads outside a `parallel` feature (rayon is opt-in through fsc-commands' `parallel`), and put any byte format the window reads in `fsc_commands::live` so fsc-py and fsc-wasm share it. At 0.28.0 the minimum Rust is 1.88 (boolmesh, the exact boolean dependency, needs it), so clippy expects `is_multiple_of`, `is_none_or` and `repeat_n`. Treat other items below as planned unless the code shows them.

## Procedure

1. Read the fsc-core source and `native/HASHOUT_LOG.md` first.
2. Mesh is structure-of-arrays: separate buffers for positions, normals, indices, vertex colour, mask. The frontend reads these through WASM by dirty range, so every write path must record which range changed.
3. Adjacency, normals and the BVH are derived data. Rebuild or refit them after topology or position changes; never leave stale.
4. Masks, face groups, vertex colour and sculpt layers live on the mesh or scene, not in other crates.
5. Undo is region delta: store only the changed range before and after. Every mutating command is exactly one undo step, so group a stroke's deltas into one entry.
6. Keep the crate free of DOM, WASM and Python concerns. Bindings live in fsc-wasm and fsc-py.
7. Log non-obvious rationale in `native/HASHOUT_LOG.md`, not in comments.

## Gate

Done when the crate builds, the dirty range reported matches what changed, and undo then redo restores the buffers exactly. Verification steps: `sub-skills/verification/SUB-SKILL.md`.

## Hands back

The edit, the dirty-range contract it preserves, and the verification output.

## Owned resources

None bundled. Neighbours: `sub-skills/sculpt-brushes/SUB-SKILL.md`, `sub-skills/topology/SUB-SKILL.md`.
