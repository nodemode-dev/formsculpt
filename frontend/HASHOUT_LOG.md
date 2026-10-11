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

## src/ui/agent-link.ts

### src/ui/agent-link.ts · AgentLink
- refers-to: src/ui/agent-link.ts :: AgentLink.tick, show, answer
- kind: rationale
- logged: v0.3.0 · 2026-10-11
Polls `/api/mcp/window` about once a second, which also tells the service a window is live. When the engine's epoch moved and no stroke of the window's own is in flight, the scene is refreshed (only meshes whose revision changed are fetched). A pending consent request shows a dialog in `#overlay-root` at `--z-modal`, built once at start and toggled with `hidden`; an answered request is remembered by its time so a poll already in flight cannot reopen it.

### src/styles/shell.css · .consent[hidden]
- refers-to: src/styles/shell.css :: .consent
- kind: warning-context
- logged: v0.3.0 · 2026-10-11
`.consent` sets `display: grid`, which beats the browser's own `[hidden] { display: none }`; without `.consent[hidden] { display: none; }` the dialog never hides. Found by the consent check: the dialog stayed on screen after Allow.

### src/app.ts · topology controls and refetch
- refers-to: src/app.ts :: TOPOLOGY, SceneView.refetch, brushState (dyntopo)
- kind: rationale
- logged: v0.4.0 · 2026-10-11
The Topology section runs subdivide, remesh_voxel (with the resolution field), decimate at 50 percent and fill_holes; the Brush section gains Dyntopo and Detail size (a fraction of the radius). During a dyntopo stroke each topology change schedules a full mesh fetch; at most one fetch per object is in flight and one more is queued, so a fast stroke does not pile up requests.

### src/app.ts · multiresolution controls, constant detail
- refers-to: src/app.ts :: MULTIRES, buildShell (statLevel, constant), SceneView.apply, brushState, constantSize, key
- kind: rationale
- logged: v0.4.1 · 2026-10-11
The Multiresolution section reuses the topology button wiring (action plus JSON args) for Divide, Lower, Higher, Delete lower and Delete higher; the Active mesh list gains Level ("2 of 2", or "none"), set in place, and the Dyntopo checkbox is disabled while the object has levels (the engine would ignore it). Constant detail sends `dyntopo_size` as the detail fraction times 5 percent of the object's diagonal, so the slider means the same on any object size. Verified under 4x CPU throttling: two divides, a stroke, Shift+D, D and both deletes with layout shift 0 and no console errors.

### src/app.ts, src/engine/stroke-client.ts · detail mode
- refers-to: src/app.ts :: buildShell (detailMode), brushState (dyntopoScreenPx); src/engine/stroke-client.ts :: StrokeClient.begin
- kind: rationale
- logged: v0.4.1 · 2026-10-11
Detail mode replaces the Constant detail checkbox: Brush sends the fraction of the radius, Constant a world size from the object's diagonal, Screen a pixel size (detail times 0.2 px) that the stroke client turns into world units with the same world-per-pixel it uses for the radius, fixed for the stroke. Rebuild lower joins the Multiresolution buttons. Verified under 4x throttling: a screen-detail stroke took a 1,280-triangle sphere to 9,316, and Rebuild lower then Shift+D on a subdivided sphere showed level 0 of 1 at 1,280 triangles, layout shift 0, no console errors.

### src/app.ts · Object and Layers sections
- refers-to: src/app.ts :: OBJECT_ACTIONS, LAYER_ACTIONS, SceneView.objectAction, layerAction, patchLayers, previous
- kind: rationale
- logged: v0.5.0 · 2026-10-11
Booleans act on the active object and the one active before it (or the only other object), so the order is: click the object to combine with, then the one to keep or cut from. Duplicate offsets the copy by 1.1 times the object's width. Bend, twist and taper take the Deform axis and the Angle field (the taper reads it as a percentage). The Layers section has a Record into menu whose options are patched in place, and a strength slider that sends on change, not on every input, and is not overwritten while it has focus. The Dyntopo checkbox is disabled while the object has levels or layers. Verified under 4x throttling: duplicate, union at resolution 96, bend, add layer, two strokes and the slider to 50 percent, layout shift 0, no console errors. There is no on-canvas transform gizmo yet; transform_object covers it for agents.

### src/app.ts, src/engine/service.ts, src/ui/dom.ts · File section
- refers-to: src/app.ts :: FILE_ACTIONS, SceneView.fileAction, download, uploadChosen, key (Mod+S, Mod+O, Mod+E); src/engine/service.ts :: uploadFile, fetchFile, postJson; src/ui/dom.ts :: saveBlob
- kind: rationale
- logged: v0.6.0 · 2026-10-11
One hidden file input serves Open: a .fsc replaces the scene, any mesh format imports. Save and Export fetch the file and save it through a temporary object URL (in ui/, since engine/ never touches the DOM). Print check writes its verdict into the panel note. The buttons sit in two columns so "Print check" fits on one line. Verified under 4x throttling: STL import, print check, save, Ctrl+E export as 3MF, and opening a .fsc with levels and layers, layout shift 0, no console errors.

