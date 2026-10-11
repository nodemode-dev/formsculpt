---
name: render-viewport
description: "Covers frontend/src/viewport in formsculptor: the WebGPU renderer with WebGL2 fallback that reads WASM buffers by dirty range. Load when changing shading, camera, picking display, buffer upload, render performance, or viewport overlays."
---

# Render Viewport

## Purpose

Own `frontend/src/viewport/`. Draws the mesh the engine owns. Hands back a render change that uploads only what changed and never touches engine logic.

## Preconditions

State: v0.1.0 scaffold; renderer features are planned until the code exists. Read `sub-skills/ui-shell/SUB-SKILL.md` for the tokens and laws shared with the shell.

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
