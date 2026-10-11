# frontend HASHOUT_LOG

Living log for the frontend side, _v0.1.0. Read the map and the TOC first; entries are anchored by file and symbol, and removed code is quoted in full.

## Org-navi-map

```text
frontend/
  index.html              critical style, pre-paint theme boot, #app and #overlay-root
  vite.config.ts          port 5196, host false, strictPort, isolation headers, /api proxy
  package.json            scripts dev, build, preview, typecheck, tauri
  public/favicon.svg      tab icon
  public/splash.html      the desktop splash window's page
  src/main.ts             stylesheet imports, start()
  src/app.ts              builds the shell once, polls the service, patches text in place
  src/version.ts          VERSION, a version site
  src/engine/service.ts   the only HTTP client: fetchVersion, dispatch, fetchMesh (FSCM binary)
  src/viewport/math.ts    Vec3 helpers, lookAt, perspective, ortho, multiply
  src/viewport/camera.ts  OrbitCamera: orbit, pan, zoom, frame, snap views, orthographic
  src/viewport/gl-renderer.ts  WebGL2 renderer: procedural matcap shading, ground grid, per-object buffers
  src/viewport/viewport.ts     canvas, DPR resize, render on demand, pointer and wheel navigation
  src/ui/dom.ts           el, setText, setAttr (write only on change)
  src/ui/icons.ts         inline currentColor SVG icons
  src/styles/tokens.css   colour, spacing, size tokens and the stacking scale
  src/styles/fonts.css    fontsource imports
  src/styles/shell.css    grid shell, bars, rail, stage, panel, status
```

## Contents

