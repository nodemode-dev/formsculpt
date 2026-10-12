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

### crates/fsc-topo/src/reconstruct.rs · reconstruct_lower
- refers-to: crates/fsc-topo/src/reconstruct.rs :: reconstruct_lower, label_from, ring; crates/fsc-core/src/multires.rs :: Object::reconstruct; crates/fsc-commands/src/multires_cmds.rs :: reconstruct
- kind: rationale
- logged: v0.4.1 · 2026-10-11
Finds the vertices a Loop or midpoint subdivision kept (even) and the ones it added on edges (odd): from a seed assumed even, every neighbour is odd, and in an odd vertex's ordered ring the even vertex across from a known one sits three places on (or at the other end of an open fan of four). A seed that contradicts itself is undone and its neighbours are tried. The all-odd triangles are the old triangles' centres and give the lower triangles with their winding. Lower positions come from twelve Jacobi sweeps that invert Loop's even-vertex rule, then the stencil from subdividing the rebuilt level is renumbered to the existing vertex order, so the live mesh is untouched. Measured: after two subdivide levels, two rebuilds return the original sphere, cube, open plane and torus within 5e-5, and an icosphere is recognised as subdivided; 1,310,720 triangles rebuild in 1.46 s. Meshes from remesh, decimate or dyntopo refuse with not_subdivided.

### process · scripted replacement in 0.4.1
- refers-to: crates/fsc-core/src/multires.rs
- kind: process
- logged: v0.4.1 · 2026-10-11
Declared slip: while multires.rs was new and not yet committed, four exact string swaps (dropping the per-level mask copies) were made by a Python one-off instead of the Edit tool. Every other change in 0.4.1 was a hand edit or a new file.

### crates/fsc-topo/src/voxel.rs · voxel_boolean, fill_signs, Grid
- refers-to: crates/fsc-topo/src/voxel.rs :: Grid, narrow_band, fill_signs, surface_nets, voxel_boolean, BooleanOp
- kind: rationale
- logged: v0.5.0 · 2026-10-11
The remesh was split into its two stages (a narrow-band distance field and naive surface nets) so booleans reuse them on one grid around both objects. Outside the band a field only has a sign: a flood from the grid's border through every cell that is unknown or positive marks the outside, and what it cannot reach is inside, so gaps inside the band on the outer side are not mistaken for interior. Union is the minimum of the two fields, intersection the maximum, difference the maximum of a and minus b. Measured on two sculpted spheres overlapping by a radius at resolution 128: union plus intersection volume matches the two inputs within 0.1 percent, difference plus intersection matches a; about 0.4 s each.

### crates/fsc-topo/src/manifold.rs · split_nonmanifold
- refers-to: crates/fsc-topo/src/manifold.rs :: split_nonmanifold, pairs_around
- kind: rationale
- logged: v0.5.0 · 2026-10-11
Surface nets put one vertex per cell, so where a difference leaves a wedge thinner than a voxel two sheets share cells and edges with four faces appear along the rim (30, 60 and 108 at resolution 64, 128 and 200). At each such edge the faces are sorted by angle around it and each is paired with the neighbour across the gap that lies inside the solid (the face normal points away from it); faces glued by manifold edges and by those pairs form fans, and each vertex gets one copy per fan. Every union, difference and intersection of two spheres and of a cube with a rotated cylinder at 64, 128 and 200 is now watertight with no non-manifold edges.

### crates/fsc-topo/src/deform.rs, crates/fsc-commands/src/object_cmds.rs · deformers and objects
- refers-to: crates/fsc-topo/src/deform.rs :: deform, Deformer, split_parts, merge; crates/fsc-commands/src/object_cmds.rs :: transform, deform, duplicate, rename, merge, split, boolean, set_positions
- kind: rationale
- logged: v0.5.0 · 2026-10-11
Bend, twist, taper and transform run over the object's bounds centre and length along the chosen axis, and each vertex moves by one minus its mask, so the mask doubles as ZBrush's topological masking for the gizmo's work. They record as an ordinary vertex edit. Merge, split and booleans change the object list, so their undo swaps the whole list (Edit::Scene); the next object id never goes back, so ids are not reused after an undo. Duplicate copies the current mesh and mask only, not levels or layers.

### crates/fsc-core/src/layers.rs · sculpt layers
- refers-to: crates/fsc-core/src/layers.rs :: Layer, Layers, Object::add_layer, set_layer_strength, remove_layer, record_into_layer; crates/fsc-commands/src/layer_cmds.rs; crates/fsc-commands/src/sculpt_cmds.rs :: push_stroke
- kind: rationale
- logged: v0.5.0 · 2026-10-11
A layer is a per-vertex offset and a strength; the mesh always shows base plus the sum of strength times offset, so strength changes move vertices by the difference. A stroke made while a layer is active adds its movement divided by the strength to that layer and is recorded as Edit::LayerStroke, whose undo reverses both. Strength, add, select, delete and apply snapshot the layers and positions. Layers pin the vertex count, so topology changes, dyntopo and level changes refuse with has_layers. Measured: a stroke at 50 percent sits within 6e-8 of halfway, at 0 percent within 1.2e-7 of the base; undoing the stroke and then setting strength 2 leaves the mesh unchanged.

