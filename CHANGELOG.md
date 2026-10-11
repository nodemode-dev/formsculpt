# Changelog

Versions follow `v*.*.*` with dots. Dates are UTC.

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
