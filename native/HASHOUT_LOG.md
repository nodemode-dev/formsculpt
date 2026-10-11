# native HASHOUT_LOG

Living log for the Rust engine side, _v0.1.0. Read the map and the TOC first; entries are anchored by file and symbol, and removed code is quoted in full.

## Org-navi-map

```text
native/
  Cargo.toml                          workspace, [workspace.package] version (a version site)
  crates/fsc-core/src/lib.rs          modules, re-exports, ENGINE_VERSION, engine_version
  crates/fsc-core/src/mesh.rs         Aabb, Mesh (SoA positions, normals, u32 indices), normals, byte views
  crates/fsc-core/src/primitives.rs   Primitive: icosphere, uv sphere, cube, cylinder, cone, torus, plane
  crates/fsc-core/src/bvh.rs          binned-SAH triangle BVH, raycast, vertices_in_sphere
  crates/fsc-core/src/scene.rs        Scene, Object (mesh, bvh, revision), unique_name
  crates/fsc-core/src/bin/bench.rs    fsc-bench, timings as one JSON line (./formsculptor bench)
  crates/fsc-py/                      PyO3 module fsc_engine (Engine.dispatch, Engine.mesh, catalogue, version), built by maturin
  crates/fsc-commands/src/scene_cmds.rs  get_scene, new_scene, add_primitive, select_object, delete_object
  crates/fsc-sculpt/src/lib.rs        shell
  crates/fsc-topo/src/lib.rs          shell
  crates/fsc-paint/src/lib.rs         shell
  crates/fsc-io/src/lib.rs            shell
  crates/fsc-commands/src/lib.rs      Command, Group, Hints, COMMANDS, find, dispatch, catalogue
  crates/fsc-commands/src/bin/catalogue.rs  prints the catalogue JSON
```

## Contents