### src/viewport/gl-renderer.ts, src/app.ts · paint and shading
- refers-to: src/viewport/gl-renderer.ts :: MESH_FS, SHADINGS, paintBuffer, patch, setShading; src/app.ts :: Paint section, paintArgs, BRUSHES; src/engine/service.ts :: parseUpdate, fetchMesh
- kind: rationale
- logged: v0.7.0 · 2026-10-11
Paint arrives as one five-float buffer bound to two attributes; a mesh first painted mid-stroke gets its buffer created in the patch path, filled with the default for the vertices the update does not cover. Matcap presets stay procedural (key, specular strength and sharpness, rim, tint) so there are no image assets to ship; Lit (PBR) uses roughness and metalness per vertex with GGX, Smith and Schlick terms, three lights and a hemisphere ambient, then Reinhard and gamma. Paint material too adds roughness and metalness to the brush; otherwise only colour is painted. Verified under 4x throttling: a red stroke, a metallic stroke in PBR, the bronze matcap and undo, layout shift 0, no console errors.

### src/app.ts · unwrap, bake and maps
- refers-to: src/app.ts :: Paint section (Unwrap UVs, Bake maps, Download maps, bakeSize), SceneView.bake
- kind: rationale
- logged: v0.7.1 · 2026-10-11
The bake reports its size, source and time in the panel note and points at the two ways out (GLB export, which embeds the maps, and Download maps, a zip of the four files from /api/engine/maps). Verified under 4x throttling: the refusal before unwrapping, unwrap, a 512 bake, the maps zip and a GLB download that validates, layout shift 0, no console errors.

### src/app.ts · modeling controls
- refers-to: src/app.ts :: TOPOLOGY (quad_remesh, quadTarget), OBJECT_ACTIONS (project), ACTIONS (extract_mask, extrude_mask, inset_mask), SceneView.objectAction
- kind: rationale
- logged: v0.8.0 · 2026-10-11
Project from follows the boolean convention: the object selected before the active one is the source. The mask operations sit with the other mask buttons under Edit. Duplicate now copies in place, as ZBrush does, because the remesh-divide-project workflow needs the copy on top of the source (the first browser run, with the old one-width offset, projected almost nothing). Verified under 4x throttling: duplicate, quad remesh to 1,500, two divides, Project from, a mask stroke, Extrude mask and Extract mask, layout shift 0, no console errors.

### src/app.ts, src/ui/presets.ts · Build section and the Tube brush
- refers-to: src/app.ts :: Build section (skeleton, lathe, tubeRadius, buildButtons), BRUSHES (tube), SceneView.build, tubeBegin, tubeMove, tubeEnd, onPlane, stage-overlay; src/ui/presets.ts :: SKELETONS, LATHES; src/styles/shell.css :: .stage-overlay, .add-grid.after-fields; src/viewport/math.ts :: distance3
- kind: rationale
- logged: v0.8.1 · 2026-10-11
Presets live in their own module as data (sphere lists and profiles), so the same shapes are easy to hand to an agent. Build skeleton uses the Voxel resolution field, clamped to the command's 32 to 256. Tube is a pseudo-brush: it hooks the viewport's stroke handler, puts each drag ray onto the plane through the orbit target facing the camera, keeps a point every 8 pixels' worth of world distance, and calls curve_tube on release; a click without a drag says what to do instead. The preview is an SVG polyline in an absolutely positioned overlay inside the stage with pointer events off, so drawing it never moves layout. Verified under 4x throttling: all four skeletons and all four lathe profiles from the panel (each watertight), a tube drawn beside the biped, the click-only message, undo, and a Draw stroke on the skeleton; layout shift 0, no console errors.