### crates/fsc-io · formats and the project file
- refers-to: crates/fsc-io/src/formats.rs :: read_obj, write_obj, read_stl, write_stl, read_ply, write_ply, read_3mf, write_3mf; crates/fsc-io/src/gltf.rs :: read_gltf, write_glb; crates/fsc-io/src/project.rs :: save, open
- kind: rationale
- logged: v0.6.0 · 2026-10-11
All formats are hand-written over bytes so the registry (paths, for agents) and the window (uploads and downloads) share one path. OBJ splits objects on o and g and fan-triangulates polygons, negative indices allowed; STL is detected as binary by its exact length, otherwise ASCII, and welds by exact position; PLY reads ASCII and binary little-endian with any scalar types and lists; glTF reads GLB or JSON with data-URI buffers, walks the default scene's node tree with matrices or TRS and flips winding under mirrored transforms; 3MF reads the first .model part in the zip. GLB export writes base positions (the mesh minus each layer at its strength) with one morph target per layer, weights set to the strengths, names in extras.targetNames; it passes the Khronos glTF validator with no errors, warnings or infos. The .fsc project is a zip (deflate) of manifest.json and little-endian buffers per object: positions, indices, mask, every non-live level's mesh, each level's stencil and detail, and each layer's offsets; normals are recomputed on open. Measured: a scene with a bent boolean union carrying a 50 percent layer and a three-level torus with a mask saves to 1.7 MB in 0.3 s and reopens in 0.05 s with every position, index and mask byte equal, and stays equal after stepping levels and changing the layer in both. Every format round-trips 126,444 triangles with equal bounds. Dependencies added: zip 2 (MIT) with deflate only, base64 0.22 (MIT or Apache-2.0).

### crates/fsc-commands/src/file_cmds.rs · print_check, repair, open_data
- refers-to: crates/fsc-commands/src/file_cmds.rs :: print_check, repair, open_data, import_data, export_data
- kind: rationale
- logged: v0.6.0 · 2026-10-11
The print check reports watertightness, non-manifold edges, inside-out volume, size, volume and area in millimetres (units_per_mm, default 0.1 so the default sphere is 20 mm), and wall thickness by casting from up to 4,000 vertices along the inward normal to the opposite surface; it lists problems and a printable flag. Repair runs the boolean's non-manifold split, hole fill, degenerate and orphan removal as one topology undo step. Opening a project clears history, offsets every loaded object's revision by a load counter shifted 20 bits (inside the 32 bits the mesh header carries) and keeps the object id high-water mark, because the window caches meshes by id and revision and drew a stale merged STL over a reopened torus (z-fighting dots) before this.

### crates/fsc-core/src/mesh.rs and callers · vertex paint
- refers-to: crates/fsc-core/src/mesh.rs :: Mesh.paint, Paint, DEFAULT_PAINT, mix_paint, ensure_paint; crates/fsc-core/src/multires.rs :: Stencil::apply_paint, pick, Level.evens; crates/fsc-topo/src/dyntopo.rs; crates/fsc-topo/src/transfer.rs :: transfer_paint; crates/fsc-sculpt/src/stroke.rs :: BrushKind::Paint, VertexEdit paint; crates/fsc-commands/src/sculpt_cmds.rs :: paint_fill, paint_clear, parse_color
- kind: rationale
- logged: v0.7.0 · 2026-10-11
Paint is five floats per vertex (colour, roughness, metalness) stored on the Mesh, empty until first painted, so every place that copies or swaps a mesh (undo snapshots, the dyntopo snapshot, level meshes, duplicate, the project file) carries it without new code; only code that changes the vertex count has to. Subdivision and level steps use the same stencils as the mask; dyntopo averages the two ends on a split and on a collapse; whole-mesh rebuilds (remesh, decimate, repair, booleans, split) sample the old surface at each new vertex with barycentric weights. Rebuilt lower levels now keep `evens`, the upper vertex for each lower vertex, because their numbering does not put the kept vertices first; before this the mask stepped down onto the wrong vertices for a rebuilt level. Measured: a paint stroke marked 80 vertices and undid to none; a metalness fill skipped fully masked vertices; paint followed two divides, a level 0 stroke up to level 2, a remesh, a union with an unpainted cube, a dyntopo stroke and a save and reopen (byte-identical); PLY round-trips colour to 8-bit precision, OBJ (v x y z r g b) and GLB (COLOR_0, validated) exactly.

