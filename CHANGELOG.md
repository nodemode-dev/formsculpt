# Changelog

Versions follow `v*.*.*` with dots. Dates are UTC.

## 1.1.0 · 2026-10-19

### Added

- From FormSurface in File: after painting in FormSurface and exporting with its Blender template, the colour, roughness and metalness come back into the object's paint here.
- 1360 interface strings in every language.

### Changed

- Send with GoF for FormSurface needs UVs (FormSurface skips a mesh without them); run UV unwrap first.
- GoF file names keep letters and digits only, as the Substance package does, so maps exported back are found.

## 1.0.0 · 2026-10-19

### Added

- GoF in Blender: the formsculptor add-on's tab has Import from GoF, Watch GoF and Send back to GoF. What formsculptor sends to Blender with GoF comes in through Blender's own glTF importer (a re-send updates the same object), and an edit sent back returns to the same object in formsculptor. The add-on finds the GoF folder by asking formsculptor, or takes one in its preferences. Get the updated add-on from File > Blender add-on.
- 1355 interface strings in every language.

### Fixed

- Send with GoF for FormSurface while FormSurface is closed no longer shows as an error: the file waits in the GoF folder and the note says so.

### Notes

- formsculptor 1.0.0. Receivers for Unreal Engine and Unity, and FormSurface's painted maps coming back, are next.

## 0.99.0 · 2026-10-19

### Added

- GoF, formsculptor's own hand-off to other apps. File's GoF target and Send with GoF (also in the File menu), and the `gof_send` tool for agents: the object goes into one GoF folder (Formsculptor/GoF in the home folder, or `FORMSCULPTOR_GOF`) as GLB for Blender, Unreal Engine and FormSurface, FBX for Unity, a Substance 3D Painter package, or OBJ for any other app, listed in the folder's `gof.json` manifest. When the other app saves over the file, the edit comes back into the same object.
- FormSurface gets the file straight into its meshes folder, and loaded in its editor when Agent link is on there. `GET /api/gof` shows the folder, the manifest and whether FormSurface is running.
- 1354 interface strings in every language.

### Notes

- Add-ons that pick GoF files up by themselves in Unreal Engine and Unity are not built yet; for now open the file from the GoF folder there. The Blender live link from 0.91.0 still works beside GoF.

## 0.98.0 · 2026-10-19

### Added

- Nomad's one-tap hide shortcuts, as One tap hide shortcuts in the panel's View section (off at first): with Box mask or Lasso mask set to Hide, tap a face group to hide it, or tap empty space to invert what is hidden. Drawn shapes still hide as before. With the option on, the Hide group tool's empty-space tap inverts too.
- 1342 interface strings in every language.

## 0.97.0 · 2026-10-18

### Added

- Nomad's one-tap mask shortcuts, as One tap mask shortcuts in the panel's View section (off at first): with the Mask tool in hand, tap the background to invert the mask, a masked area to blur it, an unmasked area to sharpen it. A drag still paints the mask.
- `pick` also reports the mask at the hit point.
- 1341 interface strings in every language.

### Notes

- Nomad's Hide tool one-tap shortcuts (tap a face group to hide it, empty space to invert) are not built yet.

## 0.96.0 · 2026-10-18

### Added

- The tablet layout's view cube works as Nomad Sculpt's: drag it to turn the view (a tap still jumps to that side), and beside it Frame active object and a new Home view, which goes back to the opening view framing everything shown. Home View is also a command.
- Nomad's gesture options, in the panel's View section:
  - Finger always smooths: a stroke made with a finger smooths, whatever the tool; a pen or mouse keeps the tool.
  - Three fingers up and down, and side to side: drag with three fingers to change the tool radius, the tool intensity, or turn the lighting. Choosing the same for both turns the other off.
- 1340 interface strings in every language.

### Notes

- Nomad's one-tap mask shortcuts are not built yet.

## 0.95.0 · 2026-10-18

### Added

- Textures in the ZBrush layout: a Texture swatch on the left shelf (between Alpha and Material, as in ZBrush) and F5 open a pop-up of the images held in Spotlight, with Load image first; click one to make it the active image. LightBox has a Texture shelf too (double-click to make one active, or Load image).
- 1332 interface strings in every language.

### Changed

- In the ZBrush layout, Z Intensity now stays the sculpting strength while the Paint brush is in hand, so Z Intensity and Rgb Intensity are kept apart as in ZBrush.

## 0.94.0 · 2026-10-18

### Added

- LightBox in the ZBrush layout, as ZBrush has it: press comma (or the LightBox button on the top shelf) and a strip of thumbnails opens over the bottom of the canvas. Click a thumbnail to select it, double-click (or press Enter) to load it; Escape closes it.
  - Projects: type a name and Save to LightBox to keep the scene with a picture of it; double-click one to open it. They are kept in your formsculptor folder, under LightBox/Project.
  - Tool (every primitive, each with its own drawing), Brush, Stroke, Alpha and Material.
- 1330 interface strings in every language.

### Notes

- Projects need the engine service, so they are not on the browser-only build.

## 0.93.0 · 2026-10-18

### Added

