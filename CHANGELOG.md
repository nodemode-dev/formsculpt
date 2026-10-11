# Changelog

Versions follow `v*.*.*` with dots. Dates are UTC.

## 0.9.0 · 2026-10-11

### Added

- `get_view`: renders the scene to a PNG on the CPU with the window's shading (matcaps, PBR, paint, masks), from named views or any yaw and pitch, and returns it to agents as an MCP image (54 commands in all). `/api/engine/view` serves the same picture.
- Send to NodeMode: standalone, the engine uploads the scene as GLB with a rendered preview and announces it to NodeMode (formvis's protocol, with a NodeMode address in the File section); embedded in the NodeMode portal, the window hands both to the host page (`host.ts`).
- Desktop: a native save dialog for Save, Export and the maps zip; the AppImage, deb and rpm build on Linux here.

### Fixed

- The frozen engine now carries the compiled engine module (it is rebuilt when missing or out of date), and its self-check fails unless a real command runs.
- The engine exits when the desktop shell that started it is gone, so a crashed or force-quit app no longer leaves it running.
- The desktop build had never run: the Tauri crate (2.12) and npm API (2.11) were mismatched, which the bundler refuses; the shell is pinned to formvis's tested set (tauri 2.11.6).
- Desktop CI on Windows: the scripts now find a Windows venv; the Linux leg installs libxdo.

## 0.8.1 · 2026-10-11

### Added

- `skeleton_mesh`: a watertight, even base mesh from spheres joined by tapered limbs, smoothly merged (biped watertight at every resolution from 48 to 256, 15 to 855 ms).
- `curve_tube`: a tube swept along a smooth curve, tapered by per-point radius, capped or closed into a ring.
- `lathe`: a solid spun from a [radius, y] profile (53 commands in all).
- A Build section in the window: skeleton presets (biped, quadruped, hand, snake), lathe profiles (vase, bottle, goblet, column) and the tube radius; a Tube brush that draws a tube on the plane through the view's centre.

### Fixed

- Implicit meshes go through the non-manifold split, as booleans do: thin limbs within one voxel left 2 to 4 non-manifold edges.

## 0.8.0 · 2026-10-11

### Added

- `quad_remesh`: an even, quad-dominant mesh near a target face count (within 6 percent in the checks), relaxed on the original surface; OBJ export writes the quads.
- `project_detail`: move an object's vertices onto another object's surface, for the remesh, divide and project workflow.
- `extract_mask` (a closed shell from the masked area), `extrude_mask` and `inset_mask` (50 commands in all).
- Quad remesh with a target, Project from, and Extract, Extrude and Inset mask in the window.

### Changed

- Duplicate in the window copies in place, as ZBrush does, instead of one object-width to the side.

## 0.7.1 · 2026-10-11

### Added

- `uv_unwrap`: projection charts packed into the unit square (UVs per corner, cleared by topology changes).
- `bake_maps`: colour and roughness-metal maps from paint, a tangent-space normal map and a 32-bit EXR displacement map from the highest subdivision level or another object (`source_id`), at 256 to 4096 pixels, optionally written to a folder (45 commands in all).
- GLB export with UVs, tangents and embedded colour, roughness-metal and normal textures (Khronos-valid; renders in three.js); `.fsc` keeps UVs and maps; `/api/engine/maps/{id}` downloads the four files as a zip.
- Unwrap UVs, Bake maps (with a size menu) and Download maps in the Paint section.

## 0.7.0 · 2026-10-11

### Added

- Vertex paint: colour, roughness and metalness per vertex. The `paint` brush on `sculpt_stroke` (with `color`, `roughness`, `metalness`), `paint_fill` held by the mask and `paint_clear` (43 commands in all), each with exact undo.
- Paint follows subdivision levels, dyntopo, remesh, decimate, repair, booleans, split and merge, and is saved in `.fsc`, read and written in PLY and OBJ (vertex colours) and GLB (COLOR_0 plus a material with the average roughness and metalness).
- The viewport shows paint live while painting, with a shading menu: five matcaps (clay, red wax, pearl, bronze, gray) and a lit PBR mode that uses the painted roughness and metalness. The Paint panel section and brush.

### Fixed

- Stepping down from a rebuilt lower level (multires_reconstruct) put the mask on the wrong vertices; levels now record which upper vertex each lower vertex is.

## 0.6.0 · 2026-10-11

### Added

- `.fsc` projects: `save_project` and `open_project` keep objects, masks, subdivision levels and sculpt layers and reopen identical; the service autosaves every minute after a change and can recover it.
- `import_mesh` for OBJ, STL, PLY, glTF/GLB (embedded buffers) and 3MF; `export_mesh` to OBJ, STL, PLY, GLB (sculpt layers as morph targets; passes the Khronos glTF validator) and 3MF.
- `print_check`: watertight, non-manifold, inside-out, size, volume and area in millimetres, sampled wall thickness, and a printable verdict. `repair_mesh` splits touching surfaces, fills holes and drops degenerate triangles (41 commands in all).
- The File section (Open, Save, Export with a format menu, Print check, Repair, Recover) and Mod+S, Mod+O, Mod+E; the service routes `/api/engine/import`, `/export`, `/project`, `/autosave`.

### Fixed

- After opening a project the window could keep drawing a cached mesh from the previous scene with the same object id and revision; loaded objects now get fresh revisions and ids never go back.

## 0.5.0 · 2026-10-11

### Added

- `boolean`: union, difference and intersection of two objects through a voxel grid, watertight, with exact whole-scene undo. Non-manifold edges that surface nets leave along thin rims are split away.
- Object commands `duplicate_object`, `rename_object`, `merge_objects`, `split_parts`, `transform_object` (move, rotate, scale around the bounds centre) and `deform_object` (bend, twist, taper); the mask holds vertices in place for transforms and deformers.
- Sculpt layers: `layer_add`, `layer_select`, `layer_set_strength`, `layer_delete`, `layer_apply`; strokes record into the active layer and undo exactly. `get_scene` lists each object's layers and active layer (35 commands in all).
- The Object and Layers panel sections.

### Changed

- The voxel remesh is now split into a distance-field stage and a surface stage shared with booleans; its output is unchanged.
- Remesh resolution in the panel is now Voxel resolution and also sets boolean resolution.

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
