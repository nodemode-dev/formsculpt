---
name: engine-core
description: "Covers fsc-core, the formsculptor mesh and scene foundation in native/: SoA mesh buffers, adjacency, normals, BVH, scene, masks, face groups, vertex colour, sculpt layers and region delta undo. Load when editing mesh data layout, picking or raycast, undo, masks, groups or layers."
---

# Engine Core

## Purpose

Own `fsc-core`. Every other crate reads and writes its types, so a change here has the widest blast radius. Hands back a core change that keeps buffer layout, dirty tracking and undo coherent.

## Preconditions

State: v0.1.0 scaffold, crate is a minimal shell. Treat each item below as planned unless the code in `native/` shows it exists.

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
