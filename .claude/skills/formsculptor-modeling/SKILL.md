---
name: formsculptor-modeling
description: Sculpt and model 3D objects in formsculptor, the 3D sculpting app, by asking in plain language through its MCP server. Use whenever the person wants a 3D model, sculpt, bust, creature, prop or base mesh made or changed, a primitive added, a surface built up, carved, smoothed, flattened, creased, pinched or pulled, a mask painted or cleared, symmetry used, or a step undone, and the formsculptor tools (get_scene, add_primitive, sculpt_stroke and the rest) are available or the person mentions formsculptor. Covers reading the scene, choosing brush radius from object bounds, building strokes as point paths, masking, symmetry and checking work. Also use when a formsculptor tool fails and the next step is unclear. Not for editing formsculptor's own source code (use formsculptor-harness for that).
---

# Sculpting in formsculptor

formsculptor's engine holds a scene of meshes on this computer. The tools change that scene directly; the formsculptor window, when it is open, shows each change live, and the tools work the same with no window at all. Every change is one undoable step.

The full tool reference, generated from the engine's command registry, is `references/tools.md`. Read it when a parameter is unclear.

## Before anything

1. Call `get_scene`. It lists every object with its id, vertex and triangle counts and bounds, and which object is active.
2. If the scene is empty, `add_primitive` first. For sculpting use `kind: "sphere"` with `detail` 6 (81,920 triangles) or 7 (327,680); a low detail gives blocky strokes. Until anything else changes it, `edit_primitive` resizes or re-details it in place, so settle proportions and resolution there before the first stroke.
3. Read the bounds. Brush radius is in object units: about a tenth of the object's largest extent is a good default (0.1 to 0.2 on a unit sphere).

## Building a stroke

`sculpt_stroke` takes `points`, a path of `[x, y, z]` or `[x, y, z, pressure]` positions on or near the surface. Dabs are placed every `radius * spacing` (spacing 0.15 by default) along the path, so a path of a few points spaced across the surface is enough; the engine fills in between.

A point on a sphere of radius r centred at the origin at height y and angle t is `[sqrt(r*r - y*y) * cos(t), y, sqrt(r*r - y*y) * sin(t)]`. Points slightly off the surface still work: the brush acts on the vertices within the radius.

## Workflow 1: block out a form

1. `add_primitive` sphere, detail 6.
2. `sculpt_stroke` with `brush: "grab"`, two points (where to grab, where to move it to), a large radius (0.3 to 0.5) to pull out a large shape such as a snout, a limb or a jaw.
3. `inflate` or `draw` with a large radius and strength 0.3 to 0.5 to add volume; `invert: true` carves instead.
4. `get_scene` to check the bounds moved as intended, then look: `get_view` returns a picture of the scene (clay shading, masks darkened). Take `front` and `right` for proportions and `three_quarter` for form; `frame: "active"` to look closely at one object.

## Workflow 2: refine and detail

