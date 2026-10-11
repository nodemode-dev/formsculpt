---
name: formsculptor-modeling
description: Sculpt and model 3D objects in formsculptor, the 3D sculpting app, by asking in plain language through its MCP server. Use whenever the person wants a 3D model, sculpt, bust, creature, prop or base mesh made or changed, a primitive added, a surface built up, carved, smoothed, flattened, creased, pinched or pulled, a mask painted or cleared, symmetry used, or a step undone, and the formsculptor tools (get_scene, add_primitive, sculpt_stroke and the rest) are available or the person mentions formsculptor. Covers reading the scene, choosing brush radius from object bounds, building strokes as point paths, masking, symmetry and checking work. Also use when a formsculptor tool fails and the next step is unclear. Not for editing formsculptor's own source code (use formsculptor-harness for that).
---

# Sculpting in formsculptor

formsculptor's engine holds a scene of meshes on this computer. The tools change that scene directly; the formsculptor window, when it is open, shows each change live, and the tools work the same with no window at all. Every change is one undoable step.

The full tool reference, generated from the engine's command registry, is `references/tools.md`. Read it when a parameter is unclear.

## Before anything

1. Call `get_scene`. It lists every object with its id, vertex and triangle counts and bounds, and which object is active.
2. If the scene is empty, `add_primitive` first. For sculpting use `kind: "sphere"` with `detail` 6 (81,920 triangles) or 7 (327,680); a low detail gives blocky strokes.
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
2. Use `symmetry: {"x": true}` for anything bilateral (faces, bodies); `radial: n` repeats a stroke n times around the y axis (petals, gears, crowns).
3. Prefer several light strokes (strength 0.2 to 0.4) to one heavy one.

## Workflow 3: protect an area

1. `sculpt_stroke` with `brush: "mask"` paints the mask over what must not move.
2. Sculpt; masked vertices resist every other brush.
3. `mask_invert` to work on the other side, `mask_blur` to soften the boundary, `mask_clear` when done.

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

1. Build parts as separate objects (`add_primitive`, `duplicate_object` with an `offset`, `transform_object` to move, rotate and scale), then `boolean` with `a`, `b` and `op` (union, difference as a minus b, intersection). The result is one watertight object and replaces both unless `keep_inputs`; detail finer than a voxel softens, so use `resolution` 160 to 256 for crisp joins. `merge_objects` only groups meshes; `split_parts` separates loose pieces.
2. `deform_object` bends, twists or tapers along an axis of the object's bounds; mask an area first to hold it still (masks also hold vertices during `transform_object`).
3. For an adjustable pass of detail, `layer_add`, sculpt, then `layer_set_strength` (0 hides it, 1 as sculpted, 2 exaggerates, negative inverts). `layer_select` with null records into the base again; `layer_apply` bakes a layer. While layers exist, remesh, decimate, dyntopo and level changes refuse with `has_layers`.

## Workflow 8: retopology and mask modeling

1. For clean, even topology: `duplicate_object` (it copies in place), `quad_remesh` the copy with `faces` (a few thousand for a base mesh), `multires_divide` it two or three times, then `project_detail` with `source_id` set to the original to bring the sculpted detail back; delete or hide the original after.
2. Mask an area, then `extract_mask` (`thickness`) for a separate shell such as a strap or armour plate, `extrude_mask` (`distance`) to raise or sink it with side walls, or `inset_mask` (`amount`, `depth`) for a panel or socket. Each needs faces fully inside the mask.

## Workflow 9: base meshes from a skeleton, a curve or a profile

1. To block out a figure, animal or hand, give `skeleton_mesh` the joints as `spheres` ([x, y, z, radius], y up, about 2 units tall for a figure) and the bones as `links` (index pairs). It returns one watertight, even mesh; raise `resolution` (up to 256) when limbs are thinner than about a fiftieth of the whole, lower `blend` (0 to 1) for sharper joints. Then sculpt it, or `multires_divide` it for detail.
2. `curve_tube` sweeps a tube along `points` ([x, y, z] or [x, y, z, radius] to taper) for horns, tails, hair strands, cables or straps; `closed` makes a ring. Union it into the body with `boolean` when it should be one piece.
3. `lathe` spins a `profile` of [radius, y] points around the y axis for vases, bottles, columns and turned parts; start and end at radius 0 for a closed top and bottom, or leave them open and they are capped flat.
4. Each makes a new active object named by `name`; `transform_object` places it.

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

There are no folders or on-canvas gizmo, `get_view` has no turntable or orbit beyond its named views and `yaw` and `pitch`, and lower levels cannot be rebuilt for a remeshed, decimated or dyntopo mesh. Say so plainly if the person asks for them, rather than approximating with strokes.
