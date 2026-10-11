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
4. `get_scene` to check the bounds moved as intended.

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

## Not yet available

There are no file import or export tools, no dynamic topology, remeshing or subdivision, and no image of the view yet. Say so plainly if the person asks for them, rather than approximating with strokes.