1. `clay` builds up layered planes, `flatten` cuts flat planes, `crease` draws a fold line, `pinch` tightens an edge, `smooth` relaxes noise. Smaller radius (0.03 to 0.1) for detail.
   - `relax` evens stretched triangles without shrinking the form (prefer it to `smooth` after grab or snake hook).
   - `planar` flattens to the plane under the stroke's first point (hard-surface faces); `trim` shaves down only what stands above (chiselled planes on rock or armour).
   - `snake_hook` pulls the area under the first point along the path: give points that leave the surface (horns, tentacles, spikes); add `dyntopo` to keep the pulled part detailed, then `relax`.
   - `noise` roughens along a stroke (`noise_scale` sets the feature size); `surface_noise` covers the whole object in a new layer whose strength you can change later, with a `pattern` (value for skin and stone, ridged for rock and bark, turbulence for wrinkles, cells for scales or, with a negative amount, craters, alpha to tile an alpha such as a fabric weave), `octaves`, `gain` and `stretch` (for example [1, 4, 1] for vertical grain).
   - `lazy` (0 to 0.95) steadies a line: the brush trails on a rope and stops short of the path's end by the rope length.
   - `alpha` shapes every dab with a gray stamp: built-ins `square`, `noise`, `cracks`, `rings`, `star`, `stripes`, `dots`, or a PNG loaded with `alpha_load` (white stamps, black does not). One point stamps once (scales, rivets, emblems); a path repeats it. `alpha_angle` turns it in degrees, `alpha_up` sets which way is up (default world +y), and `stroke: "roll"` turns it along the path (stripes along a seam). Check the result with `get_view`; stamps read best at strength 0.3 to 0.6 on a subdivided surface.
   - `stroke: "spray"` scatters dabs up to `spray` radii off the path with random turns and strengths (pores, gravel, speckle; pair it with an alpha); it is seeded, so a repeat gives the same result.
   - `falloff_curve` draws the brush profile as weights from centre to rim: `[1, 1, 1, 0]` a flat-topped hard-edged brush, `[0, 1, 0]` a ring, `[1, 0.2, 0]` a sharp point. Give points a fourth number for pressure; `pressure_gamma`, `pressure_strength` and `pressure_size` shape what it does (tapered lines: pressure falling to 0.1 along the path with `pressure_size`).
   - `paint_project` paints an image: `mode: "view"` from a named view like a projector (only onto what that view sees, so project front and side separately for a face), or `mode: "triplanar"` tiled over everything (fabric, stone). It is vertex paint, so subdivide first for detail.
   - Repeated multi-step builds are better as a Python script (`./formsculptor script file.py`, with `fsc.call("command", **args)`) or a plugin in `plugins/`; suggest one when the person asks for the same sequence again.
   - Paint in layers when a colour pass may change later: `paint_layer_add` (blend normal, multiply, screen, overlay or add, with an opacity) makes a layer that the paint brush, `paint_fill` and smudge paint into; `invert` with the paint brush erases it. Put the base colour on the base, then dirt or cavity tint on a multiply layer and highlights on a screen layer; tune them later with `paint_layer_set` (opacity, blend, visible, move), and `paint_layer_select` with index null to paint the base again. `paint_layer_merge` and `paint_layer_flatten` bake them. Remesh, dyntopo and subdivide set the layers aside (undo brings them back), so finish the shape first.
   - `smudge` drags existing paint colour along the path (blend a paint edge, streak a pattern); it does not move vertices.
   - For broad proportion changes use `lattice_deform` before detailing: move a few control points of a 3 by 3 by 3 lattice (point [i, j, k] from the bounds' min corner) and the whole form follows smoothly; a single point pulls softly, so move a pair or a face of points for a stronger change.
2. Use `symmetry: {"x": true}` for anything bilateral (faces, bodies); `radial: n` repeats a stroke n times around the y axis (petals, gears, crowns).
3. Prefer several light strokes (strength 0.2 to 0.4) to one heavy one.

## Workflow 3: protect an area

1. `sculpt_stroke` with `brush: "mask"` paints the mask over what must not move.
2. Sculpt; masked vertices resist every other brush.
3. `mask_invert` to work on the other side, `mask_blur` to soften the boundary, `mask_grow` (negative steps shrink) to adjust it, `mask_clear` when done. `get_scene` reports each object's `masked_vertices`.
4. Faster ways to mask, each combinable with `op` (replace, add, subtract, intersect):
   - `mask_shape`: look with `get_view`, then give a box, ellipse or lasso in that picture's coordinates (0 to 1, y down) with the same view, width and height.
   - `mask_topology`: from a point outward by distance along the surface, so one finger or one ear is masked without its neighbour.
   - `mask_cavity`: creases and hollows, or `mode: "peaks"` for ridges (a smooth form masks nothing).
   - `mask_by_color`: the painted areas near a colour.
5. `set_visibility` hides, shows or isolates objects so a crowded scene does not get in the way of `get_view` or picking; `arrange_objects` aligns or distributes them on an axis.
6. `get_history` lists every step by name; `history_go_to` jumps back to any of them (and forward again).
7. Face groups name regions: `group_auto` (by angle for hard-surface planes, by parts for loose pieces) or `group_from_mask`; then `mask_by_group` or `hide_faces` by group (a point on it or its number). Hidden faces cannot be hit or moved, so hide what is in the way and sculpt behind it; `hide_faces` with `show_all` brings them back. `get_view` with `shading: "groups"` shows the groups.
8. `set_folder` files objects under a folder; `set_visibility` with `folder` hides, shows or isolates them together.

## Workflow 10: repeat, insert and present

1. `insert_mesh` places a primitive or a copy of another object (`source_id`) at a `point` with its `normal` (from `pick`), standing on the surface: eyes, horns, rivets.
2. `array_object` repeats an object in a line (`offset`) or radially around an axis (`mode: "radial"`, `center`, `count`): teeth, petals, bolts; `merge` joins them into one object. `mode: "mirror"` makes one reflected copy across `axis` through the object's centre; `mode: "curve"` spaces `count` copies along `points` (`align` turns them to follow it): vertebrae, chain links, studs on a strap. `linked: true` keeps the copies following the original, so sculpt the original and every copy updates; a copy you edit yourself stops following (`instance_of` in `get_scene` shows which are still linked). Linked copies cannot be merged.
3. `subdivide` with `sharp_border` keeps open borders in place (planes, cut shells).
4. `record_turntable` writes a looping animated PNG of the model turning once, for showing the result.

## When something goes wrong

- A stroke that did the wrong thing: `undo`, then adjust radius, strength or points and try again. Do not repair it with more strokes.
- `not_found` or `no_object`: call `get_scene` for current ids; the active object is used when `id` is omitted.
- `bad_arguments`: the message names the field and its allowed range.
- A refusal about permission: the person must allow outside agents in the formsculptor window. Tell them; do not retry in a loop.
- The engine not running: the MCP server starts it on the first call; if that fails, ask the person to run `./formsculptor backend` in the formsculptor folder.

## Workflow 4: manage the mesh's resolution

1. Strokes stretch triangles. After heavy grab or inflate work, `remesh_voxel` (resolution 96 to 160 for most work) rebuilds an even, closed mesh; it clears the mask.
2. `multires_divide` adds subdivision levels and keeps the lower ones: block out big forms with `multires_set_level` at level 0 (or `step: "down"`), then step up and detail; changes on a low level carry up with the high detail kept. `get_scene` reports each object's `level` and `levels`. While an object has levels, dyntopo, remesh, decimate, subdivide and fill holes refuse with `has_levels`; `multires_delete_lower` plus `multires_delete_higher` leave a plain mesh again. `subdivide` is the one-step version with no levels to return to; `multires_reconstruct` at level 0 rebuilds a level below a mesh that came from subdivision.
3. `dyntopo` on `sculpt_stroke` (a fraction of the radius, 0.15 to 0.3) splits long edges and collapses short ones where the brush passes, so a low-detail sphere can take fine detail locally; `dyntopo_size` sets a constant edge length in world units instead.
4. `decimate` with `ratio` reduces triangles for export or speed; `mesh_check` reports watertightness, and `fill_holes` closes open boundaries before printing.

## Workflow 5: combine, bend and layer

1. Build parts as separate objects (`add_primitive`, `duplicate_object` with an `offset`, `transform_object` to move, rotate and scale), then `boolean` with `a`, `b` and `op` (union, difference as a minus b, intersection). The result is one watertight object and replaces both unless `keep_inputs`; detail finer than a voxel softens, so use `resolution` 160 to 256 for crisp joins. `merge_objects` only groups meshes; `split_parts` separates loose pieces. `blend` (world units) fillets the seam: a union with blend 0.1 to 0.3 of the parts' size fuses an arm into a shoulder or a horn into a head with a soft web, a difference with a blend leaves a rounded cut.
2. `cut_shape` cuts hard-surface shapes: a closed `outline` of world points and the `direction` it is seen along (from `get_view`'s camera, or an axis such as [0, 0, -1] for the front); the outline is extruded straight through the object and taken away, or with `keep: "inside"` kept alone. Vents, slots, windows, notches, a block trimmed to a profile; `round` softens the cut edges.
3. `deform_object` bends, twists or tapers along an axis of the object's bounds; mask an area first to hold it still (masks also hold vertices during `transform_object`).
4. For an adjustable pass of detail, `layer_add`, sculpt, then `layer_set_strength` (0 hides it, 1 as sculpted, 2 exaggerates, negative inverts). `layer_select` with null records into the base again; `layer_apply` bakes a layer. While layers exist, remesh, decimate, dyntopo and level changes refuse with `has_layers`.

## Workflow 8: retopology and mask modeling

1. For clean, even topology: `duplicate_object` (it copies in place), `quad_remesh` the copy with `faces` (a few thousand for a base mesh), `multires_divide` it two or three times, then `project_detail` with `source_id` set to the original to bring the sculpted detail back; delete or hide the original after.
2. Mask an area, then `extract_mask` (`thickness`) for a separate shell such as a strap or armour plate, `extrude_mask` (`distance`) to raise or sink it with side walls, or `inset_mask` (`amount`, `depth`) for a panel or socket. Each needs faces fully inside the mask.

## Workflow 9: base meshes from a skeleton, a curve or a profile

1. To block out a figure, animal or hand, give `skeleton_mesh` the joints as `spheres` ([x, y, z, radius], y up, about 2 units tall for a figure) and the bones as `links` (index pairs). It returns one watertight, even mesh; raise `resolution` (up to 256) when limbs are thinner than about a fiftieth of the whole, lower `blend` (0 to 1) for sharper joints. Then sculpt it, or `multires_divide` it for detail.
2. `curve_tube` sweeps a tube along `points` ([x, y, z] or [x, y, z, radius] to taper) for horns, tails, hair strands, cables or straps; `closed` makes a ring. Union it into the body with `boolean` when it should be one piece.
3. `lathe` spins a `profile` of [radius, y] points around the y axis for vases, bottles, columns and turned parts; start and end at radius 0 for a closed top and bottom, or leave them open and they are capped flat.
4. `shadow_box` builds a closed solid from two or three outlines: `front` as [x, y] points, `side` as [z, y], `top` as [x, z]; the solid is what all of them agree on, like a shadow box. A boot, a chair, a gun stock, a head block from a profile and a front view. The outlines must overlap; `blend` (0 to 0.5) rounds the meeting edges.
5. `text_mesh` makes solid lettering (`text`, `size`, `weight`, `depth`; capitals, digits and . , : - + ! ? ' /), facing the front and centred on the origin: a name on a plinth (union it, or subtract it to engrave). `extrude_shape` turns flat [x, y] `outlines` into a solid of `depth`, inner outlines cutting holes: badges, logos, plates.
6. `vdm_stamp` stamps a vector displacement map at a `point` and `normal` from `pick`: `horn`, `hook` or `mushroom`, or a map made with `vdm_capture` from a plane (add_primitive kind plane, detail 64 or more, sculpted in the middle without dyntopo). `radius` sets its half-width, `angle` its lean, negative `strength` presses it in. The surface needs enough vertices under the stamp: divide first.
7. Each makes a new active object named by `name`; `transform_object` places it.

## Workflow 7: paint

1. Colour a whole object with `paint_fill` (`color` as #rrggbb or [r, g, b] from 0 to 1, and optionally `roughness` and `metalness`); mask first to keep areas as they are.
2. Paint by hand with `sculpt_stroke` and `brush: "paint"` plus `color` (and `roughness` or `metalness` to paint those channels); the radius and points work as for sculpting. `paint_clear` returns to plain clay.
3. Paint stays through subdivision levels, dyntopo, remesh, booleans and saving; export PLY, OBJ or GLB to keep the colours (GLB also gets the average roughness and metalness as its material).
4. For a light game-ready model: sculpt detail on high subdivision levels, `multires_set_level` to 0, `uv_unwrap`, `bake_maps` (size 1024 or 2048; `path` also writes PNG and EXR files), then `export_mesh` to .glb, which carries UVs, tangents and the colour, roughness-metal and normal maps. To bake from a separate high-resolution object onto a simpler one, give `source_id`.

## Workflow 6: files and printing

1. `save_project` and `open_project` take a path ending in .fsc and keep everything (levels, layers, masks); opening replaces the scene and clears undo, so save first when the current work matters. The window also autosaves every minute.
2. `import_mesh` reads OBJ, STL, PLY, GLB (or glTF with embedded buffers) and 3MF from a path as new objects; `export_mesh` writes OBJ, STL, PLY, GLB or 3MF by the path's extension (GLB keeps layers as morph targets; STL and PLY join objects).
3. Before printing, `print_check` (set `units_per_mm` to the scale you mean and `min_wall_mm` to the printer's limit) and fix what it lists: `repair_mesh` for open or non-manifold edges, thicken or `remesh_voxel` thin walls, then check again.

## Not yet available

There are no bevel, edge loop or bridge tools, `get_view` has no turntable or orbit beyond its named views and `yaw` and `pitch`, and lower levels cannot be rebuilt for a remeshed, decimated or dyntopo mesh. Say so plainly if the person asks for them, rather than approximating with strokes.