- Focal Shift, as ZBrush has it, for every brush: below 0 the brush keeps full strength further out (-100 is a crisp stamp with no falloff), above 0 its strength fades sooner toward a point. It works over any falloff, including a drawn curve. In the panel it sits under Falloff.
- A separate strength for the Paint brush (ZBrush's Rgb Intensity), so painting and sculpting each keep their own. In the panel and tablet layouts the usual Strength slider sets the Paint brush's strength while it is in hand.
- The ZBrush layout's top shelf and QuickMenu now have all four of ZBrush's sliders, Draw Size, Focal Shift, Z Intensity and Rgb Intensity, with O and I opening Focal Shift and Rgb Intensity at the cursor beside S and U.
- For agents: `sculpt_stroke` takes `focal_shift`.
- 1320 interface strings in every language.

### Notes

- In the ZBrush layout, while the Paint brush is in hand, Z Intensity shows the paint strength too.

## 0.92.0 · 2026-10-18

### Added

- Import and export options in the File panel, used both ways:
  - File scale: multiply the size (for example 100 to go from metres to centimetres).
  - File up axis: Z up for apps such as Blender, so a model leaves standing up there and arrives standing up here. It applies to OBJ, STL, PLY and 3MF on export; GLB and glTF are always Y up, and FBX and USD say Y up in the file for the other app to follow.
  - Export triangles: write triangles where OBJ would keep quads.
  - Import as one object: join every part of a file into one object.
- For agents: `import_mesh` takes `scale`, `up_axis` and `merge`; `export_mesh` takes `scale`, `up_axis` and `triangulate`; both say what they applied.
- 1316 interface strings in every language.

### Notes

- The options also work with the browser engine.
- GoZ for ZBrush is still waiting: Maxon's GoZ SDK documentation gives no licence, and its downloads page could not be read from here.

## 0.91.0 · 2026-10-18

### Added

- A live two-way link with Blender. File > Blender add-on saves a Blender extension; install it in Blender (Preferences, Extensions, Install from Disk) and the sidebar's formsculptor tab can:
  - Send to formsculptor (the selected meshes) and Get from formsculptor (the active object), linking each.
  - Live: keep linked objects in step on a timer. Edit or move the object in Blender and it changes here; sculpt it here and it changes in Blender. Undo here comes back too.
  - When only the shape changes, only positions travel, so Blender keeps its quads and UVs and formsculptor keeps its paint, mask and face groups.
- For agents and other apps: `link_pull` (an object's mesh as base64 arrays with its revision, or just "unchanged") and `link_push` (write one back, or make a new object).
- 1310 interface strings in every language.

### Notes

- Tested with Blender 5.2.2. No file sits between the two apps; the file bridge (Bridge to App) still works for every other program.
- GoZ for ZBrush is next: Maxon publishes a GoZ SDK, and its terms are read before anything is built on it.

## 0.90.0 · 2026-10-18

### Added

- Real-world scale for printing. In the File panel, type how big the active object really is (Real size, along its longest side, height, width or depth) and Set real size. Nothing moves; from then on Print check measures at that size and STL and 3MF exports are written in millimetres at it. Leave the field empty to clear it. Saved in projects.
- Render size for Path render: square 768 (as before), 1080p, 1440p, 4K UHD, or square 2048 and 4096. Larger pictures use fewer samples and the denoiser, so 4K takes about a minute; the download is named for its size.
- For agents: `set_real_size`; `get_view` renders up to 4096 a side; `get_scene` reports `mm_per_unit`.
- 1305 interface strings in every language.

### Notes

- Without a real size, exports are in scene units as before.
- The live two-way link with Blender and a GoZ-style socket, the other half of this part of the plan, comes next, researched first.

## 0.89.0 · 2026-10-18

### Added

- A Triplanar primitive (Add > Triplanar), as Nomad Sculpt has: three crossing planes (front, side and top), each starting with a masked disc.
  - Mask a shape on each plane with the Mask tool (picked for you when you add one), then Object > Validate Triplanar. The solid is everything that falls inside the masked part of all three planes, with rounded edges, as a watertight quad mesh. Undo brings the planes back.
  - The planes can be moved, turned and scaled before validating; a triplanar moves whole (its mask is its shape, not a part to hold still).
  - For agents: `add_primitive` with kind `triplanar`, then `triplanar_solid` (with `keep_planes` to add the solid beside the planes as a preview).
- 1288 interface strings in every language.

### Notes

- The solid updates when you validate, not live while you paint.
- A mask stroke near where two planes cross masks both of them.

## 0.88.0 · 2026-10-18

### Added

- A Head primitive (Add > Head), as Nomad Sculpt has: a simple base head with ears and a neck, on even quads, facing front.
  - It comes with a sculpt layer named Feminine at 0 percent. Raise it in the Layers panel to blend from a masculine head (heavier brow, jaw and neck) to a feminine one. Strokes do not record into it.
  - For agents: `add_primitive` with kind `head`, `size` and `detail` (quads a side, 40 by default); `layer_set` on layer 0 blends it.
- 1284 interface strings in every language.

### Notes

- The head is a base to sculpt from, not a finished likeness. The bottom rim of the neck is still a little uneven.

## 0.87.0 · 2026-10-18

### Added

- Two primitives, as Nomad Sculpt has them:
  - Quad Sphere: a cube of quads pushed onto a sphere, with no poles and quads of nearly even size, so it sculpts evenly all over.
  - Icosahedron: twenty flat faces, split into more with Detail. Its edges are creases, so subdivision keeps them sharp, and each face is a face group. For agents, `project` rounds it into a sphere.
- Quad Sphere, UV Sphere (which only agents could add before) and Icosahedron in the Add panel and the Add menu. The Add panel now has two columns, so long names fit.
- 1282 interface strings in every language.

### Fixed

- A UV sphere is named UV sphere, not Uv sphere.

### Notes

- Like every mesh here, the icosahedron shades smoothly across its edges; its outline and creases are flat.

## 0.86.0 · 2026-10-18

### Added

- A scene hierarchy, as in Nomad Sculpt. An object can hang under another object or under a group, an empty node for holding things together.
  - Moving, turning or scaling a parent moves everything under it with it; hiding or showing it does the same.
  - In the object list (in every layout), rows are indented under their parent. A group row folds when clicked, its eye hides everything under it, and its cross deletes it (what was under it moves up).
  - Drag a row onto another row to hang it there, or to the top or bottom edge of a row to put it beside that row. With a mouse the drag starts as you move; on a touch screen, hold the row first.
  - Group (Mod+G) puts the active object in a new group. The row menu adds Delete With Children and Clear Parent.
  - Deleting an object moves what was under it up to its parent; Delete With Children deletes them too.
  - Undo takes back each of these as one step, and projects save the hierarchy (older projects open flat).
- For agents: `add_group`, `set_parent` and `delete_group`; `transform_object` and `set_visibility` take group ids and carry children; `delete_object` takes `children`; `get_scene` reports each object's `parent` and the `groups`.
- 1276 interface strings in every language.

### Changed

- `delete_object` now replies with a list of the removed ids.

## 0.85.0 · 2026-10-18

### Added

- Bounce light: Light > Bounce light. Light bounces once between surfaces in the viewport, so a coloured object tints what is near it and a bright floor lifts an object's shadowed underside. It works in every shading and stops at outlines.
- Bounce strength (0 to 400 percent, 200 by default). 100 percent is the bounce as measured, which is faint; more exaggerates the colour bleeding.
- 1269 interface strings in every language.

### Notes

- Like reflections, bounce light only knows what the viewport shows: light from something off screen or hidden behind another object is missed.

## 0.84.0 · 2026-10-18

### Added

- A denoiser for the path render. It smooths the noise while keeping outlines, creases and painted detail sharp, because it follows each pixel's surface (its direction, distance and colour) and how noisy that pixel's own samples were. Away from edges, 32 samples denoised are about as clean as 128 without it.
  - The window's Path Render uses it, at the same 64 samples and the same time as before.
  - For agents: `get_view` takes `denoise` (off by default, the beauty pass only).

### Fixed

- In the browser engine, the view route passed `aperture` as text, so depth of field was ignored there.

### Notes

- Outlines where two surfaces meet keep some noise: a pixel there sees both, and the filter leaves it as traced rather than blur across the edge.
- Screen-space global illumination in the window moves to the next release.

## 0.83.0 · 2026-10-18

### Changed

- The ZBrush layout now behaves like ZBrush, not just looks like it:
  - Holding a key during a stroke swaps the tool for that stroke: Shift smooths, Ctrl masks, Alt inverts the brush (Zsub while Zadd is on). The tool comes back when you let go.
  - Alt+click on an object selects it. Alt+drag on the background moves the view; let go of Alt mid-drag and it zooms. Shift+drag on the background turns the view and snaps it to the nearest side view when you let go.
  - Ctrl+click on the background inverts the mask, Ctrl+drag on the background clears it, and Ctrl+click on the model blurs the mask.
  - The left shelf shows the current stroke, alpha and material as thumbnails. Each opens a pop-up of thumbnails, also from F3, F4 and F6 (F2 for the brush).
  - B opens the brush pop-up; the next two letters pick a brush by its short code, as in ZBrush (B, S, M for Smooth).
  - Space opens the QuickMenu at the cursor: the four thumbnails, Draw Size, Z Intensity, Zadd and Zsub. [ and ] change Draw Size in steps.
  - The SubTool list keeps a fixed place in the tray, so the window no longer shifts as it opens.
- The Nomad-style layout gains Nomad's own controls:
  - Sub in the side bar, beside Sym, inverts the brush.
  - A Face Group button in the quick bar.
  - K opens the search.
  - On a touch screen, a two-finger tap undoes and a three-finger tap redoes.
- The window opens directly in the chosen layout, without redrawing after the first frame.
- Thumbnails are drawn by formsculptor itself: alpha thumbnails come from the same code as the engine's alphas, and material spheres use the viewport's matcap shading (lit materials are marked as an approximation).
- 1267 interface strings in every language.

### Notes

- Still left out because formsculptor has no counterpart yet, or the behaviour could not be confirmed from two sources: LightBox, Focal Shift, RGB Intensity, the F5 texture pop-up, Nomad's gesture options (the finger that always smooths, the one-tap mask shortcuts, the three-finger radius and intensity drag) and dragging the navigation cube.

## 0.82.0 · 2026-10-18

### Added

- A ZBrush layout, laid out like ZBrush's classic interface: a title bar, the palette menu bar in alphabetical order, top, left and right shelves, and a right tray. Turn it on with View > ZBrush Layout or the layout picker in its title bar.
  - Top shelf: Draw, Move, Scale and Rotate; Mrgb and Rgb; Zadd and Zsub; Draw Size and Z Intensity.
  - Left shelf: the brush (opens a picker of every tool), stroke, alpha, material and colour.
  - Right shelf: Persp, Floor, Sym, Frame, the Move, Zoom3D and Rotate buttons that move the view while you drag them, PolyF, Transp and Solo.
  - The tray shows one palette at a time: Tool by default, with the SubTool list (objects, layers and views) at its top. The divider or Tab opens and closes it.
  - ZBrush's keys while it is on: Q W E R, S and U (a slider at the cursor), X, F, P, Shift+P, Shift+F, B, N, Tab, D and Shift+D, Ctrl+D, Ctrl+I, Shift+Ctrl+A.
- Set as default (in the ZBrush layout's title bar) or View > Open in This Layout by Default (in any layout) chooses which layout the window opens in.
- 1263 interface strings in every language.

### Notes

- Left out because formsculptor has no counterpart yet: LightBox, Projection Master, Quicksketch, Focal Shift, RGB Intensity, M, BPR, the 2.5D canvas buttons, docking several palettes, and building a custom UI.

## 0.81.0 · 2026-10-17

### Added

- Screen-space reflections: Light > Reflections. In lit shading, surfaces mirror what the viewport shows, each by how reflective it is. Smooth metal reflects strongly, polished plastic mostly at grazing angles, and rough or matcap surfaces not at all. Reflections of things outside the view, or hidden behind other things, fade out, as with any screen-space technique.
- 1220 interface strings in every language.

## 0.80.0 · 2026-10-17

### Added

- Material types. Paint > Material type sets how the active object covers what is behind it, saved with the object:
  - Opaque: as before.
  - Dithered: see-through by a fine pattern at Material opacity. It needs no sorting, keeps depth and casts a lighter shadow.
  - Additive: adds its light over what is behind it at Material opacity and hides nothing, for glows, holograms and light beams. It casts no shadow.
  - Refraction: glass and water, as Transmission (it turns on lit shading).
- Dithered and Additive work in every shading.
- The path render now refracts transmissive objects (Fresnel reflection, total internal reflection, a tint by colour). It also follows dithered and additive materials. Shadows pass through dithered, additive and glass surfaces as a lighter shadow.
- For agents: `set_material` takes `blend` (opaque, dithered, additive) and `opacity`, and `get_scene` reports them.

### Fixed

- Path render: looking through see-through paint now shows the backdrop behind it, not the sky's bounce light.

### Notes

- 1219 interface strings in every language.

## 0.79.0 · 2026-10-17

### Added

- Colour temperature for every light: Light > Colour temperature (K) runs from 1700 (candle) through 2700 to 3200 (tungsten) and 6500 (white) to 12000 (blue sky), and sets the light's colour to match. Picking a colour afterwards replaces it.
- Spot lights. An added light's Light type can be Spot: it shines from its direction toward the scene's centre in a cone. Spot distance sets how far out it sits, Cone angle how wide it is, and Cone softness how much its edge fades. It is at full strength on the scene's centre, so moving it out widens the pool without dimming the model. The key light stays directional, since it casts the shadows.
- For agents: `get_view`'s path render lights take `kind` (`directional` or `spot`) and `kelvin`. A spot takes `at`, `to` (the framed centre by default), `cone` and `softness`, and casts soft shadows. `kelvin` multiplies `color`, so the two act as a gel. Lights in the earlier form render as before.
- 1213 interface strings in every language.

## 0.78.0 · 2026-10-17

### Added

- Two new tone maps for Lit (PBR) shading. AgX takes bright, saturated light toward white the way film does, so hot colours keep their hue. Neutral leaves base colours almost exact and softens only the highlights. Find them in Light > Tone mapping, beside Reinhard, ACES filmic and None.
- Colour grading after Tone mapping: Contrast, Saturation and White balance (cooler to warmer). The defaults change nothing, and the environment backdrop is graded with the model.
- For agents: `get_view`'s path render takes `tone` (reinhard, aces, agx, neutral, none), `exposure`, `contrast`, `saturation` and `temperature`. It runs the same curves as the window, so a render matches the viewport.
- 1206 interface strings in every language.

## 0.77.0 · 2026-10-17

### Added

- Saved camera views: a Views tab in the Objects and Layers panel lists them; click one to move there, the cross deletes it, Save view keeps the current view under a name. In the panel layout, Save view, Previous view and Next view sit in the View section, and the View menu has a Saved Views submenu. Views are saved in projects.
- Fly navigation (View > Fly Navigation, Shift+F): walk with W A S D or the arrow keys, Q and E to go down and up, Shift to go faster and Alt slower, drag to look around; Escape leaves. A badge says it is on, and a drag never sculpts while it is.
- For agents: `save_view`, `list_views` and `delete_view`; `get_scene` lists the views' names; `get_view` takes `saved` to look from one.
- 1202 interface strings in every language.

## 0.76.0 · 2026-10-17

### Added

- An Objects and Layers panel beside the left strip, with two tabs.
  - Objects: every object in the scene with its vertex count and the active one marked. Click a row to make it active; its eye shows or hides it, its lock keeps strokes off it. Add Sphere, Duplicate, Rename and Delete below.
  - Layers: the active object's base mesh and sculpt layers. Click a row to choose where strokes go; a layer's eye hides it and shows it again at the strength it had; drag its bar to set its strength (0 to 200 percent). Add layer, Apply and Delete below.
- Every button in the two top bars has a word under its icon (File, Scene, Topology, Paint, Light, View, History, More; Tool, Mirror, Dyntopo, Invert, Search, Layout, All tools).
- The window's notes (what a click did, or why it did nothing) show over the viewport for a few seconds instead of in the hidden side panel.
- 1182 interface strings in every language.

### Fixed

- The terminal is no longer filled with a line for every request the window makes. Failed requests and errors still print; FSC_ACCESS_LOG=1 prints everything again.

## 0.75.1 · 2026-10-17

### Fixed

- The interface rebuilt to the owner's recording (black viewport, corner bars, coloured tool palette, side sliders, options card, quick bar) is now what opens. It had stayed behind Window > Tablet Layout, so a first start showed the older panel layout. A browser that had it switched off from an earlier version opens it once; after that your choice holds.
- The panel layout's tool rail is no longer an empty strip on a first start: it holds Draw, Clay, Smooth, Grab, Inflate, Crease, Flatten, Pinch, Mask and Transform. Right-click any panel button to add it, or a rail button to take it off.

## 0.75.0 · 2026-10-17

### Added

- Gizmo mode (Object section, the tablet's Transform options, Edit > Transform): all handles, or only the move arrows, the turn rings or the scale squares, so the one you want is easy to grab.
- Pin gizmo: the gizmo stays at its point in the world while the object moves under it, and turns and scales about that point. Pivot to view pins it at the surface in the middle of the view.
- Modifier gestures on the gizmo: Alt-drag an arrow or the centre to move the gizmo itself (the object stays put); Ctrl or Cmd for fine control (a tenth of the travel); Shift to snap to 15 degrees and 0.1 when the snap settings are off. The amount shows while dragging.
- Typed values: Move by, Turn by (degrees) and Scale by for x, y and z, applied about the gizmo's pivot as one undo step (Apply transform, or Enter in a field).
- 1173 interface strings in every language.

### Changed

- Alt and Shift presses on a gizmo handle now go to the gizmo; elsewhere in the viewport they still turn and pan the view.

## 0.74.0 · 2026-10-17

### Added

- Darken others (View section, or View > Darken Others): every object but the active one is drawn darker (0 to 90 percent), so the one you are working on stands out.
- Wire on: the wireframe on all objects or only on the active one (View > Wire on Active Object Only).
- Statistics overlay (View > Statistics Overlay): a readout over the viewport's corner with the visible objects and their vertex and triangle counts, the active object's counts and level, and the frame time. The tablet layout keeps its own counts and does not show it.
- These settings are kept in this browser and travel in layout files.
- 1149 interface strings in every language.

## 0.73.0 · 2026-10-17

### Added

- History replay (Edit > Replay History, or Replay in the History section): plays the undo history from the start to the current step at a chosen speed, then leaves it where it was. Stop, Escape or a press in the viewport ends it early, back at the current step, without sculpting.
- Camera per step: each undo step keeps the view it was made from. Choosing a step in the history moves the view there, replays follow it, and the History movie shows each step from where it was made.
- Each step in the history shows the memory it keeps, with the total under the History settings.
- Max steps (1 to 10000, 200 by default) and Memory limit (MB): the oldest steps are dropped beyond either; the newest step is always kept. Kept in this browser.
- For agents: `set_history_limits`, `set_history_camera`; `get_history` reports each step's memory and camera; `history_go_to` returns the step's camera; `record_history` takes `camera: "steps"`.
- 1139 interface strings in every language.

### Fixed

- No console error when the window asks for an object that was removed a moment earlier (the service now answers 204 rather than 404).

## 0.72.0 · 2026-10-17

### Added

- Themes (Window > Theme, or Theme in the View section): Follow NodeMode (as before), Dark theme, Light theme and High contrast (black grounds, white text, strong borders and focus rings). The choice is applied before the window first paints and travels in layout files.
- Translucent panels: the viewport runs under the panel and the tool rail, and the panel, rail, menus, dialogs and floating toolbars are see-through with a blur. The tablet layout's floating panels follow the same switch.
- Tool buttons in the panel carry a colour stripe by family (sculpt blue, mask grey, paint green, cut red, build yellow, move purple, view cyan), the same colours as the tablet palette.
- 1123 interface strings in every language.

### Fixed

- In the tablet layout on shorter screens the options card no longer runs into the quick bar; it and the tool pill sit above the quick bar's row.

## 0.71.0 · 2026-10-17

### Added

- Pen and gamepad bindings with tap against hold (Window > Input Bindings): each input runs one command when tapped and another when held. A held tool is a temporary tool, back to the previous tool on release; an input with nothing bound to hold taps without waiting.
  - Pen: the barrel button held while drawing sculpts with its hold tool (Smooth by default) instead of panning; clicked while hovering it taps (Undo by default). The eraser end draws with its own tool (Mask by default).
  - Gamepads: all 16 buttons of a standard gamepad can be bound (defaults include undo and redo, brush size on the bumpers, strength on the D-pad, Smooth and Mask held on the triggers). The left stick turns the camera; the right stick pans and zooms.
  - Bindings are kept in this browser and travel in layout files.
- Larger Brush and Smaller Brush (] and [), Stronger Brush and Weaker Brush.
- 1117 interface strings in every language.

## 0.70.0 · 2026-10-17

### Changed

- The tablet layout (Window > Tablet Layout) is rebuilt to the arrangement of the owner's recording:
  - The viewport fills the window on black, with no menu bar or status bar.
  - Top left: a floating bar opens the scene menus (File, Scene, Topology, Paint and material, Lighting, View, History, More), with the scene's counts and the active object under it.
  - Top right: a bar with tool settings, Mirror X, dynamic topology, invert, the command palette, the way back to the panel layout and the tool grid toggle. Under it sit Orthographic, Full screen, Grid and Frame, and a view cube that turns with the camera and snaps to a face.
  - Right edge: the tool palette, every tool as a coloured disc by family over its name, in a 4-column grid or the usual tools in one column.
  - Left edge: a strip with Sym, quick Smooth, Mask and Transform swaps, and tall Radius and Intensity bars.
  - Bottom: the tool's pill and an options card (icon-over-label toggles, choices showing their value, values as filled bars), and a quick bar with Undo and Redo and their step counts, Solo, X-ray, Voxel, Grid, Wireframe, Inspector and History.
  - Menus and the Voxel panel (Resolution, Thicken thin parts, Remesh) open as floating panels with a title bar.
- While the tablet layout is on, objects are drawn in the new Orange clay shading on black, every part in the same clay, with the floor grid off. Leaving the layout gives the previous look back.

### Added

- Orange clay in View > Shading.
- 1087 interface strings in every language.

## 0.69.1 · 2026-10-17

Fixes found while Claude Code drove the window over MCP to rebuild the snake head from the owner's Nomad Sculpt recording.

### Fixed

- Voxel remesh unions parts that overlap inside one object (primitives merged before a remesh), as DynaMesh and Nomad do: three overlapping primitives came out with 3200 open edges, 979 non-manifold edges and 4 loose pieces, and now come out as one watertight piece.
- Thicken thin parts no longer leaves rings on curved surfaces (a sphere's surface strayed 70 times more at 50 percent); thin parts still survive. Remeshed vertices now sit on the surface.
- Smooth works on dense meshes: one stroke on a 381k-vertex block rounds an edge 11 times as far as before, in 0.14 s.
- Crease and pinch pull across the stroke and in proportion to spacing: strong, finely spaced strokes along a curve left a torn, hatched groove and now leave one clean fold.
- boolean keeps the name of the object it replaces (cutting nostrils into "Viper head" left it named "Viper head difference").
- Commands accept numbers and true or false sent as text where the argument is a number or a switch ("strength": "1.0" was refused).
- A stroke that changes nothing says which object it went to and that its points missed the surface; edit_build with nothing to change refuses instead of reporting success.
- The modeling skill tells agents to take stroke points from pick on a sculpted surface, and why repeated points give one dab.

### Known

- bevel_edges with concave or both leaves radial streaks in the fillet where a round part meets a flat face.

## 0.69.0 · 2026-10-17

### Added

- Floating toolbars (Window > New Toolbar): named strips of command buttons over the viewport. Add a button from any panel button's right-click menu (Add to Toolbar) or by name from the toolbar's grip menu (Add Command); move a toolbar by its grip, turn it to run down instead of across, rename or delete it. Buttons run their command and show its pressed and disabled state.
- Layout files (Window > Export Layout, Import Layout): the panel's folds, side, size and rail, the tablet layout switch, the floating toolbars, shortcut overrides and the second-press setting in one JSON file; an import applies at once and a damaged file changes nothing.
- Second Press Swaps Tool (Window menu, and in View): choosing the tool already in hand returns to the one before it. Off by default.
- 1074 interface strings in every language.

## 0.68.0 · 2026-10-17

### Added

- `transform_object` takes `pivot`: turn and scale about that point instead of the bounds centre.
- `remesh_voxel` takes `thicken` (0 to 100): thin sheets and strands survive the remesh as closed parts, the rest of the surface moving out by at most half a voxel.
- Window: the transform gizmo turns and scales about the bounds centre, the world origin or a point picked on the surface (Pick pivot), and snaps turns (5, 15, 45 or 90 degrees) and moves (0.01 to 1); these four sit in Object and in the tablet layout's Transform strip.
- Window: each row of the scene list has a visibility eye and a lock (a locked object takes no strokes or tool clicks in this window).
- Window: Thicken thin parts % beside Voxel resolution.
- 1051 interface strings in every language.

## 0.67.0 · 2026-10-17

### Added

- Tablet layout (Window > Tablet Layout): the viewport fills the window; a mini rail of tool icons runs down the left with tall Radius and Intensity sliders beside it; the active tool's name and its options (sculpt toggles, tube and curve settings, paint channels, mask actions, ZModeler settings) float at the bottom; a quick bar holds Undo, Redo, Voxel remesh, Wireframe, X-ray, Floor grid and Solo; icons at the top open the panel's sections as a floating sheet (File, Scene, Tool settings, Topology, Paint and material, Lighting, View, More); Frame, Orthographic and the command palette sit top right. Every control runs the same command or panel control as the panel layout, and the choice is remembered.
- Wireframe (Shift+W): every object's edges over it, quad edges on quad meshes. X-ray (Alt+X): every object drawn see-through. Both in View, the View menu and the quick bar.
- 1038 interface strings in every language.

## 0.66.0 · 2026-10-17

### Added

- `retopo_patch` template `skeleton`: for an object made by `skeleton_mesh` (a figure, animal or hand), all quads over the whole object with edge loops ringing each limb and running along it, from a field remesh held along every bone away from the joints; `faces` sets the count. `field_remesh_steered` in the engine takes the bones.
- Window: Skeleton (whole object) in the Retopo patch list, sized by Target quads.
- 1028 interface strings in every language.

### Changed

- `retopo_patch` finds weld partners through a spatial grid, so large templates lay in linear time.

## 0.65.0 · 2026-10-17

### Added

- Whole-object retopology templates in `retopo_patch`: `sphere` (a closed quad sphere) and `head` (with two eye loops and a mouth loop on the side facing `front`), scaled to the sculpt along up and front, wrapped onto its outer surface and evened out along it; the reply counts `folded` quads where the shape is not seen whole from its middle.
- Window: Head (whole object) and Sphere (whole object) in the Retopo patch list, placed by a click with the face toward the camera; each patch kind sets its own counts when chosen.
- 1027 interface strings in every language.

### Fixed

- The retopology wire and the crease path now appear as soon as their edges arrive, not only at the next camera move.

## 0.64.0 · 2026-10-17

### Added

- ZModeler edge inset (`zmodel_edge` action `inset`): supporting loops on both sides of an edge loop, the same distance from it all along, keeping the loop (`keep` center_and_border) or replacing it (border_only); optionally creased.
- ZModeler point split (`zmodel_point` action `split`): a circular inset round a corner that keeps quads at its centre, on a circle (`equalize`) or along the diagonals, optionally creased.
- ZModeler point stitch (`zmodel_point` action `stitch`): welds a corner to the corner nearest `to`, at the second point, halfway or at the first (`align`); a quad between them collapses, and a weld across one edge is refused since it would leave triangles.
- Window: Inset edge loop, Inset edge loop, border only, Split point and Stitch points (two clicks) in ZModeler; its amount slider, renamed Amount %, also sets the inset's spacing and the split's radius.
- 1024 interface strings in every language.

## 0.63.0 · 2026-10-17

### Added

- `field_remesh` takes `guides` (curves on the surface, as for `quad_remesh`): edge loops run along each one, and the field blends between the guides and the hard edges elsewhere. The reply counts `guided_vertices`.
- Window: Curve as guide, which sends the curve waiting from the Curve stroke tool to Quad remesh and Field remesh.
- 1019 interface strings in every language.

### Fixed

- `field_remesh` no longer leaves open edges at sharp rims (cylinders, cones): faces wound against their neighbours are made consistent or dropped and their holes closed, and pinched gaps are filled. A guide with fewer than two points is refused.

## 0.62.0 · 2026-10-17

### Added

- `field_remesh`: an all-quad remesh whose edge loops follow the form, after Instant Meshes. A smoothed four-way direction field runs along the curvature and along hard edges (`hard_angle`, 40 degrees by default), a position field lays a lattice on it, and the lattice is read off as quads near a target count (`faces`); `density_from_paint` makes red areas finer and blue coarser. Gaps near the field's singular points are closed on watertight objects; the reply counts any left (`open_edges`). One undo step; paint is carried over.
- Window: Field remesh in Topology, with Hard angle and Density from paint (it shares Target quads with Quad remesh).
- 167 engine commands; 1017 interface strings in every language.

### Changed

- `border_loops` walks a pinched border the same way every run.

## 0.61.0 · 2026-10-17

### Added

- Dynamic topology modes on `sculpt_stroke` and live strokes: `dyntopo_mode` (speed, balanced as before, quality), `dyntopo_method` (subdivide only, collapse only, both) and `dyntopo_protect_mask` (masked areas keep their triangles).
- Window: Dyntopo mode, Dyntopo method and Protect masked beside Dyntopo.
- 1013 interface strings in every language.

## 0.60.0 · 2026-10-17

### Added

- `decimate` takes `preserve_paint` (paint carried exactly, colour boundaries collapse last), `preserve_groups` (face groups and their borders kept) and `preserve_uvs` (UVs and a painted texture kept, seam vertices held) and reports the triangles reached; hidden faces now come through decimation.
- Window: Keep paint, Keep groups and Keep UVs for Decimate, on by default.
- 1004 interface strings in every language.

## 0.59.0 · 2026-10-16

### Added

- Retopology patch library: `retopo_patch` lays a grid, strip, eye loop, mouth loop or disc (a quad fan whose centre can be a five-edge pole) onto the sculpt at a point, upright along a given direction, welded into the same retopology object as `retopo_face`. `get_wire`: an object's quad edges for drawing, with their visibility behind another object.
- Window: a Retopo patch tool (Retopo patch, Patch size %, Patch columns, Patch rows), and the retopology object's wire drawn over the sculpt while Retopo quad or Retopo patch is the tool; until now a retopology object could not be seen on the sculpt.
- 166 engine commands; 1001 interface strings in every language.

## 0.58.0 · 2026-10-16

### Added

- `zmodel_edge` align (an open edge loop straightened between its ends, a closed one flattened onto its plane) and slide (the loop moved toward the aimed face's side by amount); both keep UVs. `get_creases`: the creased edges as segments, with their visibility from a camera position.
- Window: creases drawn while ZModeler is the tool, Align edge loop, Slide edge loop with Slide amount %, and Slide point by drag.
- 164 engine commands; 990 interface strings in every language.

## 0.57.0 · 2026-10-16

### Added

- Creased edges: kept sharp by subdivision (`subdivide`, `multires_divide`, the smooth preview), carried through every level, and saved in projects. `zmodel_edge` and `zmodel_point` take `crease` and `uncrease` (an edge, or every edge at a corner); `zmodel_edge` insert and `zmodel_face` inset and extrude take `crease: true` (auto-crease); a loop that splits a creased edge keeps both halves creased; `poly_pick` reports whether the edge is creased.
- Window: Crease and Uncrease for edges and points in ZModeler action, Auto-crease, a corner ring as the hover for point actions and a creased mark on the edge hover.

### Fixed

- The ten ZModeler action labels were not translated (their values held a colon the string extractor skips); they are now in every language. 985 interface strings in every language.

## 0.56.0 · 2026-10-16

### Added

- ZModeler actions on quad meshes, aimed at a picked point: `poly_pick` (the face, edge and corner under a point or ray), `zmodel_edge` (insert an edge loop at any fraction across the ring, extrude an open-border edge, close a hole with quads), `zmodel_face` (inset, extrude, delete) and `zmodel_point` (slide a corner in its tangent plane). Meshes stay all quads.
- Window: a ZModeler click tool with a hover highlight of the edge or face the next click acts on, ZModeler action and Loop position %.
- 163 engine commands; 974 interface strings in every language.

## 0.55.0 · 2026-10-16

### Changed

- Window: a painted texture's roughness and metalness now drive Lit (PBR) shading per texel, and its opacity draws see-through; before, lit shading took them from the vertex paint. Emissive from a texture is not drawn yet.

## 0.54.0 · 2026-10-16

### Added

- Spotlight, after ZBrush: images held over the view with a placement dial. `spotlight_add` (PNG, up to 16 images), `spotlight_set` (centre, size, angle, opacity, tile, flip, active), `spotlight_list`, `spotlight_image`, `spotlight_remove` (one or all) and `spotlight_apply`, which paints the image's colour (into vertex paint or the texture) or sculpts by its brightness through the current camera, onto front-facing, unoccluded surfaces, whole or under one brush dab; one undo step. The images themselves are view state, not undo steps or project data.
- Window: the Spotlight drawn over the viewport, a dial to move, turn and size the active image, Paint through, Sculpt through, a Spotlight brush (paint or sculpt), Spotlight depth %, Spotlight opacity, Flip Spotlight and Clear Spotlight.
- 159 engine commands; 967 interface strings in every language.

## 0.53.0 · 2026-10-16

### Added

- Painting straight into a texture: `texture_create` (blank or baked from vertex paint, 256 to 4096), `texture_clear`, `get_texture`, and `target: "texture"` on `sculpt_stroke` and `paint_fill`, finer than the mesh; GLB export and `bake_maps` use the texture; saved in projects.
- `pick_paint`: the paint at a surface point, from the texture or the vertex paint.
- Window: painted textures drawn in the viewport, Paint into texture, Make texture and Drop texture, an Eyedropper tool and Material presets (clay, gold, silver, copper, chrome, red plastic, rubber, bone, skin, stone).
- 153 engine commands; 948 interface strings in every language.

## 0.52.0 · 2026-10-16

### Added

- Sculpt layers carry paint and mask as well as shape, with per-channel factors: `layer_set` (strength, shape_factor, paint_factor, mask_factor, name), `layer_offset`, `layer_bake` (all three channels into the base) and `layer_extract` (difference, mask or displaced_only); saved in projects.
- Paint layers: 23 blend modes (darken, lighten, colour dodge and burn, linear burn and light, vivid, pin, hard and soft light, difference, exclusion, subtract, divide, hue, saturation, colour, luminosity added to the five) and `channel_blend` per-channel overrides.
- Window: Layer shape %, Layer paint % and Layer mask % under Strength; Offset layer, Bake layer and Extract layer; the full blend list.
- 149 engine commands; 926 interface strings in every language.

## 0.51.0 · 2026-10-16

### Added

- Selection shapes for `mask_shape`, `group_from_outline` and `cut_shape`: polygon, curve (a smooth closed outline), path (a band `stroke_width` wide), line (one side of a line) and `flip` on every shape; `hide_shape` (hide or show by shape) and `split_shape` (faces inside become a new object, closed with `close`).
- `extract_mask` `mode`: shell (as before), balloon, extrude, splat and project; all closed and watertight.
- Window: Selection shape, Selection action (mask, hide, show, new face group, split off), Flip selection and Path width for the Box and Lasso mask tools; Extract as.
- 145 engine commands; 895 interface strings in every language.

## 0.50.0 · 2026-10-16

### Added

- Alpha options in `sculpt_stroke` and live strokes: `alpha_stretch`, `alpha_rotation` (on top of the angle and roll), `alpha_tile`, `alpha_mid` (grey is zero, so draw, stamp, inflate and noise both raise and carve) and `alpha_invert`.
- Window: Alpha stretch, Alpha rotation, Alpha tiles, Alpha mid % and Invert alpha; tool presets (the brush and its settings) saved, chosen and deleted from Tool preset; a brush cursor ring at the brush's real size.
- 877 interface strings in every language.

## 0.49.0 · 2026-10-16

### Added

- Stroke filters in `sculpt_stroke` and live strokes: `front_facing` (with `view_dir`; live strokes use each ray), `connected_only`, `protect_group` and `protect_groups`, `keep_sharp` (degrees), `accumulate` (false caps a stroke at one dab's depth), `area_sample` and `depth` (depth masking for thin parts).
- Window brush fields: Front faces only, Connected only, Same group only, Keep sharp edges, Accumulate, Area sample % and Depth mask.
- 865 interface strings in every language.

## 0.48.0 · 2026-10-16

### Added

- Stroke options in `sculpt_stroke` and live strokes: `spacing_compensation` (the same deposit at any spacing), `offset` (the brush lifted above or into the surface), `jitter` (seeded position, radius and strength variation per dab) and `snap_radius` (radius in fixed steps).
- Window brush fields: Spacing %, Even deposit, Brush offset %, Stroke jitter %, Snap radius, Constant 3D radius (the brush keeps its scene size when zooming) and Shared radius (off: each brush keeps its own radius and strength).
- 858 interface strings in every language.

## 0.47.0 · 2026-10-16

### Added

- Six brushes in `sculpt_stroke` and live strokes: `elastic` (a Kelvinlet grab that reaches past the brush), `polish` (cuts to a fitted plane, keeps creases), `layer` (raises to a `height` cap per stroke), `nudge` (slides along the surface), `stamp` (one dab) and `layer_reset` (erases the active sculpt layer under the brush).
- Window: Elastic, Polish, Layer, Nudge and Layer reset in the brush rail and Sculpt menu, and a Layer height % control.
- 22 brushes; 851 interface strings in every language.

## 0.46.0 · 2026-10-16

### Added

- `repeat_object`: nested repeaters (up to four line, radial, mirror and curve steps, each with seeded randomize) validated as one merged object, separate copies, linked copies or a preview; `scatter_on_surface`: copies spread by area over another object, aligned to the normal or upright, with spacing, scale range, spin, offset and mask-only.
- Build section: Repeater step and Repeater result fields with Add repeater step, Clear steps and Apply repeater; Scatter count and Scatter on surface.
- 143 engine commands; 845 interface strings in every language.

## 0.45.0 · 2026-10-16

### Added

- UDIM in export: a `bake_maps` `udim` bake keeps every tile's maps on the object (saved in projects, no path needed), and GLB and glTF export give a tiled object one primitive and one material per tile, UVs in 0..1. Objects without tile maps export exactly as before.
- `get_uv_layout` `overview` pictures every occupied tile at once.
- UV section: All tiles shows the overview with tile borders, and a click opens a tile; Bake maps bakes every tile when the islands are spread over tiles.
- 141 engine commands; 820 interface strings in every language.

## 0.44.0 · 2026-10-16

### Added

- UDIM tiles: `uv_tiles` (the tiles islands sit in), `uv_to_tile` (move islands by whole tiles), `uv_pack_udim` (one tile per face group, or islands spread over tiles by area, each tile packed); `bake_maps` bakes one `tile` or every occupied tile (`udim`, files named NAME_color.1001.png); `get_uv_layout` pictures any `tile`.
- UV section: a UV tile field, Island to tile, Tiles by group, Tiles by island and List tiles; the UV checker tints each tile past 1001.
- 141 engine commands; 818 interface strings in every language.

## 0.43.0 · 2026-10-16

### Added

- `get_uvs` (an object's per-corner UVs as base64 float32) and `uv_unwrap` `seams: "charts"` with `chart_angle` (automatic disc-shaped charts grown by normal angle, falling back to facing seams when a chart stretches too far).
- View > UV Checker: a 16 by 16 checker drawn from the active object's UVs, red outside the unit square; UV seams gains Automatic charts.
- 138 engine commands; 805 interface strings in every language.

## 0.42.0 · 2026-10-16

### Added

- `uv_pick` (the nearest UV point to a spot in the layout: vertex, UV, island) and `uv_path` (the shortest path along mesh edges between two vertices, kept inside one island when asked).
- UV view: Alt-click pins points for Relax; Ctrl-click points of one island to build a path; Cut along path, Straighten path and Clear points.
- 137 engine commands; 801 interface strings in every language.

## 0.41.0 · 2026-10-16

### Added

- `uv_sew` (move one UV island onto another along their shared edges and weld them), `uv_cut` (cut a UV seam along a vertex path; a closed loop becomes a new island) and `uv_straighten` (lay a vertex chain on a straight UV line or an axis, spaced by 3D length, and relax the rest).
- UV section: Sew to neighbour.
- 135 engine commands; 785 interface strings in every language.

## 0.40.0 · 2026-10-16

### Added

- `uv_density` (texel density per UV island at a texture size, with mean, min, max and spread), `uv_equalize` (every island to one density, repacked, one undo step) and `uv_relax` (conformal or plain relaxing of UVs with borders and chosen vertices pinned, stretch reported before and after).
- UV section: Relax, Even density and Density.
- 132 engine commands; 778 interface strings in every language.

## 0.39.0 · 2026-10-16

### Added

- `curve_bridge`: a loft between two face-set borders (a group's or the mask's) along a path, or (Snake Curve) a tapering extrusion out of one border; a new open tube object in one undo step.
- Live stroke re-edit: Edit Last Stroke as Curve re-applies the stroke as its handles move (Live re-edit, on by default), as one replaced step.
- Snake from mask and Bridge two groups in the panel and the Sculpt menu, with Snake end size %.
- 129 engine commands; 768 interface strings in every language.

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
