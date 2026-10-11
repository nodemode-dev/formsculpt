---
name: topology
description: "Covers fsc-topo in formsculptor: dynamic topology, multires, voxel remesh, QEM decimation, quad remesh, booleans and hole fill. Load when changing mesh resolution, remeshing, decimating, boolean ops or repairing holes."
---

# Topology

## Purpose

Own `fsc-topo`. Operations that change connectivity or vertex count. Hands back a topology change that rebuilds derived data and stays undoable.

## Preconditions

State: v0.1.0 scaffold, crate is a minimal shell; every operation named here is planned until the code exists.

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
