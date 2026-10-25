---
name: ui-shell
description: "Covers the formsculpt frontend shell in frontend/src/ui, frontend/src/engine and frontend/src/styles: vanilla TypeScript and Vite UI, panels, command palette, menus, shortcuts, colour and z-index tokens, inline SVG icons, and the Web Worker engine client. Load for any UI, styling or frontend build change."
---

# UI Shell

## Purpose

Own the shell. Hands back UI work that follows the dev-rules laws and never touches engine logic.

## Preconditions

State at 0.10.0: `frontend/src/app.ts` builds the right panel in this order: File, Add, Build, Brush, Paint, Edit, Topology, Multiresolution, Object, Layers, View, note, Active mesh, Engine, History (fixed height, scrolls), Scene. The Scene list must stay last because it changes length; anything placed below it shifts (measured once at 0.0009). Window-side tools ride the viewport's stroke hook (`viewport.strokes`): tube, box and lasso masks draw in the stage's SVG overlay (`.stage-overlay`, pointer events off) and send one command on release; the topology mask picks and sends. Lists (Scene, History, layer options) reuse their nodes and patch text and attributes with `setText` and `setAttr`. Verify each change in Chromium under 4x CPU throttling (CDP `Emulation.setCPUThrottlingRate`) with a `PerformanceObserver` summing `layout-shift` entries; the total must be 0. At 0.12.0 a Keys section sits after View: every canvas key goes through `src/ui/hotkeys.ts` (`Hotkeys`), whose defaults are the registry's declared `keys` from the catalogue plus the window's own (`WINDOW_KEYS`), with the person's overrides in localStorage; never add a hard-coded key branch to `SceneView.key`, add a default instead (a registry key in `fsc-commands`, or `WINDOW_KEYS` for window-only actions). The drag stamp is a shape tool like the masks. At 0.13.0 drag dot and the curve stroke are shape tools too; the curve's handles are 3D points redrawn into the overlay on every frame (`SceneView.drawCurve` from `Viewport.onFrame`), and strokes the window sends whole (stamps, drag dots, curves) build their arguments with `brushArgs` from the last sculpting brush, so a new Brush setting must be added to `brushState`, to the stroke client's begin message and to `brushArgs` alike. The falloff editor is a fixed-height SVG. At 0.15.0 a Lighting section sits between View and Keys (environment, rotation, exposure, fixed lights, tone mapping, background and its blur, five post effects, Load HDRI); its controls only set `renderer.lighting` and `setPost` and invalidate, so they never reach the engine. At 0.16.0 the Transform and Lattice tools are shape tools drawn by `SceneView.drawTools` (from `Viewport.onFrame` and on tool change); a press that misses their handles sets `shape.miss`, which the stroke hook's `missed()` turns into camera movement. Gizmo previews go through `renderer.setPreview` and are cleared by `upload`, never by the command's reply, or the object flashes back for a frame. Touch input: two fingers pan and zoom in `Viewport`, and a finger's stroke starts only after 6 pixels of movement. At 0.19.0 the Silhouette tool is a shape tool allowed with nothing selected (`enabled()`); `SceneView.silhouetteEnd` keeps the outline as front, side or top by the view's dominant axis until Build shadow box sends them. Interface customization lives in `src/ui/layout.ts` (`PanelLayout`, applied at the end of `buildShell`): every `.panel > h2` folds the siblings up to the next h2 with `.fold-hidden`, so a new section must start with its own h2 or it folds with the one above; the panel side (`#app.panel-left`) and size (`.panel-small`, `.panel-large` zoom inside the fixed column) are kept in localStorage `formsculpt.layout` and never reach the engine. The tool rail holds buttons the person pins from the panel (context menu on any panel button); a rail button only clicks its source and mirrors its `aria-pressed` and `disabled`, so never give a rail button behaviour of its own, and keep panel button labels stable, because pins are stored as "Section/Label". At 0.20.0 VDM stamp is a click tool (`clickTool`), Lasso cut and Lasso trim are shape tools ending in `cutEnd`, which takes the outline onto the plane through the orbit centre facing the view. At 0.21.0 Insert brush and Group brush end in `strokeTool`, which picks the stroke every 6 pixels and sends one command; animation keys live in `SceneView.animKeys` and record through `/api/engine/animation`. At 0.24.0 `src/ui/spacemouse.ts` reads 3Dconnexion devices over WebHID and moves the camera each frame; test it with a stand-in `navigator.hid` that dispatches `inputreport` events, since no device is at hand. Object materials (subsurface, transmission) come from the scene summary into `renderer.setMaterial` on every refresh. At 0.25.0 `SceneView.smoothPreview` (the View section's Smooth preview) makes every fetch ask for `?smooth=N`; under it the drawn vertices are not the sculpted ones, so never patch live stroke updates in while it is on, and leave the revision behind at stroke end so the refresh fetches the smoothed result. At 0.26.0 the UV section's square `.uv-view` is always reserved; its image and empty line take turns with the hidden attribute, which needs `.uv-view [hidden] { display: none; }` because the elements carry their own display rules. At 0.28.0 `.panel-note` is a fixed three-line box (it only had a minimum height, so a wrapping note moved everything below it); keep it fixed. Live boolean draws the pair's result as renderer id 0 and leaves its two objects out of `apply`'s keep set.

## Procedure

1. Stack: vanilla TypeScript and Vite. No frameworks added without a plan decision.
2. Vite: port 5260, server.host false, strictPort, COOP/COEP headers (WASM threads), `/api` proxy to 127.0.0.1:8260.
3. `frontend/src/engine/` is the worker client and never touches the DOM. The shell talks to the engine only through it.
4. Colours come from `frontend/src/styles/tokens.css` (accent #f97316). Every z-index is a `var(--z-*)` token from its scale.
5. Icons: inline currentColor SVG in `frontend/src/ui/icons.ts`. No icon library, no emoji, and never the three-star sparkle for anything AI: Claude Code is the robot head (`agent`), Chat the speech bubble (`agentChat`). The tablet layout's own set falls back to these through `glyphBody`, so the panel, tablet and ZBrush layouts draw the same shapes.
6. Update the DOM surgically; no wholesale re-render, no flash or flicker.
7. Menus, palette entries and shortcuts derive from the catalogue (`sub-skills/command-registry/SUB-SKILL.md`).
8. No em dash in markdown or UI copy; near-zero comments; rationale into `frontend/HASHOUT_LOG.md`.

## Languages

26 interface languages, English the source and the key (`frontend/src/ui/i18n.ts`, tables in `frontend/src/i18n/[code].json`, regional es-MX and fr-CA falling back to es and fr). Text set in code goes through `t()`; everything else is caught by the page translator, which skips anything inside `[data-no-i18n]`, the terminal, chat, object names and history. A new interface string lands in every table in the same change: run `./formsculpt i18n extract`, add the translations, then `./formsculpt i18n check` must show no placeholder mismatch. Text built at run time is a `{name}` template in the tables.

## Gate

Done when the throttled DOM check is clean and a grep shows no raw z-index numbers or icon imports in the change (`sub-skills/verification/SUB-SKILL.md`).

## Hands back

The edit and the DOM check output.

## Owned resources

None bundled. Related: `sub-skills/render-viewport/SUB-SKILL.md`.
