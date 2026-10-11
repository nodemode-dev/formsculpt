---
name: render-viewport
description: "Covers frontend/src/viewport in formsculptor: the WebGPU renderer with WebGL2 fallback that reads WASM buffers by dirty range. Load when changing shading, camera, picking display, buffer upload, render performance, or viewport overlays."
---

# Render Viewport

## Purpose

Own `frontend/src/viewport/`. Draws the mesh the engine owns. Hands back a render change that uploads only what changed and never touches engine logic.

## Preconditions

State at 0.7.0: `frontend/src/viewport/gl-renderer.ts` draws WebGL2 meshes with attributes position, normal, mask, paint colour (location 3) and roughness and metalness (location 4) from one interleaved five-float buffer, created when the first paint arrives; unpainted meshes use constant attribute values. `SHADINGS` holds the matcap presets (clay, red wax, pearl, bronze, gray, as procedural parameters) and the lit PBR mode (GGX with three lights, a hemisphere ambient and Reinhard tone mapping); `setShading` picks one. Payloads: FSCM flag 2 adds paint after the mask, and FSCU appends paint when the byte length allows. Not built: WebGPU, HDRI, SSAO, depth of field, showing baked maps in the viewport. At 0.9.0 `native/crates/fsc-paint/src/render.rs` is the headless render: a CPU z-buffer rasteriser (row bands in parallel, perspective-correct barycentrics, 2x2 supersampling up to 1024) that shades with the same matcap and PBR formulas and base colours as `MESH_FS`; keep the two in step when a shading changes. UV unwrap and baking live in `native/crates/fsc-paint` (uv.rs, bake.rs, encode.rs) since 0.7.1; GLB export embeds the maps. At 0.12.0 shading is resolved per object (`setShading(name, id)`, `shadingOf`), and image matcaps (`addMatcap`, names `img:<file>`) are sampled on texture unit 0 by the view-space normal with a 1 by 1 `blank` bound otherwise; get_view still renders only the procedural shadings. Read `sub-skills/ui-shell/SUB-SKILL.md` for the tokens and laws shared with the shell.

## Procedure

1. Backends: WebGPU first, WebGL2 fallback. A feature must exist on both or degrade explicitly.
2. Data comes from WASM memory views. Upload by dirty range after a sculpt; do a full upload only on a count change (topology).
3. The viewport never mutates mesh data. Edits go through the worker client in `frontend/src/engine/` as commands.
4. The page needs COOP and COEP headers for WASM threads; the Vite server sets them (port 5196, strictPort, host false).
5. Colours come from `frontend/src/styles/tokens.css`; any overlay layer uses a `var(--z-*)` token.
6. No per-frame DOM work. Zero flash or flicker is a law.

## Gate

Done when both backends render the change, dirty-range upload is confirmed for a stroke, and the throttled DOM check shows no churn (`sub-skills/verification/SUB-SKILL.md`).

## Hands back

The render change and which backends were exercised.

## Owned resources

None bundled.