### crates/fsc-paint · unwrap, expand, bake, encode
- refers-to: crates/fsc-paint/src/uv.rs :: unwrap, expand, Expanded; crates/fsc-paint/src/bake.rs :: bake, Baked; crates/fsc-paint/src/encode.rs :: png_rgba, exr_float; crates/fsc-commands/src/bake_cmds.rs; crates/fsc-io/src/gltf.rs :: write_glb
- kind: rationale
- logged: v0.7.1 · 2026-10-11
UVs are per corner on the Mesh (Mesh.uvs), cleared by dyntopo and dropped by any rebuild, so they can never point at the wrong triangles. Unwrap labels each face by the axis direction it faces, smooths labels three times by neighbour vote (only toward a direction within 70 degrees of the face), grows charts of equal labels across shared edges, projects each chart onto its axis plane and shelf-packs the bounding boxes tallest first, growing the square by 5 percent until they fit, with 4 texels of padding at 1024. It is fast and seam-light on sculpts but stretches up to about 1.4 on 45 degree faces and does not minimise distortion; a conformal unwrap is a later step. `expand` splits vertices where UVs differ (one entry per vertex and UV pair) and builds per-vertex tangents from UV gradients with a handedness sign; the bake and the GLB writer use the same expansion, so the tangent frame a viewer reads is the one the normals were baked in. The bake rasterises each triangle in UV space, then for every texel casts from the low surface point along plus and minus the interpolated normal to the source (the highest level of a copy of the object, another object, or the object itself), keeps the nearer hit within 5 percent of the bounds diagonal, and writes the hit normal in the texel's tangent frame, the signed distance, and the paint at the hit (or the low surface's paint when the source is unpainted); six passes of edge padding fill around charts. PNG goes through the png crate; the EXR is written by hand as one uncompressed FLOAT channel Y with the required header attributes and offset table. Measured: 81,920 triangles at level 3 baked onto the 1,280-triangle base at 1024 pixels in 0.6 s with every ray hitting; the GLB passes the Khronos validator and renders in three.js 0.160 with the crease detail and paint; OpenEXR 3.3.2 reads the displacement as a 1024 square float channel with the bake's range. Dependency added: png 0.17 (MIT or Apache-2.0).

### crates/fsc-topo/src/quad.rs, region.rs · quad remesh, projection, mask modeling
- refers-to: crates/fsc-topo/src/quad.rs :: quad_remesh; crates/fsc-topo/src/region.rs :: region_faces, extract_shell, extrude_region, project_onto; crates/fsc-core/src/mesh.rs :: Mesh.quads, quad_faces; crates/fsc-commands/src/model_cmds.rs
- kind: rationale
- logged: v0.8.0 · 2026-10-11
Surface nets already emit one quad per sign-changing grid edge as two consecutive triangles, so the quad remesh is the voxel remesh at a resolution chosen from the surface area and the target count, rerun once with the resolution scaled by the square root of the miss when the first count is off by more than about 15 percent, then relaxed tangentially and snapped back to the original surface by nearest-point queries. `Mesh.quads` records that triangles pair up; dyntopo clears it and any rebuild drops it, and OBJ export turns each pair into a quad in winding order. The edge flow follows the grid axes rather than curvature, so this is not a field-aligned remesh and takes no guides. Measured: sphere, torus and cube at 500, 3,000 and 20,000 target quads land within 6 percent, all watertight, 7 to 464 ms; a 4,402-quad OBJ has only four-sided faces, edge lengths within 20 percent of the mean. Projection casts along plus and minus each vertex normal to the source and takes the nearer hit within the distance; a 600-quad remesh divided twice and projected recovered the original's creases (seen in the viewport). The mask region is the faces whose three corners are at least half masked. Extract copies the region, offsets a second copy along the normals and closes the border with wall quads (winding chosen so the shell is consistently outward); extrude and inset give every region vertex a new copy, move the copies (along the normal, and for inset the border copies toward their faces' centroids), reattach the region faces to the copies and wall the border, then drop the orphaned old interior vertices. All of them came out watertight with no non-manifold edges in the checks, and each is one undo step.

### crates/fsc-topo/src/build.rs, voxel.rs · skeleton mesh, tube, lathe
- refers-to: crates/fsc-topo/src/build.rs :: Ball, round_cone, smin, skeleton_mesh, catmull, tube, lathe; crates/fsc-topo/src/voxel.rs :: implicit_mesh; crates/fsc-commands/src/build_cmds.rs; crates/fsc-commands/src/lib.rs :: build.skeleton, build.tube, build.lathe
- kind: rationale
- logged: v0.8.1 · 2026-10-11
The skeleton tool is the ZSphere idea done as a field instead of an adaptive skin: each link is a round cone (two spheres and the tangent cone between them, from the published distance formula), unlinked spheres stay spheres, and they join by a polynomial smooth minimum with k = blend times the mean radius, so joints bulge softly rather than creasing. `implicit_mesh` samples any distance function on the voxel grid in parallel and runs surface nets, so the result is quad-dominant (`Mesh.quads` is set) and even, ready for divide or dyntopo. Surface nets left 2 to 4 non-manifold edges where two thin limbs pass within one cell at some resolutions (64 and 160 in the biped), so the result goes through `split_nonmanifold` as booleans do; after that the biped is watertight at every resolution tried from 48 to 256 (4,132 to 118,840 triangles, 15 to 855 ms). The tube resamples the points by a Catmull-Rom spline (radius interpolated linearly), sweeps rings along a parallel-transport frame so it does not twist, and closes the ends with centre fans or joins the last ring to the first when closed; the lathe spins the [radius, y] profile, collapses zero-radius points to poles and caps open ends flat. Both flip their winding when the signed volume comes out negative, so a profile drawn top-down or a curve in any direction is still outward. Measured: a 30-point helix tube, a tapered horn, a closed ring and two lathe profiles all watertight in 1 ms or less; duplicate points and zero-width profiles refuse with build_failed.

