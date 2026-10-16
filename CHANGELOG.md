# Changelog

Versions follow `v*.*.*` with dots. Dates are UTC.

## 0.38.0 · 2026-10-16

### Added

- `group_from_outline`: the faces inside a screen outline (lasso or box, in a named view or the window's own camera), optionally only those facing the camera, into a new or given face group as one undo step.
- Slice curve and Trim curve tools: drag a line across the object; Slice puts the faces on one side into a new face group, Trim cuts that side away (Ctrl for the other side).
- 128 engine commands; 757 interface strings in every language.

## 0.37.0 · 2026-10-16

### Added

- `array_object` gains `scale_start` and `scale_end` (a scale ramp across the copies) and `jitter` {position, rotation, scale, seed} (deterministic random variation), for line, radial and curve arrays, linked copies included. Window: Last copy scale, Array jitter % and Jitter seed.
- The falloff editor's points can be corners or smooth (Alt-click), as on curves; all corners behaves exactly as before.
- 752 interface strings in every language.

## 0.36.0 · 2026-10-16

### Added

- A tube profile editor in the panel: draw a closed cross-section (drag points, click an edge to add one, Alt-click for a corner or a smooth point, Shift-click to remove); Tube profile gains Drawn below and saved profiles (Save profile, Delete saved profile, kept in this browser). An unchanged tube follows the edits.
- Intensity along curve for Apply curve: even, fade in, fade out, fade both ends or pulse, sent as per-point pressure.
- 748 interface strings in every language.

## 0.35.0 · 2026-10-16

### Added

- `curve_from_border`: curve control points along the border of a face group or of the mask, longest border first, evenly spaced, optionally lifted off the surface; read-only.
- Making curves in the window: click with the Curve tool to add points one at a time; Edit Last Stroke as Curve undoes the last brush stroke and brings its path back as handles to move and apply again; Curve from Mask Edge and Curve from Group Border (Sculpt menu and the panel).
- 127 engine commands; 732 interface strings in every language.

## 0.34.0 · 2026-10-16

### Added

- `curve_ik`: drag one control point of a curve and the rest follow as a chain of fixed-length segments (FABRIK), with the first, last, both or neither end pinned; read-only, returns the new points, whether the target was reached and the segment lengths.
- Curves in the window: a radius and a twist per handle (Point radius %, Point twist), Radius along path (Uniform, Start to end with End radius %, Per point) for tubes, Surface offset and Grid snap when placing handles, and Drag as chain (through `curve_ik`).
- A Selection Sets section in the panel: the active object's sets in a list, with Save, Restore and Delete.
- `./formsculptor doctor` checks that every translation table covers the interface strings.
- 126 engine commands; 720 interface strings in every language.

## 0.33.0 · 2026-10-16

### Added

- Selection Sets: `selection_set_save`, `selection_set_restore`, `selection_set_delete`, `selection_set_list` keep and bring back named snapshots of an object's face groups, mask, vertex paint, layer strengths and UVs (all or chosen channels, one undo step, saved in projects). Window: Mask > Selection Sets.
- Channel clipboard: `channel_copy`, `channel_paste`, `channel_clear` move the mask, face groups or paint from one object to another with the same topology. Window: Edit > Channels.
- Viewport backdrop: a gradient with midpoint and vignette or a flat colour, and an infinite anti-aliased floor grid that scales with distance and fades at the horizon. View panel: Background, its colours, midpoint, vignette and Floor grid (kept between sessions); View > Floor Grid (Shift+P) and Gradient Background.
- All 26 languages complete: 703 strings in every table.
- 125 engine commands.

### Changed

- The viewport no longer draws a flat clear colour with a 41-line grid running aliased to the horizon, and the stage no longer shows a dotted pattern behind the canvas.

## 0.32.0 · 2026-10-16

### Added

- Claude in the window: a terminal dock under the viewport (`Mod+J`) with real shells, a Claude Code tab (`Shift+Mod+J`) that runs your own Claude Code connected to formsculptor's MCP tools with no permission prompt, and a Chat tab (`Alt+Mod+J`) where Claude answers in Markdown and sculpts through the same tools, showing each tool call and the views it looked at. Service: `/api/commands/{id}` (window-only commands), `/api/events` (SSE), `WS /api/terminal/{id}`.
- `./build_app` and `build_app.bat`: one command builds the installable desktop app (tools checked, engine, Python module, window, frozen sidecar, Tauri bundle; a disk image and install on macOS, a deb on Linux, msi and nsis on Windows); `./formsculptor app` runs it.
- Commands in the window: every panel button and engine command is a command with label, keys, enabled and checked state; a menu bar (File, Edit, Add, Sculpt, Mask, Topology, Object, Paint, View, Window, Help), right-click menus on the viewport and panel buttons, a `Mod+K` palette and a `Mod+/` shortcut editor with conflict handling.
- Curves: a shared curve model in the engine with smooth or sharp points, fillet rounding of a corner by a radius, Catmull-Rom, B-spline or straight interpolation and closed loops. `sample_curve`; `warp_along_curve` (bend an object along a path); `curve_tube` gains profiles (circle, square, triangle, star, flat or your own outline), round or flat caps, hollow pipes, twist, spirals and start-to-end radius; `lathe` profiles take rounded corners; `edit_build` sets one point's sharp, round, radius or twist. Window: Alt-click a curve handle for a corner, Corner round, Path shape, Closed path, Tube from curve and the tube settings.
- 26 interface languages, English by default (View > Language), switched live; `./formsculptor i18n extract|check`. German seeded; the other tables followed in 0.33.0.
- 118 engine commands.

### Changed

- The frozen engine chooses its own port before starting, so the address handed to Claude Code is real.
- Panel buttons that need an object are disabled with a reason instead of doing nothing; keys work wherever focus is.

## 0.31.0 · 2026-10-12

### Added

- `pose_joint`: a skeleton_mesh object is posed by its own skeleton: one joint turns, the bones beyond it follow, and the mesh moves with automatic weights by distance to each bone, blended across the joint (softness). Works after sculpting; undo brings back the matching skeleton. Window: the Pose joint tool (the Angle field about the Deform axis, Ctrl the other way).
- `groom_fibers`: comb, cut, curl or lengthen fibers from grow_fibers along a stroke; grow_fibers keeps each fiber's guide curve for it. Window: the Comb fibers and Cut fibers tools.
- Desktop: the Linux AppImage, deb and rpm rebuilt at 0.30.0 (the 92.5 of 97 release) and the AppImage launched under Xvfb: window up, service connected, 114 commands in its bundled engine, bevel_edge and clip_plane run through it.
- 116 commands in all. Tests: engine 51, service 18.

## 0.30.0 · 2026-10-12

### Added

- Guided quad remesh: `quad_remesh` takes `guides` (curves as point lists); the grid's edge loops turn to run along their mean direction while flat sides stay square to the world.
- `bevel_edge`: a chamfer or round fillet of a given width on the straight run of sharp edges nearest a point, cut exactly with a prism so the rest of the mesh is untouched. Window: the Bevel edge tool (Ctrl for round).
- `clip_plane`: a flat exact cut along a plane, keeping one side or slicing into two capped objects. Window: the Clip tool (a level cut through the click as seen; Ctrl slices).
- Skeleton editing: `skeleton_mesh` objects stay editable; `edit_build` takes `add` (a joint linked to the nearest) and `drag` (move the nearest joint), and `spheres`, `links`, `blend`, `resolution`. Window: the Add joint tool.
- `uv_move_point`: drag one UV vertex, or every corner within a radius with falloff. Window: Shift-drag in the UV view (Alt for the soft reach).
- `substance_sync`: Painter's exported BaseColor, Roughness and Metallic read back onto the paint and the colour map. Window: From Substance; service: `/api/engine/substance/sync`.
- Path render: added directional `lights`, `sun` off, depth of field (`aperture`, `focus`) and `pass` (depth, normal, ao). Service: `pass` and `aperture` on `/api/engine/view`.
- 114 commands in all. Tests: engine 49, service 18.

## 0.29.0 · 2026-10-12

### Added

- `cloth_drape`: cloth falling under gravity onto the other objects and a floor, edges keeping their length, masked vertices pinned. Window: Drape cloth.
- `pose`: Transpose-style posing; the unmasked part turns about a joint, with falloff. Window: Pose.
- `grow_fibers`: hair, fur and grass as tapered fibers from the masked area, with droop, clumping and colour. Window: Grow fibers.
- Path-traced rendering: `get_view` with `render: "path"` (samples, bounces): sky and sun light, soft shadows, bounced light, glossy paint, glow and see-through paint. Window: Path render; service: `/api/engine/view?render=path`.
- Manual retopology: `retopo_face` adds quads snapped onto the sculpt and joined to their neighbours. Window: the Retopo quad tool.
- Edge tools: `insert_edge_loop` (window: the Edge loop tool), `bridge` and `delete_faces` (window: Bridge borders, Delete masked faces).
- `send_to_substance`: a Substance 3D Painter package (low mesh with UVs, high mesh, baked maps under Painter's names, README). OBJ export writes UVs. Window: To Substance; service: `/api/engine/substance`.
- 110 commands in all. Tests: engine 42, service 18.

## 0.28.0 · 2026-10-12

### Added

- Exact mesh booleans: `boolean` cuts along the true intersection by default (`method: "exact"`), keeping sharp edges, flat faces, sculpted detail and paint; open input falls back to the voxel boolean and says why; `method: "voxel"` and `blend` keep the voxel route. Built on boolmesh (MPL-2.0).
- Live boolean: the window draws the exact union, difference or intersection of two objects in their place and follows as either changes; Union, Subtract or Intersect keeps it. Service route `/api/engine/boolean_preview`; also in the browser engine.
- Tests: exact booleans (notch geometry, coplanar faces, paint, fallback, nothing left), the preview route, the browser engine's preview (engine 34, service 17, browser engine 14 checks).

### Changed

- Minimum Rust is 1.88 (boolmesh needs it).
- The window's note is a fixed three-line box, so a long note no longer moves the sections below it.

## 0.27.0 · 2026-10-12

### Added

- The browser engine: the whole engine (all 102 commands, live strokes, files) built to WebAssembly as `fsc-wasm` (`./formsculptor engine wasm`, 2.4 MB). The window uses it when no engine service answers, or at `?engine=wasm`, and then needs no service at all; agents, plugins, autosave, the bridge and NodeMode still need the service.
- USD as text: `.usda` export (a Mesh prim per object with colour, opacity and UVs) and import, checked against Pixar's own library.
- Tests: the browser engine in Node (`frontend/e2e/wasm_smoke.mjs`, run by `./formsculptor test engine`), its C surface natively, USD (engine 33 and the browser engine's 12 checks, service 16). CI builds the browser engine.

### Changed

- Engine threads are the `parallel` feature of fsc-commands (on for the service and tests, off in the browser).
- Live strokes, the mesh payload and the maps zip moved from the Python module into `fsc_commands::live`, shared by both engines.

## 0.26.0 · 2026-10-12

### Added

- Opacity and emissive paint channels: paint is seven floats per vertex. `paint_fill` and the paint brush take `opacity` and `emissive`; the viewport draws see-through paint as a screen-door and glow in the paint colour; GLB carries colour alpha with blending and the glow, bakes put opacity in the colour map's alpha and write a glow map (`emissive.png`, `emissiveTexture`). Window: Opacity and Emissive in the Paint section.
- UV editing: `uv_unwrap` takes `seams: "groups"` to cut along face groups; `get_uv_layout` pictures the layout and lists its islands; `uv_transform` moves, turns or scales an island; `uv_pack` packs islands back (102 commands in all). Window: a UV section with the layout, click to choose and drag to move islands, Turn 90, Scale up and down, Pack. The scene summary says whether an object has UVs.
- Tests for each of these (engine 32, service 15).

### Changed

- Projects save seven floats of paint; projects from before 0.26.0 open opaque without glow.

## 0.25.0 · 2026-10-12

### Added

- Dynamic subdivision: a smooth preview that draws objects Loop-subdivided 1 to 3 levels for display while every command and stroke works on the sculpted mesh. Window: Smooth preview in the View section; service: `/api/engine/mesh/{id}?smooth=N`; agents: `get_view` takes `smooth`.
- `uv_unwrap` lays each chart flat by least-squares conformal maps by default (`method: "project"` keeps the box projection) and reports `stretch`, the mean and worst angle distortion. Window: Unwrap UVs reports both.
- Tests for each of these (engine 29, service 14).

### Changed

- The modeling skill's "not yet available" list no longer says there is no bevel or turntable; it names edge loops, chosen-edge chamfers and hand-placed seams instead.

## 0.24.0 · 2026-10-12

### Added

- FBX: binary FBX 7.4 export (meshes, normals, vertex colours, sculpt layers as blend shapes) and 7.x import (placement, colours, compressed arrays, welded vertices), checked both ways against assimp. Window: FBX in Export format and the file picker.
- `set_material`: subsurface scattering and transmission per object, saved in projects (99 commands in all). The viewport's lit shading draws subsurface with real thickness from the key light's depth map and refracts the environment through transmissive objects. Window: Subsurface and Transmission in the Paint section.
- 3D mouse: 3Dconnexion devices over WebHID pan, zoom and turn the view (Chrome and Edge). Window: 3D mouse in the View section.
- Tests for FBX and materials (engine 26, service 13).

## 0.23.0 · 2026-10-12

### Added

- `bevel_edges`: a volumetric bevel that rounds convex edges, fills concave ones, or both, faces staying in place. Window: Bevel edges in the Object section.
- `edit_build`: tubes and lathes stay editable (new curve or profile, points moved by index, radius, segments) until sculpted; `get_scene` shows `editable_build`. Window: Tube radius and Lathe profile remake the active one.
- `bridge_send` and `bridge_sync`: a two-way file bridge to any app (98 commands in all). Window: Bridge to app, with a sync every 3 seconds; service route `/api/engine/bridge`.
- Plugins declare `PARAMS` (fields drawn in the Plugins section) and `CLICK` (a surface tool, the Plugin tool); a new example, `rivets`.
- Tests for each of these (engine 24, service 12).

### Changed

- Compiled desktop apps wait until parity reaches 95 percent, by the owner's direction; the Linux build scripts stay in the tree.

## 0.22.0 · 2026-10-12

### Added

- Tests, now that the build is stable: 21 engine tests through the command registry (`native/crates/fsc-commands/tests`), 10 service tests on the FastAPI app (`backend/tests`), and a window smoke test (`frontend/e2e/smoke.py`). `./formsculptor test [all|engine|service|e2e]` runs them; CI runs the engine and service suites.
- Linux desktop bundles (AppImage, deb, rpm) rebuilt at this version and launched on their own bundled engine.

### Fixed

- CI's shell syntax check ran `bash -n` over the Python files in `setup/` and failed; it now skips them.

## 0.21.0 · 2026-10-12

### Added

- `insert_along`: copies of a primitive or another object strung along a surface stroke, spaced and turned to follow it. Window: the Insert brush.
- `group_auto` by `paint` (painted regions, slivers folded away) and by `uv` (UV islands); `group_paint`, a group brush along a stroke. Window: Groups by paint, Groups by UV and the Group brush.
- `record_animation`: a timeline of eased keyframes for the camera's turn, height and zoom and for sculpt layer strengths, rendered to a looping animated PNG; `record_history`, a movie of every undo step from the start (94 commands in all). Window: Add key, Record animation, Clear keys and History movie; service routes `/api/engine/animation` and `/api/engine/history_movie`.
- glTF in its separate form: `export_mesh` to a `.gltf` writes a `.bin` beside it, and `import_mesh` reads a `.gltf` whose buffers are files beside it.

## 0.20.0 · 2026-10-12

### Added

- `vdm_stamp` and `vdm_capture`: vector displacement stamps that can overhang (horn, hook, mushroom built in) and new maps captured from a sculpted plane. Window: the VDM stamp tool, Stamp map and Capture VDM.
- `text_mesh`: solid 3D text in a rounded stroke font. `extrude_shape`: closed outlines (holes by even-odd) extruded with rounded edges. Window: Text with Build text, and Extrude outline from the front Silhouette outline.
- `boolean` `blend`: a filleted seam, so a union fuses two parts and a difference leaves a rounded cut. Window: Boolean blend.
- `cut_shape`: a closed outline extruded through the object along the view, cut away or kept (90 commands in all). Window: Lasso cut and Lasso trim.

### Fixed

- The cube and plane primitives were wound inward since 0.2.0: the cube was inside out and the plane faced down. Both face out now, and every primitive passes the print check.

## 0.19.0 · 2026-10-12

### Added

- `shadow_box`: a closed solid from two or three outlines seen from the front, side and top, with a blend for rounded meeting edges (85 commands in all). Window: the Silhouette tool keeps an outline per view, then Build shadow box and Clear outlines.
- `array_object` modes `mirror` (one reflected copy across an axis) and `curve` (copies spaced along a path, turned to follow it), and `linked` copies that follow every change to their original until edited themselves; `get_scene` shows `instance_of`. Window: Mirror copy, Array on curve (along the Curve stroke's path) and Linked copies.
- Interface customization: every panel section folds under its heading, the panel can sit on the left, it comes in small, normal and large sizes, and any panel button can be pinned to the tool rail by right-clicking it (Clear rail empties it), all kept in the browser.

## 0.18.0 · 2026-10-11

### Added

- `paint_project`: paint an image onto the surface from a view (only what that view sees, fading at grazing angles) or tiled triplanar, into the active paint layer (84 commands in all). Window: Project image and Tile image with Image opacity.
- Reference images on a plane in the scene (front, side or top) as well as over the view.
- Python scripting: `./formsculptor script FILE [JSON]` runs a script with `fsc.call(command, **args)` on the running engine or its own.
- Plugins: Python files in `plugins/` or `~/.formsculptor/plugins` with a `run(fsc, args)`, listed and run from the window's Plugins section (`/api/plugins`); two examples, a rock and a display pedestal.

## 0.17.0 · 2026-10-11

### Added

- `edit_primitive`: change a primitive's size, detail, height or thickness until anything else changes it; `get_scene` shows `primitive` while it is editable (83 commands in all). Window: Primitive size, detail and height with Apply to primitive.
- `surface_noise` noise editor: `pattern` (value, ridged, turbulence, cells, alpha), `octaves`, `gain`, `stretch`, and `alpha` for the tiled alpha pattern. Window: the noise editor fields in the Layers section.
- Lighting: the key light's direction, height, strength and colour, in the view or fixed in the world; shadows from the key light; up to three added lights; a ground shadow catcher.

## 0.16.0 · 2026-10-11

### Added

- `lattice_deform`: free-form deformation by a lattice of 2 to 6 control points per axis around the object, held by the mask (82 commands in all).
- Window: a Transform tool (an on-canvas gizmo: arrows move, rings turn, squares scale along an axis, the centre square scales evenly, previewed live and sent as one transform_object) and a Lattice tool (drag the cage's points).
- Touch: two fingers pan and pinch-zoom; a finger's stroke starts once it moves, so a pinch leaves no dab.
- Pen tilt turns the brush alpha (`tilt` on the stroke socket's samples).

## 0.15.0 · 2026-10-11

### Added

- Image-based lighting in the window: Studio, Sunset, Overcast and Night environments, or a loaded Radiance .hdr or image, lighting lit (PBR) shading with diffuse and roughness-blurred reflections; rotation, exposure, the fixed lights' share, tone mapping (Reinhard, ACES filmic, none), and the environment shown behind the model with a blur.
- Post effects: screen-space ambient occlusion, bloom, depth of field focused at the orbit target, vignette and film grain, each off by default.
- A Lighting section between View and Keys.

## 0.14.0 · 2026-10-11

### Added

- Paint layers: `paint_layer_add`, `paint_layer_set` (name, blend, opacity, visibility, move up or down), `paint_layer_select` (a layer or the base), `paint_layer_delete`, `paint_layer_merge` (down, keeping the look) and `paint_layer_flatten` (81 commands in all). Blend modes: normal, multiply, screen, overlay, add.
- The paint brush, `paint_fill`, `paint_clear` and smudge work on the active paint layer or the base; inverting the paint brush erases a layer. Undo covers both the layer and the composite.
- Projects save the paint base and layers; `get_scene` lists `paint_layers` and `active_paint_layer`.
- Window: Paint into, Blend, Opacity and Visible in the Paint section, with New paint layer, Delete paint layer, Merge down, Flatten paint, Move up and Move down.

## 0.13.0 · 2026-10-11

### Added

- `sculpt_stroke`: `falloff_curve` (a drawn brush profile from centre to rim), `stroke: "spray"` with `spray` (seeded scatter, random turn and strength per dab), `pressure_gamma`, `pressure_strength` and `pressure_size`.
- Window: a falloff curve editor with the Falloff presets, Pressure curve, Pressure strength and Pressure size, a Spray stroke, a Drag dot tool, and the Curve stroke tool (draw on the surface, drag handles, Apply curve with the last brush as often as needed, Clear curve).

### Changed

- Drag stamp now uses the last sculpting brush and all Brush settings, not always Draw.

## 0.12.0 · 2026-10-11

### Added

- Brush alphas: `alpha` on `sculpt_stroke` and in the window shapes every dab with a gray stamp (built-ins square, noise, cracks, rings, star, stripes, dots), turned by `alpha_angle`, upright by `alpha_up` or rolled along the path with `stroke: "roll"`; `alpha_load` adds any PNG by name (75 commands in all).
- `smudge` brush: drags paint colour along the stroke.
- Window: Alpha, Alpha angle and Stroke fields with Load alpha; a Drag stamp tool (one dab sized and turned by the drag); Smudge.
- Image matcaps (Load matcap in the View section) and Shade active only, for a different shading per object.
- Custom hotkeys: a Keys section to bind any tool, view, file action or registry command; the registry's declared keys are now the defaults, and changes are kept in this browser.

## 0.11.0 · 2026-10-11

### Added

- Face groups: `group_auto` (by angle or by parts), `group_from_mask`, `group_clear`, `mask_by_group`; groups carried through subdivision and levels, saved in projects, shown in colour (Face groups in the View section, `shading: "groups"` in get_view).
- `hide_faces`: hide, show or isolate faces by group or mask, invert, show all; hidden faces are not drawn, rendered or hit, and their vertices hold still.
- `array_object` (linear or radial, separate or merged), `insert_mesh` (a primitive or a copy at a point on a surface), `set_folder` with folder visibility, `record_turntable` (an animated PNG), `subdivide` with `sharp_border` (74 commands in all).
- Window: Group from mask, Auto groups, Hide masked, Invert hidden, Show faces; Mask group, Hide group and Insert tools; Array in line and Array radial; Put in folder and Isolate folder; Trackball rotation; a reference image with opacity; a Turntable download.

## 0.10.0 · 2026-10-11

### Added

- Brushes: `relax` (evens triangles without shrinking), `planar`, `trim`, `snake_hook` and `noise`; the lazy stabilizer is now a rope, with a Lazy mouse slider in the Brush panel.
- `surface_noise`: fractal noise over the whole object in a new sculpt layer, adjustable by its strength.
- Masks: `mask_shape` (box, ellipse, lasso, in get_view picture coordinates or the window's camera), `mask_topology` (by distance along the surface), `mask_cavity` (creases or peaks), `mask_by_color`, `mask_grow` (and shrink); Box mask, Lasso mask and Topo mask tools and mask buttons in the window.
- `set_visibility` (hide, show, isolate, show all), `arrange_objects` (align and distribute, levels included), `pick`, `get_history` and `history_go_to` (65 commands in all); `get_scene` reports masked vertices.
- Window: a View section (field of view, Hide, Isolate, Show all), pivot on a double-clicked point (with Orbit, or Alt), Align and Distribute, a Noise layer button and a History list that jumps to any step.

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
