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

### src/app.ts, src/ui/hotkeys.ts, src/viewport/gl-renderer.ts, camera.ts, src/engine/stroke-client.ts · alphas, drag stamp, smudge, image matcaps, custom keys
- refers-to: src/ui/hotkeys.ts :: Hotkeys, comboOf, commandActions, WINDOW_ACTIONS, WINDOW_KEYS, KEY_ARGS; src/app.ts :: BRUSHES (smudge, stamp), ALPHAS, Brush section (alpha, alphaAngle, strokeMode, alphaLoad), View section (shadeActive, matcap), Keys section (keyAction, keyCombo, keyButtons), SceneView.stampEnd, applyShading, loadAlpha, loadMatcap, loadCatalogue, showKeys, keyButton, runAction, key, brushState.alpha; src/viewport/gl-renderer.ts :: uImage, uMatcap, blank, setShading(name, id), shadingOf, addMatcap; src/viewport/camera.ts :: screenUp; src/engine/stroke-client.ts :: BrushState.alpha
- kind: rationale
- logged: v0.12.0 · 2026-10-11
Every key the canvas answers now goes through one map of shortcut to action: the defaults are the keys the registry declares (the catalogue's keys field, which nothing read before) plus the window's own views and file actions, and the person's changes are stored as overrides in localStorage (wrapped in try/catch) and laid over them, so Reset brings the registry's keys back and a cleared key stays cleared. A command is bindable when it changes something and needs nothing beyond the active object's id. The drag stamp is a shape tool: the overlay draws a circle through the pointer, and on release one point goes to sculpt_stroke with the drag's length as radius and its direction as the alpha's turn (straight up keeps it upright). Image matcaps are textures looked up by the view-space normal and tinted by paint; a 1 by 1 blank stays bound when none is in use, so the sampler is never incomplete. Shading can be set for the active object only; the choice lives in the window. Verified under 4x throttling: star and roll stripes strokes, a loaded triangle alpha and its drag stamp, paint then smudge (Smudge Stroke in history), an image matcap with a bronze cube beside it, Shift+S bound to Smudge, Ctrl+I from the registry masking 1,538 vertices, a cleared Ctrl+Z doing nothing, the binding kept through a reload, and Reset; layout shift 0, no console errors. Process slip declared: the renderer changes were applied by a Python string replace instead of hand edits.

### src/app.ts, src/engine/stroke-client.ts, src/styles/shell.css · falloff editor, pressure, spray, drag dot, curve strokes
- refers-to: src/app.ts :: FALLOFFS, FALLOFF_POINTS, BRUSHES (dragdot, curve), Brush section (falloff, falloffCurve, pressureGamma, pressureStrength, pressureSize, curveButtons), overlay (curveLine, curveHandles), SceneView.lastBrush, curve, brushArgs, curveScreen, spline, drawCurve, curveEnd, applyCurve, clearCurve, drawFalloff, falloffWeights, stampEnd (now the last brush); src/engine/stroke-client.ts :: BrushState.stroke, falloffCurve, pressure; src/styles/shell.css :: .falloff-editor, .stage-overlay path
- kind: rationale
- logged: v0.13.0 · 2026-10-11
The falloff editor is a fixed-height SVG of five weights dragged up and down; it always shows, drawing the chosen preset exactly, and a drag switches the Falloff field to Drawn curve, so the panel never changes height. A curve stroke is drawn on screen, resampled to a handle about every 40 pixels and picked onto the active object, so it stays on the surface as the view turns (redrawn on every frame through Viewport.onFrame); pressing near a handle moves and re-picks it; Apply curve sends one sculpt_stroke along a Catmull-Rom spline through the handles with the last sculpting brush and every Brush setting (brushArgs, shared with the stamps), and the curve stays for another pass. Drag dot is a dab of the brush's size that follows the pointer and lands on release. Verified under 4x throttling: a drawn ring falloff stroke, a spray stroke, a pressure-size stroke at curve 2, a clay drag dot (one step), an 8-handle curve drawn, a handle dragged, Apply curve with crease (one step) and again after orbiting, Clear curve, and the refusal with no curve; layout shift 0, no console errors. The first run's drag on the falloff editor missed because the editor was below the panel's fold; the test now scrolls it into view.

### src/app.ts, src/engine/service.ts, src/styles/shell.css · paint layer controls
- refers-to: src/app.ts :: Paint section (paintLayer, paintBlend, paintOpacity, paintVisible, paintLayerButtons), SceneView.patchPaintLayers, paintLayerAction; src/engine/service.ts :: ObjectSummary.paint_layers, active_paint_layer; src/styles/shell.css :: .btn:disabled
- kind: rationale
- logged: v0.14.0 · 2026-10-11
The Paint section gained a Paint into select (Base and the layers, listed top first as a stack reads, valued by the engine's bottom-first index), Blend, Opacity and Visible for the chosen layer, and New paint layer, Delete paint layer, Merge down, Flatten paint, Move up and Move down; controls that need a layer are disabled until one exists, with a disabled button style added. The labels say paint because the Layers section already has Add layer and Delete for sculpt layers (the first test run hit exactly that ambiguity). Options are reused in place, so the panel keeps its height. Verified under 4x throttling: fill, a new layer, a paint stroke into it, multiply at 50 percent, hide and show, a second layer listed on top, move down, base selected (layer controls off), merge down and its undo, delete and flatten; layout shift 0, no console errors.

### src/viewport/environment.ts, post.ts, gl-renderer.ts, src/app.ts · HDRI lighting and post-processing
- refers-to: src/viewport/environment.ts :: ENV_PRESETS, presetEnvironment, parseHdr, imageEnvironment, mipChain; src/viewport/post.ts :: PostFx (begin, finish, SSAO_FS, BRIGHT_FS, BLUR_FS, COMPOSITE_FS), PostSettings, NO_POST; src/viewport/gl-renderer.ts :: MESH_FS (envAt, tonemap, uEnvOn, uEnvRot, uEnvMaxLod, uExposure, uLightScale, uTone), BG_VS, BG_FS, setEnvironment, setPost, lighting, Lighting, TONES; src/app.ts :: Lighting section (envSelect, envRotate, exposure, fixedLights, tone, envBackground, envBlur, envLoad, postSsao, postBloom, postDof, postVignette, postGrain), SceneView.chooseEnvironment, applyLighting, loadEnvironment
- kind: rationale
- logged: v0.15.0 · 2026-10-11
Environments are equirectangular float images: four made in code (a photo studio with three soft lights, sunset, overcast, night), Radiance .hdr files (flat and run-length scanlines, any orientation) or ordinary images taken as sRGB. The mip chain is built on the CPU by box filtering and uploaded as RGB16F level by level, because WebGL2 cannot generate mips for half-float textures without a colour-buffer extension; TEXTURE_MAX_LEVEL bounds the chain. Lit shading adds diffuse from the next-to-smallest mip along the world normal and reflections from a mip chosen by roughness along the reflected ray, weighted by Karis's analytic split-sum fit (no lookup texture), with the three fixed lights kept at an adjustable share, then exposure and a choice of Reinhard, ACES filmic or no tone mapping. Choosing an environment switches the view to lit shading, since matcaps ignore it. The background is a fullscreen triangle at the far plane that looks up each pixel's view ray, blurred by mip level; it is drawn with the depth test off, because at the far plane it ties the cleared depth and failed the test in the first run. Post effects draw the scene into a 4x multisampled target, resolve colour and depth separately, then run SSAO at half size from depth alone (16 samples, a 4 by 4 interleaved kernel turn that the composite's 4 by 4 blur cancels; per-pixel noise left the first version mottled), a quarter-size bright pass blurred twice for bloom, and a composite with depth of field focused at the orbit target, the occlusion, bloom, a vignette and grain. With every effect off the renderer draws to the canvas as before. Verified under 4x throttling: studio lighting on a metal sphere and a torus, sunset with the background shown and turned, a loaded .hdr, each effect alone changing the picture (occlusion 0.5, bloom 3.95, depth of field 1.05, vignette 3.59, grain 2.79 per channel on average) and all together, a stroke with effects on, and the environment off again; layout shift 0, no console errors or GL warnings.

### src/ui/gizmo.ts, src/app.ts, src/viewport/gl-renderer.ts, viewport.ts, src/styles · transform gizmo, lattice tool, touch gestures, pen tilt
- refers-to: src/ui/gizmo.ts :: Gizmo (paths, ring, hit, update, matrix), GizmoDrag, GizmoHandle; src/app.ts :: BRUSHES (gizmo, lattice), LATTICE_DIV, SceneView.projector, activeObject, gizmoFrame, latticePoints, handleTool, gizmoEnd, latticeEnd, drawTools, shape.miss; src/viewport/gl-renderer.ts :: MESH_VS uModel, setPreview, previews, IDENTITY; src/viewport/viewport.ts :: StrokeHandler (tilt), tilt, touches, gesture, gestureFrame, pendingStart, beginStroke; src/engine/stroke-client.ts :: begin and move (tilt); src/styles/tokens.css :: --axis-x, --axis-y, --axis-z; src/styles/shell.css :: gizmo and lattice paths
- kind: rationale
- logged: v0.16.0 · 2026-10-11
The Transform tool draws three arrows, three rings and three squares in the axis colours plus a centre square, at a constant size on screen around the bounds centre that transform_object turns and scales about. Where handles cross on screen the nearest wins (the first version took the first ring in order and turned about x when a y-ring point was pressed). A drag previews on the GPU through a per-object model matrix (normals by its inverse transpose) built exactly as the engine applies the change, and the release sends one transform_object; the preview stays until the new geometry is uploaded and upload clears it in the same frame, so the object never jumps back: the preview and the result differed by 0.0 per channel. A press that misses every handle turns the view instead, through the stroke hook's missed signal. The Lattice tool shows the 3 by 3 by 3 cage lattice_deform uses; dragging a point moves it in the plane facing the camera, the cage follows live, and the release sends one lattice_deform. Two fingers pan and pinch-zoom, and a second finger arriving ends nothing because a finger's stroke now waits until it has moved 6 pixels (the first version left a one-dab stroke behind every pinch); a tap still lays one dab. A pen's lean turns the brush alpha. Verified under 4x throttling: move along x, scale along y, turn about y (y size kept), uniform scale, a missed press orbiting with no new step, undo, a lattice drag raising a sphere, a two-finger pinch and pan with no stroke, a one-finger stroke, upright and tilted pen stamps; layout shift 0, no console errors.

### src/viewport/gl-renderer.ts, src/app.ts · movable key light, shadows, shadow catcher, noise editor, primitive fields
- refers-to: src/viewport/gl-renderer.ts :: MESH_VS vWorld, MESH_FS (uKeyDir, uKeyColor, keyShadow, uShadowVP, uShadowMap), renderShadow, drawGround, SHADOW_VS, SHADOW_FS, GROUND_VS, GROUND_FS, GpuMesh.lo/hi, Lighting (keyAzimuth, keyElevation, keyIntensity, keyColor, keyFollowsView, shadows, ground); src/app.ts :: hexToLinear, Lighting section (key light fields, Shadows, Ground shadow), Add section (primSize, primDetail, primHeight, primApply), Layers section (noise editor fields), SceneView.patchPrimitive, applyPrimitive, noiseArgs; src/engine/service.ts :: ObjectSummary.primitive
- kind: rationale
- logged: v0.17.0 · 2026-10-11
The key light is no longer fixed: azimuth, height, strength and colour, either turning with the view (as before, and the default) or fixed in the world, in every shading mode. Shadows come from a 2048 square depth map rendered from the key light with an orthographic camera fitted around every mesh's bounds (taken at upload) and the floor area, sampled with a 3 by 3 filter so edges are soft and shadowed areas keep a quarter of the key light. The ground shadow is a shadow catcher: an invisible floor just under the scene that draws only the shadow falling on it (blended, fading at its edge), so models read as standing on something without adding a mesh. The noise editor drives surface_noise with sizes as percentages of the object's size; the alpha pattern uses the Brush section's alpha. The primitive fields show a new primitive's settings and stay disabled for anything else. Verified under 4x throttling: a cylinder remade at height 0.6 and 24 segments, a moved sphere no longer editable, turning the key light, shadows from a sphere and a torus onto a plane with the light following the view and then fixed in the world at 120 degrees, the ground shadow, studio lighting with shadows, cells and stretched alpha noise layers; layout shift 0, no console errors or GL warnings.

### src/viewport/gl-renderer.ts, src/app.ts · added lights
- refers-to: src/viewport/gl-renderer.ts :: MESH_FS (uExtraCount, uExtraDir, uExtraColor), Lighting.extra, LightSpec; src/app.ts :: Lighting section (lightPick, lightAdd, lightRemove), SceneView.lightSpecs, patchLightList, showLight
- kind: rationale
- logged: v0.17.0 · 2026-10-11
Up to three lights can be added beside the key; the Light choice picks which one the direction, height, strength, colour and follows-the-view fields edit (the fields keep their aria labels from the key-only version). A new light starts opposite the key, dimmer and cool. Added lights cast no shadows; in lit (PBR) shading they light in colour through the same GGX term as the key, and in the matcap modes they add to the fill as brightness only, as the key does there. Verified: adding a light, moving it, returning to the key's settings with Remove disabled, removing it; a strong added light from the right brightened the scene in the matcap mode.

### src/app.ts, src/viewport/gl-renderer.ts · projection painting, reference planes, plugins
- refers-to: src/app.ts :: Paint section (project_view, project_tile, projectOpacity, projectInput), SceneView.projectImage, View section (referencePlace), applyReference, Plugins section (pluginPick, pluginButtons), loadPlugins, runPlugin; src/viewport/gl-renderer.ts :: setReferenceImage, setReferencePlane, drawReference, REFERENCE_VS, REFERENCE_FS
- kind: rationale
- logged: v0.18.0 · 2026-10-11
Project image paints the chosen picture onto the active object from the current camera (as it would sit over the canvas), Tile image tiles it triplanar; any image the browser reads is re-encoded as PNG (at most 2048 on its long side) before it goes to the engine. The reference image can now sit on an image plane in the scene instead of over the view: front, side or top, just behind the scene's bounds and sized to them, drawn with blending and without writing depth, so it reads as a board behind the model from that view and the model passes in front of it when the camera turns. A Plugins section lists the service's plugins and runs the chosen one; its last printed line or its error shows in the note. Verified under 4x throttling: a view projection and a 50 percent tiled checker, the reference over the view and on the front and side planes, clearing it, the pedestal plugin adding an object, and its refusal with nothing selected; layout shift 0, no console errors.

### src/app.ts · silhouette tool, mirror and curve arrays, linked copies
- refers-to: src/app.ts :: BRUSHES (silhouette), SHAPE_TOOLS, shape enabled, silhouetteEnd, silhouettes, Build section (array_mirror, array_curve, shadow_box, clear_outlines, arrayLinked), runBuild; src/engine/service.ts :: ObjectSummary.instance_of
- kind: rationale
- logged: v0.19.0 · 2026-10-12
The Silhouette tool draws an outline over the view, works with nothing selected, and keeps it as the front, side or top outline by whichever axis the camera looks along most; each point is taken onto the plane through the orbit target facing that axis, thinned to at most about 256 points. Build shadow box sends the kept outlines (it asks for a second one until there are two) at the Remesh resolution, and Clear outlines forgets them. Mirror copy reflects the active object across x; Array on curve spaces copies along the waiting Curve stroke's spline, and says so when there is none. The Linked copies box makes any array linked, so strokes and commands on the original reach the copies. Verified under 4x throttling: front and side outlines built a 38,668-vertex shadow box, three linked copies went from revision 1 to 2 after a stroke on the original, a mirror copy, and the curve array's message without a curve; layout shift 0, no console errors.

### src/ui/layout.ts, src/app.ts, src/styles/shell.css · interface customization
- refers-to: src/ui/layout.ts :: PanelLayout (apply, fold, setSide, setSize, attachRail, drawRail, clearRail), load, save, short; src/app.ts :: buildShell (panelLeft, uiSize, clearRail, layout); src/styles/shell.css :: h2.fold, .fold-hidden, .panel-small, .panel-large, #app.panel-left, .rail, .rail-btn, .panel-hint
- kind: rationale
- logged: v0.19.0 · 2026-10-12
Every panel heading folds its section away (click, Enter or Space; aria-expanded follows), the panel can move to the left of the viewport with the tool rail to the right, and the panel can be drawn small, normal or large through CSS zoom inside its fixed column, so the viewport never resizes when the size changes. The tool rail, empty until now, is a toolbar the person builds: the context menu on any panel button (right button or the keyboard's menu key) pins it to the rail or takes it off, a rail button carries two letters with the full name as its accessible name and tooltip, presses the panel button it stands for (so it can never drift from what that button does), and mirrors its pressed and disabled state through a MutationObserver; Clear rail empties it. Fold headings are named "<section> section" because the first run named them like the tool buttons (Paint, Build) and broke exact-name lookups in the p14 regression. The choices are kept in this browser under formsculptor.layout (wrapped, so blocked storage only loses them at reload) and are layout only, so they never reach the engine or projects. A fold is the person's own input, so it is not counted as layout shift. Verified: 18 fold headings, folding File hid its two blocks, Enter unfolded it, the panel moved left of the stage with the rail on the right, large size applied, and all three choices came back after a reload; under 4x throttling, four buttons pinned (Sp, Cl, Sm, Un), the rail Sphere added a sphere, the rail Clay pressed the panel's Clay and the pressed state followed a panel click, unpinning by right-click, the rail kept over a reload and moved right with the panel on the left, Clear rail; layout shift 0, no console errors.

### src/app.ts · VDM stamp, lasso cut and trim, 3D text, extruded outlines, boolean blend
- refers-to: src/app.ts :: BRUSHES (vdm, cut, trim_shape), SHAPE_TOOLS, CLICK_TOOLS, clickTool (vdm), cutEnd, Build section (vdmPick, textInput, extrude_outline, vdm_capture, text), Object section (fuseBlend), boolean action
- kind: rationale
- logged: v0.20.0 · 2026-10-12
VDM stamp is a click tool: the picked point and normal, the brush radius in world units at that depth, the alpha angle as its turn and Ctrl to press it in. Capture VDM names maps Map 1, Map 2 and adds them to Stamp map. Lasso cut and Lasso trim draw like the lasso mask; on release the outline is taken onto the plane through the orbit centre facing the view and cut straight through along the view at the Remesh resolution (in a perspective view the cut is the outline as seen at that depth). Build text makes the Text field's words; Extrude outline turns the front outline drawn with the Silhouette tool into a flat solid. Boolean blend sets the fillet for Union, Subtract and Intersect. Verified under 4x throttling: text built, a horn stamped on a divided sphere (front from 1.0 to 1.419), a lasso cut through a cube (a ray through the middle hits nothing afterwards), a blended union of a sphere and a rod, and the refusals for capturing a non-plane and extruding with no outline; layout shift 0, no console errors.

### src/app.ts · insert brush, group brush, grouping by paint and UV, animation keys
- refers-to: src/app.ts :: BRUSHES (insert_brush, group_brush), SHAPE_TOOLS, strokeTool, ACTION_CALLS (group_by_paint, group_by_uv), ACTIONS, View section (anim_key, anim_record, anim_clear), animKeys, animationAction
- kind: rationale
- logged: v0.21.0 · 2026-10-12
Insert brush and Group brush draw a stroke like the lasso; on release the stroke is sampled every 6 pixels (at most 256 points), each sample picked on the surface, and the whole stroke goes as one insert_along (Insert shape and Insert size, or the previous object) or one group_paint (the brush radius at that depth). Add key stores the camera's turn and height and the active object's layer strengths every 1.5 s, unwinding the turn so the camera goes the short way round; Record animation downloads the render from the service's animation route, and History movie downloads the undo history as a movie turning 90 degrees across it. Verified under 4x throttling: cones strung across a sphere, a group painted across it, six UV groups on a cube, the paint-grouping refusal on an unpainted cube, a two-key animation downloaded as a 36-frame 384 pixel APNG, and the refusal with no keys; layout shift 0, no console errors.

### e2e/smoke.py · the window smoke test
- refers-to: e2e/smoke.py :: main, check, call, objects
- kind: rationale
- logged: v0.22.0 · 2026-10-12
The per-release scratch checks condensed into one in-tree run: in Chromium on SwiftShader under 4x CPU throttling, with a layout-shift observer from the first paint, it checks that the service answers, a sphere is added, a stroke changes the mesh and undo takes it back, a lasso cut goes through (a ray passes), text builds, a section folds, and the history movie downloads as a PNG, then that layout shift is 0 and the console has no errors. It prints one line per check and exits 1 on any failure. It needs a booted window (./formsculptor test e2e), so CI does not run it yet. All ten checks passed on its first run.

### src/app.ts, src/engine/service.ts · plugin fields and the Plugin tool, bevel, the bridge, editable tubes and lathes
- refers-to: src/app.ts :: PluginInfo, Plugins section (pluginParams), drawPluginParams, pluginArgs, loadPlugins, runPlugin (click plugins arm the Plugin tool), clickTool (plugin), BRUSHES (plugin), OBJECT_ACTIONS (bevel, bridge), bridgeOut, bridgeSync, bridgeTimer, tubeRadius and lathe listeners; src/engine/service.ts :: ObjectSummary.editable_build
- kind: rationale
- logged: v0.23.0 · 2026-10-12
The Plugins section draws the chosen plugin's fields and sends their values as args. Run plugin on a click plugin presses the Plugin tool instead and says to click a surface; the click sends point, normal and object id with the fields. Bevel edges rounds both convex and concave edges with the Angle or taper % field as the radius in percent of the object's size (1 to 25, 5 by default). Bridge to app writes the object as OBJ to the bridge folder, shows the path, and syncs every 3 seconds while any bridge is open, refreshing when something comes back. Changing Tube radius or Lathe profile remakes the active tube or lathe while it is still editable. Verified under 4x throttling: rock's Size and Seed fields with a run at size 0.5, the rivets fields and a six-rivet ring from a click, a cube bevelled, a torus saved over a bridged cube's file coming back into the cube, and a tube going from 0.1 to 0.4 deep when the radius field changed; layout shift 0, no console errors.

### src/viewport/gl-renderer.ts, src/app.ts · subsurface and transmission in lit shading
- refers-to: src/viewport/gl-renderer.ts :: MESH_FS (uDepthOn, uShadowRange, uSss, uSssColor, uSssRadius, uTrans, uIor, thickness), ObjectMaterial, setMaterial, materials, shadowRange, render (depth map drawn for subsurface); src/app.ts :: Paint section (subsurface, transmission), refresh (materials to the renderer); src/engine/service.ts :: ObjectSummary.material
- kind: rationale
- logged: v0.24.0 · 2026-10-12
Subsurface wraps the key light past the terminator tinted by the subsurface colour and lets it through parts thinner than the radius when lit from behind. The thickness is real: the key light's orthographic depth map (drawn whenever an object has subsurface, even with shadows off) is linear over three scene radii, so the depth difference times that range is the material between a point and the light. The through-light fades in on surfaces turned away from the light; the first version switched it on at the terminator and left a hard seam. Transmission refracts the environment (or the sky gradient without one) through the surface at the index of refraction, tinted by its colour, with Schlick Fresnel reflection on top; it does not see other objects behind it. The Paint section's Subsurface and Transmission sliders set the active object's material and switch to lit shading. Verified: a back-lit sphere and a thin plate with subsurface 1 tinted and glowing (mean change 6.2, 2.2, 1.2 per channel against off), the same as glass in the first environment preset, set, undone and saved.

### src/ui/spacemouse.ts, src/app.ts · 3D mouse
- refers-to: src/ui/spacemouse.ts :: SpaceMouse (connect, feed, tick), readReport, motionOf, SixAxis, ViewMotion; src/app.ts :: View section (spacemouse), connectSpaceMouse
- kind: rationale
- logged: v0.24.0 · 2026-10-12
3Dconnexion devices over WebHID (vendors 0x256F and the older Logitech 0x046D): report 1 is translation, or all six axes on newer devices (12 bytes), report 2 rotation, each a signed 16-bit value about plus or minus 350. Each frame while the cap is off centre (a dead zone of 12 keeps a resting cap still), twist turns the view, tilt raises or lowers it, push pans and the depth axis zooms, scaled by the frame time. Verified with a stand-in WebHID device in Chromium that emitted real input reports: the 3Dconnexion filters were asked for, a twist and a depth push each changed the view, and a small report inside the dead zone did not; no physical device was available, so it has not been tried on hardware. WebHID exists in Chrome and Edge, not in Firefox, Safari or the Linux desktop shell's WebKitGTK, and the button says so.

### src/app.ts · an open issue: the 0.12.0 window check under software GPU
- refers-to: src/app.ts :: refresh (material redraw only on change); scratch p12ui
- kind: bug
- logged: v0.24.0 · 2026-10-12
The 0.12.0 window check (two alpha strokes on a 163,842-vertex sphere at 4x CPU throttling, then a canvas screenshot) now times out every run, where before it timed out only under load. Measured: at that step the page's main thread is idle (a CPU profile over 6 s was almost all idle, no draw calls in 4 s) yet only one animation frame arrives in 4 s, so the time goes in Chromium's software GPU (SwiftShader here) working through the frames the strokes queued; at 0.23.0 the same screenshot took 29.6 s against a 30 s limit inside the screenshot call, and this release's larger fragment shader tipped it over. It is the software renderer in this sandbox rather than the window, but it is logged open until it is measured on a hardware GPU and the per-frame cost of the mesh shader is reduced. A first fix in this release stopped the scene refresh redrawing when no material changed. The in-tree smoke test's undo check assumed new_scene empties the history; new_scene is an undo step itself, so on a service with earlier work the check failed; it now compares against the step count before the stroke.

### src/app.ts, src/engine/service.ts · smooth preview and the unwrap report
- refers-to: src/app.ts :: Shell.smoothPick, View section (Smooth preview), smoothPreview, setSmoothPreview, refetch and apply (smooth), strokes.onUpdate and onEnd (no patching under the preview), unwrapUvs; src/engine/service.ts :: fetchMesh (smooth)
- kind: rationale
- logged: v0.25.0 · 2026-10-12
The View section's Smooth preview (Off, 1, 2 or 3 levels) refetches every visible object as the engine's smoothed copy and says the sculpted mesh is unchanged. A live stroke's updates name the sculpted mesh's vertices, which are not the drawn ones under the preview, so they are not patched in; the stroke shows when it ends, because the renderer's revision is left behind and the refresh fetches the smoothed result (one fetch; the first version also refetched directly and fetched twice). Unwrap UVs now says how many charts it made and the mean and worst angle distortion. Verified: a cube at 2 levels fetched mesh/2?smooth=2 with engine triangles and revision unchanged, a Draw stroke under the preview refetched once and showed, Off fetched the plain mesh, and a sphere unwrapped into 6 charts at 1.01; layout shift 0, no console errors.

### src/app.ts, src/styles/shell.css · the UV section
- refers-to: src/app.ts :: UV section (uvSeams, uvView, uvImage, uvEmpty, uvButtons), uvIsland, uvShown, drawUv, uvAt, wireUvView, uvRun, unwrapUvs (seams), refresh (drawUv); src/styles/shell.css :: .uv-view; src/engine/service.ts :: ObjectSummary.uvs
- kind: rationale
- logged: v0.26.0 · 2026-10-12
A UV section under Paint: Seams (by facing, or along face groups), the active object's layout as a square picture from get_uv_layout, and Unwrap, Pack, Turn 90, Scale up, Scale down and Redraw. A click on the picture chooses the island under it (drawn brighter, kept by number); a drag of more than 3 pixels moves the island it started on by the drag in UV units. The picture is redrawn when the active object, its revision or the chosen island changes. The square is reserved whether or not there is a layout, so nothing moves; an image and a "No UVs yet" line take turns in it. The first run showed both at once: the hidden attribute lost to the CSS display rules, so .uv-view [hidden] hides them, and the image starts hidden. Verified under the window: groups refused without groups, a cube grouped by angle unwrapped into 6, a sphere's island chosen by a click, scaled from width 0.322 to 0.402, dragged 40 pixels for a 0.162 move against 0.162 expected, packed to 56 percent and undone; layout shift 0, no console errors.

### src/engine/wasm.ts, src/engine/service.ts, src/engine/stroke-client.ts, src/app.ts, e2e/wasm_smoke.mjs · the window on the browser engine
- refers-to: src/engine/wasm.ts :: WasmEngine (load, with, dispatch, mesh, strokeBegin, strokeSample, strokeEnd, save, open, importFile, exportFile, maps, catalogue, version); src/engine/service.ts :: useBrowserEngine, browserEngine, NEEDS_SERVICE, localFile, fetchVersion, dispatch, uploadFile, fetchFile, postJson, postBody, fetchMesh, parseMesh, fetchCatalogue; src/engine/stroke-client.ts :: sendLocal, send, warm; src/app.ts :: chooseEngine, start, boot, poll, loadCatalogue, loadPlugins; e2e/wasm_smoke.mjs
- kind: rationale
- logged: v0.27.0 · 2026-10-12
Before anything asks the engine, the window picks one: the engine service when /api/version answers, otherwise the browser engine at /engine/fsc_engine.wasm (?engine=wasm or ?engine=service forces either). service.ts stays the one place engine calls go: with the browser engine set, dispatch, meshes, uploads (projects open, other files import), the project, export, maps and view downloads run in the page, and routes that only the service has (autosave recovery, NodeMode, the bridge, plugins, turntable and animation movies) answer with a message saying so. Live strokes call the module directly instead of the socket, with the same callbacks. The agent link and the five-second service poll are off with the browser engine, since agents reach the engine through the service; the status reads "browser engine" and its version. Module output is copied out of wasm memory on every call, since the next call replaces it and memory may grow. A trap marks the engine stopped and later calls say to reload, with the panic message when there is one. Verified at ?engine=wasm with the service running: a sphere added, a clay stroke as one step, the smooth preview, a UV unwrap with its picture, undo, a 133,547-byte project saved and a GLB exported, with no request to /api at all; layout shift 0, no console errors.

### src/app.ts, src/engine/service.ts, src/engine/wasm.ts, src/styles/shell.css · Live boolean, and a fixed note box
- refers-to: src/app.ts :: Object section (liveBoolean), live, liveKey, setLive, updateLive, apply (objects replaced by the preview, renderer id 0), objectAction (a boolean button keeps the live pair); src/engine/service.ts :: fetchBooleanPreview; src/engine/wasm.ts :: booleanPreview; src/styles/shell.css :: .panel-note
- kind: rationale
- logged: v0.28.0 · 2026-10-12
Live boolean (Off, Union, Difference, Intersection) in the Object section takes the active object as a and the one chosen before it as b, stops drawing both and draws their exact result instead (renderer id 0), recomputed whenever either object's revision changes, so moving or sculpting either updates it; pressing Union, Subtract or Intersect keeps the result with that operation, and Off draws the two again. If an object goes or the input cannot be cut exactly, it stops and says why. The result follows on each change rather than during a gizmo drag. Measuring it showed a layout shift of 0.0027: the note above Active mesh had only a minimum height, so a note that wrapped to two lines pushed every section below it down 34 pixels (earlier window checks did not catch it because their notes changed right after a click). The note is now a fixed three-line box that scrolls a longer message inside it. Verified with the service and on the browser engine: a sphere minus an offset cube drawn live, switched to union after the cube moved, kept with Subtract leaving one object; layout shift 0, no console errors.

A slip against the hand-edit rule: the import line at the top of src/app.ts (adding fetchBooleanPreview) was changed with sed -i rather than an edit. It changed only that line, read back afterwards.

### src/app.ts, src/engine/service.ts · retopology and edge loop tools, cloth, pose, fibers, path render, Substance
- refers-to: src/app.ts :: BRUSHES (retopo, edge_loop), CLICK_TOOLS, TOPOLOGY (delete_faces, bridge), OBJECT_ACTIONS (cloth, pose, fibers, render, substance), clickTool (retopo, edge_loop), retopoPoints, retopoInto, objectAction, substanceOut; src/engine/service.ts :: localFile (samples, bounces)
- kind: rationale
- logged: v0.29.0 · 2026-10-12
Retopo quad is a click tool: four clicks on the sculpt (or on retopology already over it) make a quad that joins its neighbours; the sculpt stays active and the note counts corners. Edge loop is a click tool that inserts a loop across the clicked edge of a quad mesh. The Topology section gains Delete masked faces and Bridge borders; the Object section gains Drape cloth (the active object falls onto the others), Pose (the unmasked part turns about the view's pivot by the angle field around the deform axis), Grow fibers (in the paint colour), Path render (a 768-pixel, 64-sample path-traced PNG download) and To Substance (the package in ~/.formsculptor/substance, through the service). Verified under the window: two retopology quads sharing an edge (6 vertices), an edge loop adding 96 triangles to a quad-remeshed sphere, a 216 KB path render, a plane draped over the sphere as one step, fibers as a new object, and the Substance button refusing an object without UVs in words; layout shift 0, no console errors.

### src/app.ts · bevel edge, clip and add joint tools, UV point drag, From Substance
- refers-to: src/app.ts :: BRUSHES (bevel_edge, clip, zsphere), CLICK_TOOLS, clickTool (bevel_edge, clip, zsphere), OBJECT_ACTIONS (substance_sync), objectAction, substanceIn, wireUvView (Shift, Alt)
- kind: rationale
- logged: v0.30.0 · 2026-10-12
Bevel edge is a click tool: a click on or beside an edge chamfers its straight run (Ctrl rounds it). Clip is a click tool: a level cut through the clicked point as seen (the plane's normal is the camera's screen up, so everything above the click on screen goes); Ctrl slices into two objects. Add joint is a click tool on a skeleton base mesh: a joint just off the clicked surface, linked to the nearest joint, and the mesh remade. In the UV view, Shift-drag moves the nearest UV point and Shift-Alt-drag moves the points around it with falloff; a plain drag still moves the island. From Substance reads Painter's export from the object's package folder through the service and redraws.

### src/app.ts · Pose joint, Comb fibers and Cut fibers tools
- refers-to: src/app.ts :: BRUSHES (pose_joint, groom_comb, groom_cut), CLICK_TOOLS, clickTool (pose_joint, groom_comb, groom_cut); import cross
- kind: rationale
- logged: v0.31.0 · 2026-10-12
Pose joint is a click tool: the joint nearest the click turns by the Angle field about the Deform axis (Ctrl the other way). Comb fibers and Cut fibers are click tools on a fibers object: a short stroke through the click, as wide as the brush radius on screen, combs toward the screen's right (Ctrl left) or cuts there. Verified under the window: a click on a skeleton arm made a Pose Joint step, a click over a crown of fibers a Groom (comb) step; layout shift 0, no console errors.

### src/viewport/gl-renderer.ts, src/engine/service.ts, src/app.ts · opacity and emissive in the viewport
- refers-to: src/viewport/gl-renderer.ts :: MESH_VS (aFx, location 5), MESH_FS (vFx: screen-door opacity, emissive), PAINT_DEFAULT, PF, paintBuffer (28-byte stride), patch, render (vertexAttrib2f 5); src/engine/service.ts :: PAINT_FLOATS, parseUpdate, fetchMesh; src/app.ts :: Paint section (Opacity, Emissive), paintArgs
- kind: rationale
- logged: v0.26.0 · 2026-10-12
Paint is seven floats in the payloads and the GPU buffer, the new pair at attribute location 5. Opacity draws as a screen-door: interleaved gradient noise on the pixel position decides which fragments of a see-through surface are kept, so it needs no sorting and keeps depth where drawn (what lies behind shows through the gaps); the shadow and depth passes still treat it as solid. Emissive mixes the shaded colour toward the paint colour, unlit. Unpainted meshes and the face-group view set location 5 to solid, no glow. The Paint section's Opacity and Emissive sliders join roughness and metalness under Paint material too, for the brush and Fill object. Verified: a sphere filled at opacity 0.35 showed a torus through it, and at emissive 0.9 glowed whole; live paint strokes, paint layers and the smoke test unchanged; layout shift 0, no console errors.

### src/ui/commands.ts, menu.ts, menus.ts, command-palette.ts, app-dialogs.ts, chrome.ts, command-map.ts, prefs.ts, src/app.ts, src/ui/hotkeys.ts, src/ui/layout.ts · command registry, menu bar, palette, shortcuts dialog
- refers-to: src/ui/commands.ts :: Commands, commands, comboOf, normalizeCombo; src/ui/menus.ts :: menuDefs, tidy; src/ui/chrome.ts :: syncChrome, scheduleChrome; src/ui/command-map.ts :: PANEL_CMDS; src/app.ts :: SceneView.on, registerCommands, registerCatalogue, wireContextMenus, wireChrome; src/ui/layout.ts :: attachRail, togglePin; src/ui/hotkeys.ts :: legacyTarget; src/ui/prefs.ts :: takeLegacyKeys, persistShortcuts
- kind: rationale
- logged: v0.31.0 · 2026-10-16
Every action is now a command with a dotted id in one registry (src/ui/commands.ts), ported from FormVis: the menu bar, the Mod+K palette, the shortcuts dialog, the context menus, the rail pins and the panel buttons all read it, so a command added later (the terminal dock's) shows in all of them through onChange. Key combos are stored Alt, Shift, Mod, key; the engine catalogue's Mod, Shift order is normalised on the way in. Panel buttons keep their place, class and text and gain data-cmd; one delegated click listener on #app runs the command, and the old per-button handlers were kept as the commands' run bodies by changing only their registration (SceneView.on files the closure under the button's command id instead of adding a listener), so no behaviour was copied. A requestAnimationFrame-batched syncChrome writes each data-cmd node's title ("Label (Keys)", or the reason it is off), disabled and aria-pressed only when they differ, replacing the rail's MutationObserver mirroring and the three places that set disabled by hand (paint layer buttons, Apply to primitive, Remove light), whose rules are now the commands' enabled tests. Where a window command and a catalogue entry share an id (file.open, history.undo, topo.subdivide and the like) the window command is registered first and the catalogue only fills its group and description; every other catalogue entry becomes a command that adds the active object's id when its schema takes one. Menus are rebuilt from tidy(), which drops ids not registered and stray separators, each time one opens, so Window shows view.terminal, view.newTerminal, agent.claude and agent.chat as soon as they exist. Shortcuts live in localStorage formsculptor.prefs as {shortcuts: {id: [combos]}}; the old formsculptor.keys form (combo to action) is read once after the catalogue loads, mapped to command ids (legacyTarget), saved and removed. Rail pins were "Section/Label"; attachRail now keys them by command id and rewrites old ones on load. The Keys section's select and key box became one Keyboard shortcuts button. The single window keydown dispatcher runs commands.forEvent and skips while a dialog or menu is open, inside .xterm or .dock, and while typing in a field (Mod combos still run, except Mod+Z, Y, X, C, V and A in a text field). Behaviour changed: keys now work wherever focus is, not only with the canvas focused; buttons that need an object are disabled with "Select an object first" instead of doing nothing; there were no built-in tool letters, so brush keys are the commands tool.brush.<name> and carry over through the migration. Verified on the browser engine: eleven menus, the palette listing twelve rows, the shortcuts dialog, a pin menu on a panel button, the viewport menu, aria-pressed following the chosen brush; no console errors.

A slip against the hand-edit rule: app-dialogs.ts, a file made this turn, had an unused import and a stray export removed with a one-off node replace, and a keydown line removed the same way; both read back afterwards. A stray empty python3 call was also made and did nothing.

### ui/terminal-dock.ts, ui/chat-pane.ts, ui/terminal.ts · the dock
- refers-to: ui/terminal-dock.ts, ui/chat-pane.ts, ui/markdown.ts, ui/terminal.ts, io/api.ts, io/events.ts, styles/dock.css
- kind: rationale
- logged: v0.32.0 · 2026-10-16
Ported from FormSurface's shell (itself FormLit's, same owner, Apache-2.0). The family's fl-dock and fl-chat class names are kept so the shared files stay comparable; dock.css maps their surface, line and danger tokens onto ours inside .fl-dock. The dock is a grid row ("dock") between the stage and the status bar, 0 px until opened, then --dock-open-h (280 px, 140 px to 70 percent, saved in formsculptor.dock_h). Terminals mount once and are shown or hidden, tabs are appended one node at a time, chat is append-only; nothing rebuilds. Keys inside the dock stop propagating except Mod+J, so Ctrl+C reaches the shell rather than the app. The commands are only enabled with a service behind the window (the browser engine has no PTY). Verified under 4x CPU throttling: a typed echo came back, Chat and Claude Code tabs opened, layout shift 0, no console errors.

### Curve tool · corners, rounding and the engine's samples
- refers-to: app.ts :: SceneView.curve, drawCurve, resample, setCorners, tubeFromCurve, tubeArgs, curveEnd
- kind: rationale
- logged: v0.32.0 · 2026-10-16
The waiting curve keeps sharp and round per handle. A click on a handle (moved under 3 px) picks it; Alt-click toggles a corner. Corner round is a share of the mean span (half a span at 100) so the slider means the same at any scale; the engine clamps a fillet to half of either neighbouring span. The drawn line is sample_curve's output, re-asked after each change with one call in flight and one queued; while a handle is dragged the old screen spline shows so the drag stays live. Apply curve and Tube from curve send the same points, so what is built is what was drawn. When the engine lacks sample_curve (an older browser engine) the window falls back to its own spline.

### ui/i18n.ts · 26 languages
- refers-to: ui/i18n.ts, ui/i18n/langs.ts, i18n/*.json, main.ts, ui/chrome.ts
- kind: rationale
- logged: v0.32.0 · 2026-10-16
FormVis's live model with FormLit's {name} templates: JSON tables keyed by the English, loaded with import.meta.glob only when chosen, regional variants falling back to their parent. The page translator runs from a MutationObserver (before paint) and keeps each node's English in a WeakMap, so a switch back is exact; it skips the terminal, chat, object names and history. The saved language (formsculptor.language) loads before the shell is built, so the first paint is already translated. The menus and data-cmd titles re-label through onLanguage. Changing language measured a layout shift of 0.003 as label widths change; that is the text itself, not a rebuild.

### Viewport backdrop · studio gradient and infinite floor grid
- refers-to: viewport/gl-renderer.ts :: GRADIENT_FS, FLOOR_FS, Backdrop, drawGradient, drawFloor; viewport/viewport.ts; app.ts :: SceneView.wireBackdrop; styles/tokens.css --viewport-*
- kind: rationale
- logged: v0.33.0 · 2026-10-16
Replaces the flat clear colour, the 41-line GL_LINES grid that ran aliased to the horizon, and the CSS dot pattern behind the canvas. Modelled on the two references: Nomad's Background (Gradient with a midpoint factor, or a flat Color) and ZBrush's document gradient with a floor grid toggled from the canvas. The gradient is a full-screen pass with a soft vignette and a one-step noise dither so an 8-bit ramp never bands. The floor is one full-screen pass that intersects each pixel's view ray with y = 0 and draws anti-aliased lines from screen-space derivatives; the cell size steps by tens with the camera distance and the two scales cross-fade, so the grid never pops while zooming. It fades with distance and at grazing angles, so a front or side view shows a clean horizon and the ortho top view a crisp blueprint. Depth writes are off, so meshes always cover it. The theme's tokens seed the colours; the View panel's Background controls and Shift+P (Floor Grid) override them and persist in formsculptor.backdrop. The stage's CSS gradient matches the GL one, so the frame before the first draw shows no seam.

### Selection Sets and channel clipboard in the window
- refers-to: app.ts :: registerCommands (selection.*, channel.*), SceneView.selectionSet; ui/menus.ts MASK and EDIT
- kind: rationale
- logged: v0.33.0 · 2026-10-16
Mask > Selection Sets saves, restores and deletes by name through the PromptDialog: saving offers the next free "Set n", restoring and deleting offer the newest set and refuse a name the object does not have. Edit > Channels copies, pastes and clears the mask, groups or paint through the engine's one-channel clipboard, so they go from one object to another with matching topology. The labels sit in a [id, label, ...] table so setup/i18n.py extracts them.

### Curves round two in the window · per-point shape, snapping, chain drag; the Selection Sets list
- refers-to: app.ts :: curvePoints, tubeArgs, snapHandle, chainDrag, setPointShape, curveEnd, patchSets
- kind: rationale
- logged: v0.34.0 · 2026-10-16
Each handle carries a radius factor and a twist; curvePoints sends radius as Tube radius times the factor, so changing Tube radius rescales every handle together. Radius along path maps straight onto curve_tube's radius_mode (uniform, start_end with End radius % of Tube radius, per_point); setting a handle's Point radius % or Point twist switches the mode to Per point so the change shows. Picked handle points pass through snapHandle: lifted along the surface normal by Surface offset, then rounded per axis to Grid snap's step when it is above 0. Drag as chain sends the handles to curve_ik with the first handle pinned (or nothing pinned when the first is dragged), so the spacing holds and the rest follow; chained handles leave the surface, which suits tubes more than surface strokes. The Selection Sets list reuses its option elements and is refreshed after every scene refresh with one selection_set_list call; Restore and Delete use the chosen set.

### Making curves: clicked points, the last stroke, borders
- refers-to: app.ts :: curveEnd (the click branch), lastStroke, editLastStroke, curveFromBorder, adoptCurve; ui/menus.ts SCULPT
- kind: rationale
- logged: v0.35.0 · 2026-10-16
With the Curve tool, a press that moves under 8 px adds a handle where it lands (through snapHandle) instead of only explaining itself, so a curve can be built point by point; a curve of one handle waits undrawn until the second. Every brush stroke keeps its rays (up to 4000) in lastStroke; Edit last stroke as curve runs the engine's undo, re-picks up to 24 evenly chosen rays on the restored surface, and waits on them as a curve with the stroke's own brush as the one Apply curve uses. That is the re-edit of 11.3 without its live part: the stroke is not re-applied while handles move, only on Apply curve. Curve from mask edge and Curve from group border call curve_from_border (16 points, Surface offset as its offset; the group number asked through the prompt dialog) and set Closed path from the reply. Hint and note strings that already had translations were kept and new sentences added beside them, so no table carries a stale key.

### Tube profile editor, saved profiles, intensity along a curve
- refers-to: app.ts :: profilePts, drawProfile, wireProfile, profileAction, savedProfiles, listProfiles, tubeArgs, applyCurve; styles/shell.css .profile-editor; ui/command-map.ts curve.profile_save, curve.profile_delete
- kind: rationale
- logged: v0.36.0 · 2026-10-16
The editor is a fixed 160 px square SVG (viewBox 200, 76 units per radius, so a point can sit up to 1.2 radii out) that is always in the panel, so choosing a profile never moves anything below it. It shows the outline as straight segments with square corner marks and round smooth marks; the engine does the real rounding of smooth points when it builds the tube, so the editor is a control cage, not a preview of the curve. A press near a point drags it, Alt toggles corner or smooth, Shift removes it (never below three), and a press anywhere else inserts a smooth point after the nearest edge and drags it. Any change sets Tube profile to Drawn below (or keeps the saved profile chosen) and, on release, re-sends the profile to an unchanged tube through edit_build. Saved profiles live in this browser under formsculptor.profiles and appear in Tube profile as saved:Name after the presets; choosing one loads its points into the editor. Intensity along curve turns Apply curve's points into [x, y, z, pressure] with fade in, fade out, both, or an 8-cycle pulse (0.1 to 1), and forces pressure_strength on for that stroke so the shape counts even with Pressure strength off.

### The falloff on the curve model; the repeater's ramp and jitter
- refers-to: app.ts :: falloffAt, falloffSent, falloffSharp, drawFalloff, brushState, buildAction (array_*)
- kind: rationale
- logged: v0.37.0 · 2026-10-16
The falloff editor's five points are corners by default, which is exactly the straight-line curve sent before, and then only the five weights are sent, so existing strokes are unchanged. Alt-click makes a point smooth (round mark): the window evaluates Hermite spans, with a corner's tangents along its span and a smooth point's along its neighbours (the curve model's sharp and smooth, in one dimension), clamps to 0..1, and sends 33 samples, which the engine's falloff_curve accepts (2 to 64 weights) and interpolates linearly. Last copy scale, Array jitter % and Jitter seed map onto array_object's scale ramp and jitter for line, radial and curve arrays (never mirror, which refuses them); one jitter share sets turn up to 45 degrees, scale up to half and offset up to a quarter of the object's largest side, so one slider reads as how messy the repeat is.

### Slice curve and Trim curve
- refers-to: app.ts :: curveSideEnd, shapeEnd, BRUSHES (slice_curve, trim_curve), SHAPE_TOOLS; ui/menus.ts MASK and TOPOLOGY
- kind: rationale
- logged: v0.38.0 · 2026-10-16
ZBrush's Slice Curve and Trim Curve, from an open stroke. The stroke (thinned to 3 px steps) becomes the edge of a half-plane on screen: its ends are carried on along their last direction and the outline closes on the stroke's left, or its right with Ctrl. The carry-on distance is only as far as the object's projected box from either end plus 24 px, not a fixed multiple of the view: cut_shape builds its result over the outline's extent, and a first version that reached four view diagonals out returned a 116-triangle lump from a 20480-triangle sphere (3244 after the change, the true cap). Slice sends the outline to group_from_outline with the camera's view-projection, as the lasso mask does, and turns on the face-group view; Trim sends it to cut_shape keeping the outside, as the lasso cut does.

### Live stroke re-edit; Snake from mask and Bridge two groups
- refers-to: app.ts :: editLastStroke, reapplyLive, historyDone, liveCurve, liveDone, curveEnd (handle move), curveBridge; ui/menus.ts SCULPT
- kind: rationale
- logged: v0.39.0 · 2026-10-16
With Live re-edit on (the default), Edit last stroke as curve undoes the stroke, applies the curve at once (so nothing visibly changes) and remembers that curve and the history position after it. Each handle move then undoes that application and applies again, so the stroke follows its handles and the history does not grow (measured: 9 steps after the stroke, 9 after the conversion, 9 after a move). Before undoing, the history position is compared with the one remembered; if anything else happened in between, the live link ends with a note instead of undoing someone else's step. Off, it behaves as in 0.35.0. Snake from mask and Bridge two groups call curve_bridge (Snake: the mask border scaled to Snake end size % at the far end; Bridge: two group numbers through the prompt dialog), along the waiting curve's handles when there is one, lifted by Surface offset.

### UV section: Relax, Even density, Density
- refers-to: app.ts :: uvMeasure, the UV buttons; ui/command-map.ts uv.relax_view, uv.equalize_view, uv.density_view; ui/menus.ts PAINT
- kind: rationale
- logged: v0.40.0 · 2026-10-16
Three buttons over the engine's new UV commands. Relax runs uv_relax conformally for 60 passes with island borders held, and reports the stretch before and after. Even density runs uv_equalize (which repacks) and reports the spread before and after. Density reads uv_density at the Bake size, so the number matches the maps that will be baked. Notes are {name} templates through t(), filled after translation. Both changing actions refresh the scene and redraw the layout picture in place.

### UV section: Sew to neighbour
- refers-to: app.ts :: uvSew, the UV buttons; ui/command-map.ts uv.sew_view; ui/menus.ts PAINT
- kind: rationale
- logged: v0.41.0 · 2026-10-16
The window has no way yet to name a second island or a vertex path, so cut and straighten stay agent and palette commands for now, and sewing takes the chosen island and tries uv_sew against each other island in turn (the chosen one moves), stopping at the first that shares mesh edges; any refusal other than no_shared_edges is shown and stops the search. Island numbers come from uv_density's list, which uses the same island numbering. The choice is cleared after a sew because the numbering changes.

### UV view: pins and vertex paths
- refers-to: app.ts :: uvMark, drawUvMarks, uvPathAction, uvPins, uvPath, uvMarksFor, drawUv, wireUvView; styles/shell.css .uv-marks
- kind: rationale
- logged: v0.42.0 · 2026-10-16
Alt-click in the UV view pins the nearest UV point (uv_pick, radius 0.03; a second Alt-click unpins), and Relax sends the pins as uv_relax's pinned list. Ctrl-click starts a path at the nearest point and each further Ctrl-click in the same island extends it by uv_path's shortest edge path, so a few clicks give a long chain; a click in another island starts a new path. Cut along path and Straighten path send the chain to uv_cut and uv_straighten (clicking the first point again closes a loop for a cut). The marks are an SVG over the picture in UV space (0 to 100, v up) with pointer events off, drawn through the clicked points and pins only, not every vertex of the path. Pins and the path name vertices of the layout as it stood, so they are dropped whenever the active object or its revision changes.

### UV checker in the viewport; Automatic charts
- refers-to: viewport/gl-renderer.ts :: setChecker, dropChecker, expand (given colours), refreshExpanded, uChecker in the mesh shader; app.ts :: syncChecker, checkerFor, refresh; UV seams choices
- kind: rationale
- logged: v0.43.0 · 2026-10-16
UVs are per triangle corner and the viewport draws indexed vertices, so the checker reuses the per-corner copy the face-group view already builds (expand), with the corner's UV in the colour attribute instead of a group colour; the fragment shader turns it into a 16 by 16 checker and paints anything outside 0..1 red. The copy follows sculpting through refreshExpanded like the group copy, is dropped with the mesh on any re-upload, and the window refetches get_uvs only when the active object or its revision changes (checkerFor). It draws for the active object only and wins over the group colours while on. UV seams gains Automatic charts (uv_unwrap seams charts at its default 45 degrees).

### UDIM tiles in the UV section and the checker
- refers-to: app.ts :: uvTileNumber, uvTileOffset, uvAt, drawUv, drawUvMarks, wireUvView, uvTiles; ui/command-map.ts :: uv to_tile, tiles_group, tiles_island, tiles; viewport/gl-renderer.ts :: the uChecker branch
- kind: rationale
- logged: v0.44.0 · 2026-10-16
The UV view shows one tile at a time (the UV tile field, 1001 to 1100) through get_uv_layout's tile, so the picture and its square stay the same size and nothing below moves. uvAt returns real UV space (the tile's corner added), so island drags, point drags, pins and paths all work unchanged in any tile; only the island pick, which asks get_uv_layout, sends the point inside the tile, and the marks overlay subtracts the corner. Tiles by group and Tiles by island run uv_pack_udim starting at the shown tile, Island to tile moves the chosen island into it, and List tiles reports each tile's island count. The checker tints every tile past 1001 with its own hue so tiles can be told apart on the model, and red now means outside the UDIM range (u below 0 or from 10, v below 0) instead of outside the unit square.