### crates/fsc-paint/src/render.rs, crates/fsc-commands/src/view_cmds.rs · headless render and get_view
- refers-to: crates/fsc-paint/src/render.rs :: render, View, Shading, RenderObject, Camera, raster_band, shade; crates/fsc-paint/src/encode.rs :: png_rgba_rect; crates/fsc-commands/src/view_cmds.rs :: get_view, view_schema; crates/fsc-commands/src/lib.rs :: view.get
- kind: rationale
- logged: v0.9.0 · 2026-10-11
Agents could act but not look; get_view gives them a picture without a GPU or a window. The renderer is a CPU z-buffer: vertices projected once per object (in parallel), the frame split into row bands that each rasterise every triangle clipped to the band (no locks, each band owns its rows), perspective-correct barycentrics stored per sample, then a deferred shading pass that interpolates normal, position, paint and mask and applies the viewport's formulas (the matcap reflection lookup, key, fill, rim and cavity terms, and the three-light GGX mode, with the same base colours for active and other objects), so a render matches what the person sees. Up to 1024 pixels it supersamples 2 by 2. The camera is the window's orbit camera (40 degree field of view, fitted to the bounds of all drawn objects or the active one, aspect-aware). The command returns the PNG as base64 with its facts; the service turns that into an MCP image block. Measured: 640 square in 70 to 90 ms for 20,000 to 67,000 triangles, 1024 in 223 ms, 2048 (no supersampling) in 174 ms; renders checked by eye against the viewport (clay, PBR, paint, light and transparent backgrounds, framing the active object). Dependency added to fsc-commands: base64 0.22 (already in fsc-io).

### crates/fsc-sculpt/src/stroke.rs, brush.rs · relax, planar, trim, snake hook, noise; lazy rope
- refers-to: crates/fsc-sculpt/src/brush.rs :: BrushKind (Relax, Planar, Trim, SnakeHook, Noise), BrushSettings.noise_scale; crates/fsc-sculpt/src/stroke.rs :: Stroke.planes, Stroke.snake_prev, sample_ray, sample_point (lazy rope, snake hook), snake_dab, dab, value_noise, fbm
- kind: rationale
- logged: v0.10.0 · 2026-10-11
Relax keeps only the in-surface part of the move toward the neighbours' average, so it evens triangles without wearing the form down: six passes over a sphere's cap left its mean radius at 1.0000 where smooth took it to 0.9224. Planar locks its plane (centre and normal) at the stroke's first dab, one per symmetry copy, and flattens toward it: the centre came out flat to 0.0003 (tilted by the first dab's normal, as it should). Trim moves only what stands above each dab's plane (inverted, only what lies below), so the tops of noise came down from 1.022 to 0.986 while the hollows stayed. Snake hook moves the vertices around the previous dab centre by the step to the next, and after the first hit follows the pointer on the camera-facing plane through the last dab, as grab does; its core follows fully at every strength (strength widens the pulled area by the falloff's power), because a partial move left the pulled area behind the brush and out of reach of the next dab, which stalled the hook in the window (1.045 against 1.442 after the fix). Noise displaces along the normal by three octaves of hashed value noise. Lazy is now a rope (lazy times three radii): the brush trails the pointer and moves only when the rope is taut, replacing the earlier exponential smoothing.

