---
name: ui-shell
description: "Covers the formsculptor frontend shell in frontend/src/ui, frontend/src/engine and frontend/src/styles: vanilla TypeScript and Vite UI, panels, command palette, menus, shortcuts, colour and z-index tokens, inline SVG icons, and the Web Worker engine client. Load for any UI, styling or frontend build change."
---

# UI Shell

## Purpose

Own the shell. Hands back UI work that follows the dev-rules laws and never touches engine logic.

## Preconditions

State at 0.10.0: `frontend/src/app.ts` builds the right panel in this order: File, Add, Build, Brush, Paint, Edit, Topology, Multiresolution, Object, Layers, View, note, Active mesh, Engine, History (fixed height, scrolls), Scene. The Scene list must stay last because it changes length; anything placed below it shifts (measured once at 0.0009). Window-side tools ride the viewport's stroke hook (`viewport.strokes`): tube, box and lasso masks draw in the stage's SVG overlay (`.stage-overlay`, pointer events off) and send one command on release; the topology mask picks and sends. Lists (Scene, History, layer options) reuse their nodes and patch text and attributes with `setText` and `setAttr`. Verify each change in Chromium under 4x CPU throttling (CDP `Emulation.setCPUThrottlingRate`) with a `PerformanceObserver` summing `layout-shift` entries; the total must be 0. At 0.12.0 a Keys section sits after View: every canvas key goes through `src/ui/hotkeys.ts` (`Hotkeys`), whose defaults are the registry's declared `keys` from the catalogue plus the window's own (`WINDOW_KEYS`), with the person's overrides in localStorage; never add a hard-coded key branch to `SceneView.key`, add a default instead (a registry key in `fsc-commands`, or `WINDOW_KEYS` for window-only actions). The drag stamp is a shape tool like the masks. At 0.13.0 drag dot and the curve stroke are shape tools too; the curve's handles are 3D points redrawn into the overlay on every frame (`SceneView.drawCurve` from `Viewport.onFrame`), and strokes the window sends whole (stamps, drag dots, curves) build their arguments with `brushArgs` from the last sculpting brush, so a new Brush setting must be added to `brushState`, to the stroke client's begin message and to `brushArgs` alike. The falloff editor is a fixed-height SVG. At 0.15.0 a Lighting section sits between View and Keys (environment, rotation, exposure, fixed lights, tone mapping, background and its blur, five post effects, Load HDRI); its controls only set `renderer.lighting` and `setPost` and invalidate, so they never reach the engine. At 0.16.0 the Transform and Lattice tools are shape tools drawn by `SceneView.drawTools` (from `Viewport.onFrame` and on tool change); a press that misses their handles sets `shape.miss`, which the stroke hook's `missed()` turns into camera movement. Gizmo previews go through `renderer.setPreview` and are cleared by `upload`, never by the command's reply, or the object flashes back for a frame. Touch input: two fingers pan and zoom in `Viewport`, and a finger's stroke starts only after 6 pixels of movement.

## Procedure

1. Stack: vanilla TypeScript and Vite. No frameworks added without a plan decision.
2. Vite: port 5196, server.host false, strictPort, COOP/COEP headers (WASM threads), `/api` proxy to 127.0.0.1:8196.
3. `frontend/src/engine/` is the worker client and never touches the DOM. The shell talks to the engine only through it.
4. Colours come from `frontend/src/styles/tokens.css` (accent #f97316). Every z-index is a `var(--z-*)` token from its scale.
5. Icons: inline currentColor SVG in `frontend/src/ui/icons.ts`. No icon library, no emoji.
6. Update the DOM surgically; no wholesale re-render, no flash or flicker.
7. Menus, palette entries and shortcuts derive from the catalogue (`sub-skills/command-registry/SUB-SKILL.md`).
8. No em dash in markdown or UI copy; near-zero comments; rationale into `frontend/HASHOUT_LOG.md`.

## Gate

Done when the throttled DOM check is clean and a grep shows no raw z-index numbers or icon imports in the change (`sub-skills/verification/SUB-SKILL.md`).

## Hands back

The edit and the DOM check output.

## Owned resources

None bundled. Related: `sub-skills/render-viewport/SUB-SKILL.md`.
