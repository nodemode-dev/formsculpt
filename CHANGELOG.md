# Changelog

Versions follow `v*.*.*` with dots. Dates are UTC.

## 0.4.1 · 2026-10-11

### Added

- Multiresolution levels: `multires_divide`, `multires_set_level` (a level number, or one step up or down), `multires_delete_higher`, `multires_delete_lower`, and `multires_reconstruct`, which rebuilds a lower level for a mesh with subdivision structure (23 commands in all). Edits on a lower level carry up with the higher detail kept in a tangent frame; each is one undo step. The Multiresolution panel section, the Level readout and the keys Mod+D, D and Shift+D.
- Dynamic topology collapses edges shorter than 0.4 of the detail size, and `dyntopo_size` sets a fixed edge length; the panel's Detail mode offers brush, constant and screen detail.
- `get_scene` reports each object's `level` and `levels`.

### Changed

- Strokes refit only the BVH nodes they touched: a sample costs 0.21 ms on 327,680 triangles and 0.70 ms on 1,310,720 (3.6 and 14.6 ms in 0.4.0).
- Mod+D now divides; `subdivide` keeps its one-step behaviour without a key.
- While an object has levels, `subdivide`, `remesh_voxel`, `decimate`, `fill_holes` and dyntopo strokes refuse with `has_levels`.

## 0.4.0 · 2026-10-11

### Added

- Topology in `fsc-topo`: Loop subdivision, voxel remesh (narrow-band distance field, pseudo-normal signs, surface nets), quadric decimation, hole filling and edge utilities.
- Dynamic topology for strokes (`dyntopo` on `sculpt_stroke` and in the Brush panel): edges under the brush split to the detail size, with curved midpoints and tangential relaxation.
- Commands `subdivide`, `remesh_voxel`, `decimate`, `fill_holes`, `mesh_check` (18 in all), each topology change one undo step.
- The Topology panel section, and live mesh refetch during dyntopo strokes.

## 0.3.0 · 2026-10-11

### Added

- Sculpting in `fsc-sculpt`: strokes with dab spacing, pressure, lazy smoothing and four falloffs; brushes draw, clay, inflate, pinch, crease, flatten, smooth, grab and mask; mirror x, y, z and radial symmetry.
- Vertex topology tables, per-object masks and BVH refit in `fsc-core`.
- Undo and redo for strokes, mask changes, adding and deleting objects and new scenes.
- Commands `sculpt_stroke`, `mask_invert`, `mask_clear`, `mask_blur`, `mask_sharpen`, `undo`, `redo` (13 in all).
- Live strokes from the viewport over `/api/engine/stroke` with partial buffer uploads; masks shown in the viewport; the Brush and Edit panels and the undo keys.
- The MCP server at `/mcp` over the engine's catalogue, the `sculpt_strategy` prompt, the consent dialog, live window refresh after an agent's change, `./formsculptor mcp`, `./formsculptor agents sync|install`, `.mcp.json`, `.codex/config.toml`, `.cursor/mcp.json` and the `formsculptor-modeling` skill.

## 0.2.0 · 2026-10-11

### Added

- The mesh core in `fsc-core`: contiguous mesh buffers with area-weighted normals, seven primitives (icosphere, uv sphere, welded cube, cylinder, cone, torus, plane), a binned-SAH triangle BVH with raycasts and brush-radius vertex queries, and a revisioned scene.
- `./formsculptor bench`, timing mesh build, normals, the BVH, raycasts and radius queries into `bench-results/`.
- Five registry commands: `get_scene`, `new_scene`, `add_primitive`, `select_object`, `delete_object`; commands now run against an engine session.
- The `fsc_engine` PyO3 module (`native/crates/fsc-py`), built by maturin into the backend; `./formsculptor engine build` and `install` build it.
- Engine routes: `/api/engine/catalogue`, `/api/engine/dispatch` and `/api/engine/mesh/{id}` (binary); capabilities report the engine module.
- The WebGL2 viewport: clay matcap shading, ground grid, orbit, pan, zoom, snap views, orthographic toggle, frame; the panel adds primitives and lists the scene.

### Not yet

- The WASM build of the engine for the window (the wasm32 target could not be fetched on the build machine) and the WebGPU renderer.

## 0.1.0 · 2026-10-11

### Added

- The scaffold: the `./formsculptor` dispatcher with one script per command in `setup/`, and Windows twins for install and boot.
- The Rust engine workspace in `native/` with six crates, and `fsc-commands`, the single command registry, with its first command (`system.info`, MCP name `get_engine_info`) and the `fsc-catalogue` exporter.
- The FastAPI engine service on 127.0.0.1:8196 with health, version and capabilities, cross-origin isolation headers, and the built frontend served from `/`.
- The frontend shell on Vite (port 5196, loopback only): tokens, the stacking scale, inline icons, theme applied before first paint, a live service status.
- The Tauri 2 desktop shell in formvis's shape, with a content security policy and the `.fsc` file association.
- CI for the frontend build, the engine check, the backend self-check and probe, and the script and text hygiene scans; the desktop release workflow; the wiki workflow.
- The `formsculptor-harness` project skill pack.