- [index.html](#indexhtml)
- [vite.config.ts](#viteconfigts)
- [src/app.ts](#srcappts)
- [src/styles/tokens.css](#srcstylestokenscss)

## index.html

### index.html · theme boot script
- refers-to: index.html :: inline script in head
- kind: rationale
- logged: v0.1.0 · 2026-10-11
Runs before the body exists, so the saved accent and the light profile are on the root element before first paint; no theme flash on hard reload. It reads the shared `nodemode_ui_settings` key so a NodeMode host and formsculptor agree. `#app` stays hidden until `body.app-ready`, which `start()` adds after the shell is built in one pass.

## vite.config.ts

### vite.config.ts · ISOLATION headers
- refers-to: vite.config.ts :: ISOLATION
- kind: rationale
- logged: v0.1.0 · 2026-10-11
Cross-origin isolation (COOP same-origin, COEP require-corp) is set from day one because the WASM engine planned for 0.2.0 runs threads over SharedArrayBuffer, which browsers only allow on isolated pages. The engine service sends the same headers (backend `CrossOriginIsolation`).

## src/app.ts

### src/app.ts · buildShell and poll
- refers-to: src/app.ts :: buildShell, poll
- kind: rationale
- logged: v0.1.0 · 2026-10-11
The shell is built once. `poll` only writes the status dot's `data-state` and two text nodes through `setAttr` and `setText`, which skip unchanged values, so a 5 second poll never rebuilds or reflows. The status text has a reserved `min-width` (22ch) and the version cell 9ch with tabular numerals, so the change from "connecting" to "service connected" moves nothing.

## src/styles/tokens.css

### src/styles/tokens.css · stacking scale
- refers-to: src/styles/tokens.css :: --z-* tokens
- kind: explainer
- logged: v0.1.0 · 2026-10-11
Every z-index is a `var(--z-*)`. The bands mirror formvis and NodeMode so an embedded formsculptor never collides with its host.

| Token | Value | Use |
| --- | --- | --- |
| --z-base | 0 | stage |
| --z-raised | 8 | hovered items |
| --z-selected | 9 | selected items |
| --z-rail | 10 | tool rail |
| --z-chrome | 100 | app bar, status bar |
| --z-menu | 200 | menus anchored to chrome |
| --z-temp | 500 | drag artifacts |
| --z-companion | 800 | floating panels |
| --z-taskbar | 900 | floating taskbar |
| --z-modal | 1000 | modals |
| --z-settings | 1500 | settings popout |
| --z-context | 2000 | context menus, dialogs |
| --z-portal | 4000 | extension panels |
| --z-overlay | 50000 | full-screen editors |
| --z-overlay-modal | 60000 | modals over an overlay |
| --z-debug | 100000 | development only |

Anything that must escape its container renders into `#overlay-root`, a direct child of `body`.

## src/engine/service.ts

### src/engine/service.ts · whole file
- refers-to: src/engine/service.ts
- kind: warning-context
- logged: v0.2.0 · 2026-10-11
In 0.2.0 this file was replaced whole instead of extended by edits (a SEDO slip, declared in the turn ledger). `fetchVersion` is unchanged; the 0.1.0 `Health` interface, never used, went with it:

```ts
export interface Health {
  ok: boolean;
}
```

## src/viewport/gl-renderer.ts

### src/viewport/gl-renderer.ts · MESH_FS
- refers-to: src/viewport/gl-renderer.ts :: MESH_FS
- kind: rationale
- logged: v0.2.0 · 2026-10-11
A procedural clay matcap (key, fill, rim, a highlight placed by the matcap coordinate of the reflected view vector) so the viewport needs no texture asset. WebGL2 comes first because the Linux desktop webview (WebKitGTK) has no WebGPU; the WebGPU path is the next renderer behind the same `upload`, `remove`, `render` surface. Back faces flip their normal so open meshes (the plane) shade on both sides.

## src/viewport/viewport.ts

### src/viewport/viewport.ts · invalidate
- refers-to: src/viewport/viewport.ts :: invalidate, draw
- kind: rationale
- logged: v0.2.0 · 2026-10-11
Renders on demand: one requestAnimationFrame per change, none while idle. Left drag orbits, right or middle drag (or Shift) pans, Ctrl drag or the wheel zooms. Keys on the focused canvas (in app.ts): 1, 3, 7 for front, right and top, Ctrl for the opposite side, 5 toggles orthographic, F frames the active object, Delete removes it.

## src/app.ts

### src/app.ts · SceneView and panel order
- refers-to: src/app.ts :: SceneView.apply, SceneView.patchList, buildShell
- kind: rationale
- logged: v0.2.0 · 2026-10-11
The object list is patched by id: rows are added or removed only when objects are, and names, counts and `aria-current` are written through `setText` and `setAttr`. Meshes are fetched only when an object's revision differs from the uploaded one. The Scene list sits last in the panel: a throttled run with it above the stats measured a layout shift of 0.0012 as the list grew; at the bottom it pushes nothing and the shift is 0.

## src/engine/stroke-client.ts

### src/engine/stroke-client.ts · StrokeClient
- refers-to: src/engine/stroke-client.ts :: StrokeClient.begin, move, end, missed
- kind: rationale
- logged: v0.3.0 · 2026-10-11
One WebSocket, opened early (`warm`) so the first stroke does not wait on a handshake. The brush radius is set in screen pixels and converted to world units at the stroke's start from the camera distance. If the server says the first sample missed the mesh, the viewport ends the stroke and turns the drag into an orbit, the way Nomad treats a drag that starts off the model. Vite proxies the socket (`ws: true`).

## src/viewport/gl-renderer.ts

### src/viewport/gl-renderer.ts · patch
- refers-to: src/viewport/gl-renderer.ts :: GlRenderer.patch, setRevision
- kind: rationale
- logged: v0.3.0 · 2026-10-11
The renderer keeps a CPU copy of each mesh, writes changed vertices into it and uploads the span from the lowest to the highest changed index with `bufferSubData`; the mask is a third vertex attribute that darkens masked areas in the shader. At stroke end the mesh's revision is set from the server so the next scene refresh does not refetch the whole mesh.

### src/viewport/gl-renderer.ts · import line
- refers-to: src/viewport/gl-renderer.ts :: first line
- kind: warning-context
- logged: v0.3.0 · 2026-10-11
In 0.3.0 the import line (adding `VertexUpdate`) was changed with `sed -i` instead of a hand edit, against SEDO. The result is the intended one-line change; it is declared here and in the turn ledger.

## src/app.ts

### src/app.ts · brush panel
- refers-to: src/app.ts :: BRUSHES, ACTIONS, SceneView constructor, brushState
- kind: explainer
- logged: v0.3.0 · 2026-10-11
Brushes: Orbit (no brush), Draw, Clay, Inflate, Pinch, Crease, Flatten, Smooth, Grab, Mask. Radius (pixels), strength, invert (or hold Ctrl), mirror X and radial count. Edit buttons dispatch undo, redo and the mask commands; Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y on the canvas. Alt or Shift held at pointer-down navigates instead of sculpting.