- [crates/fsc-core/src/mesh.rs](#cratesfsc-coresrcmeshrs)
- [crates/fsc-core/src/bvh.rs](#cratesfsc-coresrcbvhrs)
- [crates/fsc-core/src/primitives.rs](#cratesfsc-coresrcprimitivesrs)
- [crates/fsc-commands/src/lib.rs](#cratesfsc-commandssrclibrs)
- [crates/fsc-py/src/lib.rs](#cratesfsc-pysrclibrs)

## crates/fsc-core/src/mesh.rs

### crates/fsc-core/src/mesh.rs · Mesh
- refers-to: crates/fsc-core/src/mesh.rs :: Mesh, recompute_normals, positions_bytes
- kind: rationale
- logged: v0.2.0 · 2026-10-11
Positions and normals are `Vec<[f32; 3]>` and indices `Vec<u32>` so each is one contiguous little-endian block the viewport uploads as is (and that WASM memory will expose directly). glam's `Vec3` is used only for arithmetic. Normals are area-weighted (unnormalised face cross products summed per vertex); face normals run on rayon behind the default `parallel` feature, which the WASM build can turn off.

## crates/fsc-core/src/bvh.rs

### crates/fsc-core/src/bvh.rs · Bvh::build and vertices_in_sphere
- refers-to: crates/fsc-core/src/bvh.rs :: Bvh::build, sah_split, vertices_in_sphere
- kind: rationale
- logged: v0.2.0 · 2026-10-11
Triangle BVH, 12-bin SAH on the longest centroid axis, leaves of at most 8 triangles, built with an explicit stack (no recursion depth limit). Measured with `fsc-bench`: 5,242,880 triangles build in about 1.8 to 2.0 s single-threaded, rays cost 1.6 microseconds, a radius-0.05 query about 200 microseconds. `vertices_in_sphere` first used a per-query `seen` bitmap the size of the mesh; it now collects, sorts and dedups, which is cheaper than clearing millions of flags per dab.

```rust
let mut seen = vec![false; mesh.vertex_count()];
```

The removed line allocated a flag per vertex on every query; it went in 0.2.0 for the sort-and-dedup above. Next: build the top levels in parallel and refit rather than rebuild after a stroke (Phase 3).

## crates/fsc-core/src/primitives.rs

### crates/fsc-core/src/primitives.rs · cube and weld
- refers-to: crates/fsc-core/src/primitives.rs :: cube, weld
- kind: explainer
- logged: v0.2.0 · 2026-10-11
The cube is six subdivided faces welded by quantised position (epsilon size times 1e-5), so it is one closed manifold for sculpting rather than six sheets. The other kinds generate shared vertices directly. Each kind clamps its counts so a command cannot ask for an unbounded mesh.

## crates/fsc-commands/src/lib.rs

### crates/fsc-commands/src/lib.rs · COMMANDS
- refers-to: crates/fsc-commands/src/lib.rs :: Command, COMMANDS, dispatch, catalogue
- kind: rationale
- logged: v0.1.0 · 2026-10-11
The one registry. formvis kept tool names in Python, tool behaviour in a TypeScript if-chain and UI command ids in a third list; here a command is declared once with its dotted id, its MCP name, its group, label, description, default keys, MCP annotations, a schema function and a run function. `find` accepts either name, `dispatch` runs it, `catalogue` exports everything the UI, the MCP server and `agents sync` need. Schemas are hand-written `serde_json` values for now; the plan is to derive them from argument structs once the first real commands land.

### crates/fsc-commands/src/lib.rs · Session and the run signature
- refers-to: crates/fsc-commands/src/lib.rs :: Session, Command::run, dispatch
- kind: removed-code
- logged: v0.2.0 · 2026-10-11
Commands now take the engine state. `dispatch` normalises a null argument to `{}` and refuses non-objects before a command runs, so every command can assume an object.

```rust
pub run: fn(&Value) -> Result<Value, CommandError>,
```

The 0.1.0 signature had no state; replaced by `fn(&mut Session, &Value)`.

## crates/fsc-py/src/lib.rs

### crates/fsc-py/src/lib.rs · Engine
- refers-to: crates/fsc-py/src/lib.rs :: Engine, MeshBuffers, catalogue, version
- kind: rationale
- logged: v0.2.0 · 2026-10-11
The headless path. The service holds one `Engine` whose `Session` sits behind a mutex; `dispatch` takes and returns JSON strings (`{"ok": true, "result": ...}` or `{"ok": false, "error": {"code", "message"}}`), so the Python side never mirrors a command's arguments. `mesh` hands back three `bytes` objects and the revision. Built as an abi3 wheel for Python 3.12 and newer by `maturin develop` from `setup/engine_build`.

## crates/fsc-core/src/topology.rs

### crates/fsc-core/src/topology.rs · Topology
- refers-to: crates/fsc-core/src/topology.rs :: Topology::build, update_normals, ring_vertices
- kind: rationale
- logged: v0.3.0 · 2026-10-11
Compressed vertex-to-face and vertex-to-neighbour tables (offsets plus flat arrays), built once per topology change. Brushes use them to recompute normals only for the touched vertices and their one-ring, and Smooth and mask blur use the neighbour lists. `Object` now owns `topology` and a per-vertex `mask` (0 free, 1 fully masked); `Object::new` builds both.

### crates/fsc-core/src/bvh.rs · refit
- refers-to: crates/fsc-core/src/bvh.rs :: Bvh::refit
- kind: todo
- logged: v0.3.0 · 2026-10-11
A full bottom-up refit (children always sit after their parent, so a reverse sweep is enough). It runs once per stroke sample, not per dab. Measured: an interactive sample costs about 3.6 ms on 327,680 triangles and 14.6 ms on 1,310,720, most of it this refit; next is refitting only the leaves the dab touched and their ancestors.

## crates/fsc-sculpt/src

### crates/fsc-sculpt/src/stroke.rs · Stroke
- refers-to: crates/fsc-sculpt/src/stroke.rs :: Stroke::sample_ray, sample_point, grab_between, dab, finish
- kind: rationale
- logged: v0.3.0 · 2026-10-11
A stroke places dabs every radius times spacing along the path (with optional lazy smoothing), applies each to every symmetry copy (mirror planes through the origin composed with radial repeats around y), remembers each vertex's first position and mask value, and on finish turns that into one `VertexEdit` (indices, before, after, mask before and after) for the history. Brushes weight by falloff, pressure and strength, and by one minus the mask; Mask paints the mask itself. Grab captures the vertices under the first hit and moves them by the pointer's offset on the plane facing the camera. Per-dab displacement factors (0.06 times radius for Draw and Inflate, 0.08 for the Clay plane) were chosen by eye on a unit sphere and will be tuned against the reference apps' feel.

### crates/fsc-sculpt/src/lib.rs · whole file
- refers-to: crates/fsc-sculpt/src/lib.rs
- kind: removed-code
- logged: v0.3.0 · 2026-10-11
The 0.1.0 shell was replaced with the module list (a three-line file, written whole):

```rust
pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
```

`engine_version` is kept below the module list.

## crates/fsc-commands/src/history.rs

### crates/fsc-commands/src/history.rs · Edit
- refers-to: crates/fsc-commands/src/history.rs :: Edit, History
- kind: rationale
- logged: v0.3.0 · 2026-10-11
Four kinds of undo step: vertex edits, adding an object, deleting one, clearing the scene. Add and Delete share one rule: if an object is parked in the step, put it back; otherwise take it out and park it, so the same code serves undo and redo. Objects move between the scene and the history rather than being cloned. 200 steps are kept.

### crates/fsc-py/src/lib.rs · stroke_begin, stroke_sample, stroke_end
- refers-to: crates/fsc-py/src/lib.rs :: Engine::stroke_*, update_bytes
- kind: rationale
- logged: v0.3.0 · 2026-10-11
The interactive path for the window: a stroke opened with brush settings parsed by the same `parse_brush` the `sculpt_stroke` command uses, fed camera rays, and closed into one history step. Each sample returns an FSCU update: `FSCU`, count, revision, object id (u32 each), then indices, positions, normals and mask values of the changed vertices. These three methods are a transport for the viewport, not registry commands; agents use `sculpt_stroke`, which runs the same Stroke code.

## crates/fsc-topo/src

### crates/fsc-topo/src/voxel.rs · voxel_remesh
- refers-to: crates/fsc-topo/src/voxel.rs :: voxel_remesh, PseudoNormals
- kind: rationale
- logged: v0.4.0 · 2026-10-11
Grid points within two cells of any triangle get a signed distance from a BVH nearest-feature query; the sign comes from the angle-weighted pseudo-normal of the closest feature (face, edge or vertex), the Baerentzen and Aanaes method, because the first version's interpolated vertex normals gave wrong signs at sharp edges and left a grabbed cube's remesh open. Naive surface nets then place one vertex per sign-changing cell and a quad per sign-changing grid edge; the result is flipped if its signed volume is negative. Distances run on rayon. Measured on this 2-core machine: sphere, cube, torus and cylinder at resolution 64 and 128 all came out watertight in 52 to 536 ms.

### crates/fsc-topo/src/decimate.rs · decimate
- refers-to: crates/fsc-topo/src/decimate.rs :: decimate, Quadric
- kind: rationale
- logged: v0.4.0 · 2026-10-11
Garland and Heckbert quadrics (area weighted, heavy perpendicular planes on borders), a lazy heap with per-vertex version stamps, collapse to the cheapest of the midpoint and the two ends (no matrix inversion), refused when the link condition fails or a face would flip (normal turns by more than about 78 degrees). 81,920 to 20,480 triangles in 136 ms.

### crates/fsc-topo/src/dyntopo.rs · refine_sphere
- refers-to: crates/fsc-topo/src/dyntopo.rs :: refine_sphere
- kind: rationale
- logged: v0.4.0 · 2026-10-11
Splits edges longer than the detail size whose midpoints lie in the brush sphere, longest first, at most once per face per pass, two passes. New vertices bulge off the edge by a curvature estimate from the end normals (the sagitta of a circle through both ends) so splits do not flatten a curved surface. The stroke then relaxes the region tangentially twice and rebuilds topology and BVH once per sample. Refinement only; no edge collapse yet.

### crates/fsc-topo/src/subdivide.rs · loop_subdivide
- refers-to: crates/fsc-topo/src/subdivide.rs :: loop_subdivide
- kind: explainer
- logged: v0.4.0 · 2026-10-11
Loop subdivision (Warren's beta, boundary rules), or a flat split when `smooth` is false; `parents` maps each new vertex to the edge it came from so the mask is averaged into the children.

### crates/fsc-core/src/multires.rs · Multires, Stencil, Object::set_level
- refers-to: crates/fsc-core/src/multires.rs :: Stencil, Level, Multires, frames, Object::divide, Object::undivide, Object::set_level, Object::delete_higher, Object::delete_lower
- kind: rationale
- logged: v0.4.1 · 2026-10-11
A level stack lives on the object. The live level's mesh is `Object.mesh` (so strokes, the BVH and the window need no change); every other level keeps its mesh in its `Level`. Each level above the base keeps the Loop stencil from the level below (CSR rows of source and weight), because `loop_subdivide` numbers edge vertices in hash-map order and could not be re-run to the same indices. Stepping down stores the higher level as offsets from the subdivided lower level in a per-vertex frame (area-weighted normal, tangent toward a fixed neighbour), so a lower-level grab or bend turns the detail with the surface instead of shearing it. Stepping up rebuilds positions from those offsets. The mask follows the level: averaged up through the stencil, cut to the even vertices going down. Measured: a level round trip returns positions within 6e-8; a sphere with three levels to 1,310,720 triangles steps down to 0 in 83 ms and back up in 679 ms, most of it the BVH rebuild. Not built: reconstructing lower levels for a mesh that never had them.

### crates/fsc-commands/src/multires_cmds.rs · divide, set_level, delete_higher, delete_lower, guard
- refers-to: crates/fsc-commands/src/multires_cmds.rs :: divide, set_level, prune, guard; crates/fsc-commands/src/history.rs :: Edit::Divide, Edit::Level, Edit::Levels
- kind: rationale
- logged: v0.4.1 · 2026-10-11
Undo of a divide parks the removed level's mesh and stencil and redo puts them back, so later strokes replay on the same vertex numbers; a level change replays as a step; a delete swaps the whole stack, which is the only copy. While an object has levels, whole-mesh changes (subdivide, remesh, decimate, fill holes, dyntopo strokes) refuse with `has_levels`, as ZBrush does, because they would orphan every stored level; the live stroke socket drops dyntopo silently instead and the window disables the checkbox. Divide takes Mod+D from the one-step subdivide, D steps up and Shift+D down.

### crates/fsc-topo/src/dyntopo.rs · collapse_sphere, compact
- refers-to: crates/fsc-topo/src/dyntopo.rs :: collapse_sphere, fan_neighbors, compact
- kind: rationale
- logged: v0.4.1 · 2026-10-11
Runs after each `refine_sphere` and collapses edges shorter than 0.4 of the detail size, shortest first, to their midpoint. It refuses when an end is on a border or non-manifold (every fan neighbour must appear exactly twice), when the link condition fails, when either opposite vertex or the merged vertex would drop under valence 3, when a remaining face would turn more than about 72 degrees or vanish, or when the mask is over half. One collapse locks both one-rings for the rest of the pass. Collapsed vertices stay in the buffer unreferenced while the stroke runs (the stroke and the window keep their vertex numbers) and `compact` drops them when the stroke ends. Measured: a smooth stroke with constant detail 0.15 on an 81,920-triangle sphere left 78,676 triangles, watertight, no non-manifold edges, no orphans.

### crates/fsc-core/src/bvh.rs · refit_triangles
- refers-to: crates/fsc-core/src/bvh.rs :: Bvh::refit_triangles, parent, leaf_of; crates/fsc-core/src/scene.rs :: Object::refit_vertices
- kind: rationale
- logged: v0.4.1 · 2026-10-11
Each triangle knows its leaf and each node its parent. A max-heap of node indices refits touched leaves, then their ancestors (children always sit after their parent, so popping the largest index first finishes both children before the parent), and stops climbing where a box did not change. Above a quarter of the leaves it falls back to the full sweep. Measured in-process: a draw stroke sample costs 0.21 ms on 327,680 triangles and 0.70 ms on 1,310,720, against 3.6 ms and 14.6 ms with the full refit in 0.4.0.