### crates/fsc-commands/src/mask_cmds.rs, arrange_cmds.rs, layer_cmds.rs · masks, arrange, visibility, pick, history, surface noise
- refers-to: crates/fsc-commands/src/mask_cmds.rs :: curvature, mask_cavity, mask_by_color, mask_grow, surface_distance, mask_topology, mask_shape, inside_polygon, combine; crates/fsc-commands/src/arrange_cmds.rs :: arrange, set_visibility, pick, get_history, history_go_to; crates/fsc-commands/src/history.rs :: History.steps, go_to; crates/fsc-commands/src/layer_cmds.rs :: surface_noise; crates/fsc-core/src/multires.rs :: Multires.translate; crates/fsc-core/src/layers.rs :: Object.shift (now public); crates/fsc-paint/src/render.rs :: view_coordinates; crates/fsc-commands/src/scene_cmds.rs :: summary.masked_vertices
- kind: rationale
- logged: v0.10.0 · 2026-10-11
Cavity masking took three tries. The neighbour-average curvature is biased by valence, so an icosphere's irregular vertices showed as faint stars; blurring it only spread them; the per-edge normal curvature (2 n.(q-p)/|q-p|^2, averaged) is exact on a sphere at any valence and removed them. Curvature is measured against the object's median, so a sphere has neither cavities nor peaks, and the threshold has a floor of 15 percent of the curvature's 90th percentile, so float noise on a featureless surface masks nothing (sphere and plane: 0 masked). Topology masking runs Dijkstra along edges: from the top of one of two merged cylinders 0.42 apart it masked 108 vertices, all on that cylinder, where a distance-in-space mask would have taken 170 of the other's. Shape masks take picture coordinates, either for a named view (framed as get_view frames, through view_coordinates) or for a view-projection matrix (the window's camera); points behind the camera are skipped. Every mask command combines with the current mask by replace, add, subtract or intersect. arrange_objects moves stored lower levels too (Multires.translate: lower levels hold absolute positions, higher ones translation-free detail), checked by stepping up after arranging at level 0. get_scene now reports masked_vertices per object, which agents (and the tests) need to read a mask without changing it; the earlier test helper that counted by sharpening and undoing undid the user's own step whenever the sharpen changed nothing. surface_noise writes its offsets into a new layer and shifts the mesh by them, so layer strength fades (0 gives back the exact sphere) or inverts it; one undo step restores the exact positions.

### fsc-core mesh.rs, bvh.rs; fsc-sculpt stroke.rs; fsc-commands group_cmds.rs · face groups and hidden faces
- refers-to: crates/fsc-core/src/mesh.rs :: Mesh.groups, Mesh.hidden, has_groups, has_hidden, face_hidden, hidden_vertices, groups_bytes, hidden_bytes; crates/fsc-core/src/bvh.rs :: raycast (skips hidden); crates/fsc-topo/src/subdivide.rs :: loop_subdivide_with (groups and hidden carried, sharp_border); crates/fsc-sculpt/src/stroke.rs :: free; crates/fsc-commands/src/group_cmds.rs :: group_from_mask, group_auto, group_clear, mask_by_group, hide_faces, commit; crates/fsc-io/src/project.rs :: groups.u32, hidden.u8; crates/fsc-py/src/lib.rs :: faces; crates/fsc-paint/src/render.rs :: Shading::Groups, group_color, hidden faces skipped
- kind: rationale
- logged: v0.11.0 · 2026-10-11
Groups and hidden flags are per triangle on the Mesh and count only while their length matches the triangle count, the rule UVs already follow, so any tool that rebuilds or changes triangles without carrying them simply drops them instead of pointing at the wrong faces. Subdivision carries them exactly (each parent's four children are consecutive); dynamic topology clears them explicitly. Hidden faces are skipped by the BVH ray cast (so strokes, picks and stroke starts pass through them) and by get_view; a vertex whose every face is hidden is held like a full mask through `free`, used at every brush site in place of the bare mask read. Group and hide edits swap a copy of the mesh as one Edit::Topology step. Group by angle floods across edges whose faces' normals are within the angle (a cube: 6); by parts across shared vertices (a merged sphere and torus: 2). Measured: a stroke across a hidden boundary moved 652 vertices and none inside the hidden area; the pick from above passed a hidden cap and hit the bottom; groups survived subdivide and multiresolution steps; .fsc kept groups and 24,368 hidden faces; undo restored groups. Lower subdivision levels are saved separately and do not yet keep their groups through a save.

### fsc-commands place_cmds.rs, view_cmds.rs, arrange_cmds.rs; fsc-paint encode.rs · arrays, insert, turntable, folders
- refers-to: crates/fsc-commands/src/place_cmds.rs :: array_object, insert_mesh, moved, add_object; crates/fsc-commands/src/view_cmds.rs :: record_turntable, render_pixels (get_view split into rendering and encoding); crates/fsc-paint/src/encode.rs :: apng_rgba; crates/fsc-commands/src/arrange_cmds.rs :: set_folder, set_visibility (folder); crates/fsc-core/src/scene.rs :: Object.folder
- kind: rationale
- logged: v0.11.0 · 2026-10-11
Arrays copy the mesh (paint and groups ride along) along an offset or around an axis through a centre; separate copies are recorded as one scene step (the first version made one undo step per copy, seven for a radial array of eight), merged copies as one topology step. They are copies, not live instances. Insert scales a primitive or another object's mesh to `size`, stands it along the surface normal with Quat::from_rotation_arc and sinks it by a fraction of its height; pick supplies the point and normal. The turntable renders `frames` views around the vertical axis (framed once, so it turns in place) and writes an APNG through the png crate's animation support (acTL, one fcTL per frame): 24 frames at 256 pixels in 0.28 s, 48 at 384 for the window. It writes a file, so it is a WRITE command; the service route writes a temporary file and returns it. Sharp-border subdivision keeps open-border vertices in place: a plane's corners stayed at a reach of 1.0 where Loop's border rule pulled them to 0.84. Folders are one level, saved in projects, and set_visibility can take a folder instead of ids.

### fsc-sculpt alpha.rs, stroke.rs, brush.rs; fsc-commands sculpt_cmds.rs; fsc-paint encode.rs · alphas, roll strokes, smudge
- refers-to: crates/fsc-sculpt/src/alpha.rs :: Alpha, BUILTIN, builtin, sample, from_pixels, turn; crates/fsc-sculpt/src/stroke.rs :: Stroke.alpha, prev_dab, smudge, set_alpha, dab (alpha frame, smudge branch); crates/fsc-sculpt/src/brush.rs :: BrushKind::Smudge, BrushSettings.alpha_up, alpha_angle, roll; crates/fsc-commands/src/sculpt_cmds.rs :: parse_brush (alpha_up, alpha_angle, stroke), stroke_alpha, alpha_load, alpha_load_schema; crates/fsc-commands/src/lib.rs :: Session.alphas, sculpt.alpha_load; crates/fsc-paint/src/encode.rs :: png_gray; crates/fsc-py/src/lib.rs :: stroke_begin (alpha)
- kind: rationale
- logged: v0.12.0 · 2026-10-11
An alpha is a square gray grid sampled bilinearly in the dab's own frame: the frame's up is the stroke direction when the stroke rolls, otherwise the given up (the window sends the screen's up, so a stamp stands upright on screen), projected onto the dab's area plane, then turned by alpha_angle; outside the square it gives 0, and it multiplies the falloff weight, so every brush that moves or paints takes the shape. Seven alphas are generated in code (square, noise, cracks, rings, star, stripes, dots); alpha_load decodes any PNG to gray (the mean of RGB, up to 4096 square) and resamples it to a square, kept in the session by name and refusing built-in names. A single point stamps once, which is what the window's drag stamp sends. Smudge carries the paint colour from the previous dab of the same symmetry copy, blends it in by weight and lets the carried colour drift 35 percent toward what it passes. Measured on a 7-level sphere: one dab moved 14,389 vertices plain and 6,033 with the star; turning the star 36 degrees changed 8,470; dots and roll strokes with stripes differed at 8,217; a loaded ring left its centre untouched (0 of 7,586 moved); a smudge from red into white tinted 713 of 713 vertices along its path. Process slip declared: the alpha_load append and its registry entry were written by shell heredoc and a Python string replace instead of hand edits.

### fsc-sculpt brush.rs, stroke.rs; fsc-commands sculpt_cmds.rs · drawn falloff, pressure curve, spray
- refers-to: crates/fsc-sculpt/src/brush.rs :: Falloff::Curve, Falloff::curve, CURVE_SAMPLES, BrushSettings.spray, pressure_gamma, pressure_strength, pressure_size; crates/fsc-sculpt/src/stroke.rs :: Stroke.rng, jitter, last_normal, random, sample_point (pressure curve, spray scatter), dab (pressure size, jitter); crates/fsc-commands/src/sculpt_cmds.rs :: parse_brush (falloff_curve, stroke spray, spray, pressure_gamma, pressure_strength, pressure_size), brush_props
- kind: rationale
- logged: v0.13.0 · 2026-10-11
A drawn falloff is any 2 to 64 weights from the centre to the rim, resampled to a fixed 17-entry table so BrushSettings stays Copy and every brush reads it through Falloff::weight like the presets. Pressure goes through a power curve once per sample, then scales strength, radius (down to a tenth) or both. Spray scatters each dab in the plane across the area normal of the last unmirrored dab (no nearest-point query was needed), turns its alpha at random and gives it 40 to 100 percent of the strength; the generator is a seeded xorshift per stroke, so a spray stroke replays exactly (undo and redo, agents). Measured on a 7-level sphere: a dots stroke reached 0.117 off its path, spray 1.5 reached 0.269 and moved 8,158 vertices against 3,864, and two runs were identical; a [0, 1, 0] ring moved the centre 0.004 and the middle 0.028 where smooth moved them 0.030 and 0.015; a [1, 1, 1, 0] plateau held 0.030 to half the radius; at pressure 0.5 gamma 3 cut the peak from 0.015 to 0.0038; pressure 0.3 with pressure size touched 883 vertices instead of 9,699.

### fsc-core paint_layers.rs; fsc-sculpt stroke.rs; fsc-commands paint_layer_cmds.rs, sculpt_cmds.rs, history.rs; fsc-io project.rs · paint layers
- refers-to: crates/fsc-core/src/paint_layers.rs :: Blend, PaintLayer, PaintLayers (valid, below), Object.layered, check_paint_layers, composite_paint, paint_vertex, paint_target, set_paint_target; crates/fsc-core/src/scene.rs :: Object.paint_layers; crates/fsc-sculpt/src/stroke.rs :: VertexEdit.layer, VertexEdit::apply, Stroke.layer_before, paint_target, remember, vertex_edit, dab (paint and smudge through paint_vertex); crates/fsc-commands/src/paint_layer_cmds.rs :: add, set, select, delete, merge, flatten, step; crates/fsc-commands/src/sculpt_cmds.rs :: paint_fill, paint_clear; crates/fsc-commands/src/history.rs :: Edit::PaintLayers; crates/fsc-commands/src/scene_cmds.rs :: summary.paint_layers, active_paint_layer; crates/fsc-io/src/project.rs :: paint_layers (paintbase.f32, paintlayerN.f32)
- kind: rationale
- logged: v0.14.0 · 2026-10-11
Paint layers sit above a base and are composited into Mesh.paint, which stays the one array every renderer, bake and exporter reads, so none of them changed. A layer stores colour, roughness, metalness and a coverage per vertex; the colour channels blend by mode (normal, multiply, screen, overlay, add) and roughness and metalness mix normally, each by coverage times opacity. An untouched vertex starts from what shows through it, so painting one channel leaves the others. Brushes, fills and smudge go to the active layer or the base through paint_vertex and recomposite only what they touched; a paint stroke records the target's values before and after next to the composite in the same VertexEdit, so one undo step restores both. Stack changes are one Edit::PaintLayers step that swaps the whole stack and composite. Merging down solves for the lower layer's value that reproduces the look (exact to 2e-3 over a whole sphere); flatten bakes visible layers. A stack whose arrays no longer match the mesh (after subdivide, remesh, dyntopo) is ignored rather than dropped, so undoing the topology step brings it back; layer commands drop a stale stack explicitly. Projects save the base and each layer beside the composite, so older readers still see the colours. Measured: gray base with a red layer at the front read red, 0.751/0.251/0.251 at opacity 0.5, 0.502/0/0 multiplied, 1/0.502/0.502 screened, gray hidden; a blue layer at 0.5 over red read 0.5/0/0.5; erase, undo and redo moved it between 1/0/0 and 0.5/0/0.5; reordering changed only where both covered; a green base fill showed only where the layers did not cover; save and reopen kept the layers and identical paint; flatten kept the look exactly.

### fsc-topo deform.rs; fsc-commands object_cmds.rs; fsc-sculpt stroke.rs; fsc-py lib.rs · lattice deformation and pen tilt
- refers-to: crates/fsc-topo/src/deform.rs :: lattice_box, bernstein, lattice_deform; crates/fsc-topo/src/lib.rs :: exports; crates/fsc-commands/src/object_cmds.rs :: lattice, lattice_schema; crates/fsc-commands/src/lib.rs :: object.lattice; crates/fsc-sculpt/src/stroke.rs :: Stroke.tilt, set_tilt, dab (alpha turn); crates/fsc-py/src/lib.rs :: stroke_sample (tilt)
- kind: rationale
- logged: v0.16.0 · 2026-10-11
lattice_deform is free-form deformation with Bernstein weights over the object's bounds grown by 2 percent of their largest side: a vertex moves by the weighted sum of its control points' offsets, so an untouched lattice changes nothing and any move bends the surface smoothly (a single moved point pulls with a soft, wide falloff: lifting the top-centre point of a 3 by 3 by 3 lattice by 0.6 raised a unit sphere's pole by 0.144 and left the bottom still). Masked vertices are held in proportion to the mask (a masked cap moved 0.0); one undo step restores the exact positions; out-of-range points, divisions outside 2 to 6 and an empty move list are refused. The reply carries the lattice's box so agents can place points. Pen tilt reaches the stroke as an extra turn of the alpha per sample (296 vertices differed between an upright and a tilted star stamp); the Python stroke_sample keeps its three-argument form. Process slip declared: the fsc-topo export line was changed with sed instead of a hand edit.

### fsc-sculpt noise.rs; fsc-commands layer_cmds.rs, scene_cmds.rs; fsc-core scene.rs · noise editor and editable primitives
- refers-to: crates/fsc-sculpt/src/noise.rs :: NoisePattern, NoiseSettings, noise_at, fractal, cells, hash; crates/fsc-commands/src/layer_cmds.rs :: surface_noise (pattern, octaves, gain, stretch, alpha), noise_schema; crates/fsc-commands/src/scene_cmds.rs :: add_primitive, primitive_mesh, edit_primitive, edit_primitive_schema, summary.primitive; crates/fsc-core/src/scene.rs :: Object.primitive; crates/fsc-commands/src/lib.rs :: scene.edit_primitive
- kind: rationale
- logged: v0.17.0 · 2026-10-11
surface_noise gained a pattern (value, ridged, turbulence, cells, alpha), octaves, gain and a stretch per axis; every pattern returns about -1..1 so amount reads the same. Cells is Worley distance to one jittered point per cell; the alpha pattern tiles an alpha over the surface from the three axis planes, blended by the fourth power of the normal's components so seams stay narrow. All five rendered distinctly on a sphere (the star alpha tiled cleanly). A primitive remembers the arguments it was made with and its revision; edit_primitive merges new size, detail, height or thickness and rebuilds it as one topology step while the revision still matches and it has no levels or layers, so any stroke, transform or remesh fixes its shape. Measured: a 32-segment cylinder rebuilt to 12 segments and height 4 (290 to 50 vertices, bounds 2 by 4 by 2); after a stroke the edit is refused. Undoing an edit also fixes the shape (the swap raises the revision), and the editable state is not saved in projects.

### fsc-commands sculpt_cmds.rs; fsc-paint encode.rs · projection painting
- refers-to: crates/fsc-commands/src/sculpt_cmds.rs :: paint_project, project_schema; crates/fsc-paint/src/encode.rs :: png_rgba_decode; crates/fsc-commands/src/lib.rs :: paint.project
- kind: rationale
- logged: v0.18.0 · 2026-10-11
paint_project paints an image onto vertex paint in two ways. View mode projects it like a slide projector from a named view (framed as get_view frames it) or from a view-projection matrix (the window's camera): a vertex takes the bilinear sample under it only when its normal faces the camera and a ray from it toward the camera meets nothing first, so surfaces behind other parts stay untouched (549 of 549 vertices on a sphere hidden behind another kept their paint, 251 of 251 visible ones took it), and grazing surfaces fade in so the image does not smear around the silhouette. Triplanar mode tiles the image from the three axis planes blended by the fourth power of the normal's components. Colours are taken from sRGB to linear; the image's alpha times opacity sets the weight; masks protect; everything goes through paint_vertex, so the active paint layer receives it and one undo step restores both. 163,842 vertices took 0.12 s.

### fsc-topo build.rs; fsc-commands build_cmds.rs · shadow box
- refers-to: crates/fsc-topo/src/build.rs :: polygon_sd, silhouette_mesh; crates/fsc-topo/src/lib.rs :: exports; crates/fsc-commands/src/build_cmds.rs :: shadow_box, shadow_box_schema; crates/fsc-commands/src/lib.rs :: build.shadow_box
- kind: rationale
- logged: v0.19.0 · 2026-10-12
shadow_box builds a solid from two or three outlines seen from the front (x, y), the side (z, y) and the top (x, z), the way a shadow box or a quick-mesh helper does: each outline becomes a signed distance in its plane, extruded along the axis it is seen down, and the extrusions are intersected with a smooth maximum whose width is the blend, then meshed through implicit_mesh like metaballs and skeletons, so the result is always closed. An outline is any 3 to 2,048 points; a single outline, or outlines that do not overlap, are refused with a message rather than an empty mesh. Measured: a boot from a side profile and a front rectangle at resolution 128 gave 30,208 vertices, watertight, its bounds the outlines' overlap.

### fsc-commands place_cmds.rs, lib.rs, scene_cmds.rs; fsc-core scene.rs; fsc-py lib.rs · mirror and curve arrays, linked copies
- refers-to: crates/fsc-commands/src/place_cmds.rs :: array (mode line, radial, mirror, curve; linked), transformed, sync_instances; crates/fsc-core/src/scene.rs :: Instance, Object.instance; crates/fsc-commands/src/lib.rs :: dispatch (sync after success), sync_instances; crates/fsc-commands/src/scene_cmds.rs :: summary.instance_of; crates/fsc-py/src/lib.rs :: stroke_end
- kind: rationale
- logged: v0.19.0 · 2026-10-12
Every array mode now produces a list of matrices and one copy per matrix. Mirror reflects about the object's centre plane on an axis (the copy's winding is flipped whenever a matrix has a negative determinant, so normals stay outward); curve spaces copies along a polyline by arc length and, with align, turns each to the curve's tangent. Linked copies remember their source, their matrix, the source's revision and their own; after every successful command and every finished stroke, sync_instances rebuilds a copy whose source changed (one replace_mesh, not an undo step, since undoing the source change syncs it back) and unlinks a copy that was edited itself or whose source is gone. Linked with merge is refused. Measured: a mirror copy centred at x -1 from a sphere at +1; six linked copies along a stroke followed the stroke and an undo; a copy edited directly unlinked; subdividing the source updated every linked copy.

### fsc-core primitives.rs · cube and plane faced inward
- refers-to: crates/fsc-core/src/primitives.rs :: plane_face
- kind: bug
- logged: v0.20.0 · 2026-10-12
plane_face, which builds the plane and every side of the cube, laid its cells with grid_indices, which winds them to face v x u; the faces it builds must face u x v. So since 0.2.0 the cube was inside out (print_check reported it) and the plane faced down, which a draw stroke on a plane showed by pushing it down. Each triangle is now turned over after grid_indices; the sphere, UV sphere, cylinder, cone and torus were already right. All seven primitives now pass print_check. Projects and files saved with an older cube keep their inside-out winding; repair_mesh or a voxel remesh fixes one.

### fsc-sculpt vdm.rs; fsc-commands vdm_cmds.rs · vector displacement stamps
- refers-to: crates/fsc-sculpt/src/vdm.rs :: Vdm (builtin, capture, sample, reach), VdmPlacement, vdm_stamp; crates/fsc-commands/src/vdm_cmds.rs :: vdm_stamp, vdm_capture, stamp_schema, capture_schema; crates/fsc-commands/src/lib.rs :: Session.vdms, sculpt.vdm_stamp, sculpt.vdm_capture
- kind: rationale
- logged: v0.20.0 · 2026-10-12
A vector map holds an offset across, out and along for every node of a square grid, in half-widths of the stamp, so a stamp can move the surface sideways and back over itself. Three are generated: a horn and a hook (a cone whose tip leans by the square of its height, so it passes over its base's edge) and a mushroom (the flat disc mapped onto the cap's top, round its rim, back in under it, down the stem and out over the ground). vdm_capture reads a plane made by add_primitive and sculpted: the plane's grid is known from its arguments, so each vertex's move from its flat place is the offset, less the corners' mean move; a plane whose vertex count changed (dynamic topology) is refused. The stamp moves vertices under its square within half a radius of the surface plane, held by the mask and faded over the outer tenth, as one undo step through set_positions. Measured: the mushroom on a 200-division plane gave a cap 0.374 wide over a stem 0.09 wide (an overhang a height map cannot make); the hook's tip passed its base; a bump captured from a sculpted plane raised a sphere's pole from 1.0 to 1.044; unknown maps, a flat plane and a stamp off the surface are refused.

### fsc-topo text.rs, voxel.rs; fsc-commands build_cmds.rs, object_cmds.rs, topo_cmds.rs · 3D text, vector shapes, fused booleans, cuts
- refers-to: crates/fsc-topo/src/text.rs :: glyph, text_mesh, extrude_shape, extruded, TEXT_CHARACTERS; crates/fsc-topo/src/voxel.rs :: voxel_boolean_blend, narrow_band (band), fill_signs (band); crates/fsc-commands/src/build_cmds.rs :: text, extrude_shape; crates/fsc-commands/src/object_cmds.rs :: boolean (blend); crates/fsc-commands/src/topo_cmds.rs :: cut_shape
- kind: rationale
- logged: v0.20.0 · 2026-10-12
3D text sweeps a round pen along a single-stroke font (capitals, digits and common punctuation on a 4 by 6 grid, no font files) and extrudes the result with rounded edges, meshed through implicit_mesh with the cells chosen so a stroke spans about six of them; extrude_shape does the same for closed outlines with even-odd holes. Both are watertight (print_check passed on text and on a washer whose hole a ray passes through). Booleans gained a blend: a smooth minimum or maximum of the two distance fields over that many world units, with the narrow band widened to the blend so the fillet reads true distance; a blended union of a sphere and a rod widened the rod from 0.30 to 0.36 where it meets the sphere, both results printable. cut_shape extrudes an outline through the object along the view in a right-handed frame and takes it away or keeps it with that boolean: a star cut clean through a cube (a ray through it hits nothing), trimmed to a star, and a slot with rounded edges; an outline covering everything is refused. Cut edges are as sharp as the voxel resolution allows.