### src/host.ts, src/ui/desktop.ts, src/app.ts · embedding, Send to NodeMode, desktop saves
- refers-to: src/host.ts :: HostLink, dataUrl; src/ui/desktop.ts :: inDesktop, saveNative; src/app.ts :: FILE_ACTIONS (nodemode), NODEMODE_KEY, SceneView.sendToNodeMode, SceneView.download, host; src/engine/service.ts :: postBody; src/styles/shell.css :: .field[hidden], .field input[type="url"], .add-grid .btn.wide
- kind: rationale
- logged: v0.9.0 · 2026-10-11
host.ts is formvis's host link with the default plugin id changed: in an iframe it answers the portal's init with ready and hands files over as plugin:<id>:assign. Embedded, Send to NodeMode becomes Send to canvas and hands over the GLB and a 768 pixel transparent render as data URLs; the address row is hidden in the constructor, before the first paint (it needed `.field[hidden]`, as `.field` sets display). Standalone, it posts the address and the active object's name to /api/nodemode/send and shows NodeMode's answer; the address is remembered in localStorage inside try/catch. In the desktop window (where Tauri injects its IPC) Save, Export and Download maps send the fetched bytes to the shell's save_file command as the raw IPC body, with the name percent-encoded in a header; the browser keeps blob downloads. postBody turns a FastAPI detail into the note's message. Verified under 4x throttling: the five standalone cases against a stand-in NodeMode, the embedded handoff from a host page (layout shift 0 in the frame), the remembered address after reload; layout shift 0, and the only console lines are the 502 and 422 responses of the deliberate failure cases.

### src/app.ts, src/viewport/viewport.ts, src/viewport/camera.ts · mask tools, view controls, history
- refers-to: src/app.ts :: BRUSHES (relax, planar, trim, snake_hook, noise, mask_box, mask_lasso, mask_topo), SHAPE_TOOLS, ACTIONS and ACTION_CALLS, OBJECT_ACTIONS (align, distribute), LAYER_ACTIONS (surface_noise), View section (fov, viewButtons), History section (historyList, patchHistory), SceneView.shapeEnd, topoMask, pivot, apply (hidden objects); src/viewport/viewport.ts :: rayAtClient, pivotAt, setFov; src/viewport/camera.ts :: OrbitCamera.pivotAt; src/styles/shell.css :: .history
- kind: rationale
- logged: v0.10.0 · 2026-10-11
Box and lasso masks are window tools on the stroke hook: the outline is drawn in the existing stage overlay from the canvas's pointer events, and on release the points (as fractions of the canvas) go to mask_shape with the camera's view-projection matrix, so the engine masks exactly what was drawn; Ctrl subtracts. The topology mask picks the clicked point (the new pick command) and masks outward by twice the brush radius in world units. Double-click pivots only with Orbit or with Alt held, as a double-click with a sculpting brush would also lay down dabs; pivotAt keeps the eye where it is and re-derives yaw, pitch and distance, so the view does not jump. Hidden objects are dropped from the renderer and refetched when shown. The History list has a fixed height and scrolls, rows are reused, and it sits above the Scene list: below it, the first run measured a layout shift of 0.0009 when the scene lost an object. Verified under 4x throttling: each new brush, both shape masks (491 masked, 353 after a Ctrl box), the topology mask and its refusal off the object, grow, shrink, cavity, peaks, mask colour's refusal without paint, field of view, the pivot, align, hide and show all, a history jump to Start and back, and the Noise layer; layout shift 0, no console errors.

### src/viewport/gl-renderer.ts, camera.ts, src/app.ts · face group display, hidden faces, click tools, trackball, reference
- refers-to: src/viewport/gl-renderer.ts :: GpuMesh.drawn, drawnTris, expanded, groupColor, setShowGroups, expand, fillExpanded, refreshExpanded, dropExpanded; src/engine/service.ts :: fetchMesh (flags 4 and 8), MeshBuffers.groups, hidden; src/viewport/camera.ts :: OrbitCamera.trackball, setTrackball, axes, rotate; src/app.ts :: CLICK_TOOLS, clickTool, View section (showGroups, trackball, reference, turntable), Build section (insert, array), Object section (folder)
- kind: rationale
- logged: v0.11.0 · 2026-10-11
An indexed mesh cannot give each triangle its own colour (WebGL2 has no primitive id), so with Face groups on each mesh with groups gets a de-indexed copy, built lazily, with a colour per corner from the group (pastel hues spaced by the golden ratio, the same formula as get_view's groups shading); a stroke patch refills its positions, normals and mask from the indexed copy, so sculpting shows live in group colours. Hidden triangles are left out of the index buffer at upload. Click tools (Topo mask, Mask group, Hide group with Ctrl to isolate, Insert) pick the point and send one command. Trackball keeps a direction and a free up vector turned about the view's own axes (Rodrigues), so it rolls and passes over the poles; switching back re-derives yaw and pitch, and a snapped view drops the roll. The reference image is an absolutely placed image over the canvas with pointer events off, see-through by its opacity slider; nothing reflows when it appears. Verified under 4x throttling: auto groups on a cube (6) and their display, a stroke in group view, hide group, invert, show faces, mask group, hide masked, group from a box mask, insert a cone, radial and linear arrays and one undo for an array, trackball over the pole, the reference at 60 percent, the turntable download (48 frames), folders; layout shift 0, no console errors.
