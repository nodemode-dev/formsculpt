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
