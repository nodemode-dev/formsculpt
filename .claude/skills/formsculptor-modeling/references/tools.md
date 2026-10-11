# formsculptor tools

Generated from the engine's command registry (formsculptor 0.2.0) by `./formsculptor agents sync`; do not edit by hand.

## Contents

- Engine: `get_engine_info`
- Scene: `get_scene`, `new_scene`
- Objects: `add_primitive`, `select_object`, `delete_object`
- Sculpting: `sculpt_stroke`
- Masking: `mask_invert`, `mask_clear`, `mask_blur`, `mask_sharpen`
- History: `undo`, `redo`

## get_engine_info

The engine's name and version. Use it to confirm the engine is reachable before other calls.

- no parameters

Note: read-only.

## get_scene

Every object in the scene with its id, name, visibility, vertex and triangle counts and bounds, and which object is active. Call it first to learn object ids.

- no parameters

Note: read-only.

## new_scene

Remove every object and start an empty scene.

- no parameters

Note: removes or replaces (undoable); in the window: Mod+Alt+N.

## add_primitive

Add a primitive mesh centred at the origin and make it active. Kinds: sphere (an icosphere; subdivisions 0 to 9, each level four times the triangles; 6 gives 81,920), uv_sphere, cube (divisions per edge), cylinder, cone, torus and plane. Returns the new object's summary.

- `detail`: integer; range 0 to 2048; Subdivisions for sphere (0 to 9, default 5), divisions for cube and plane (default 16), segments for the round kinds (default 64).
- `height`: number; default 2; range -inf to 1000; Cylinder and cone height.
- `kind`: string (sphere | uv_sphere | cube | cylinder | cone | torus | plane); required
- `name`: string
- `size`: number; default 1; range -inf to 1000; Radius for sphere, uv_sphere, cylinder, cone and the torus ring; edge length for cube and plane.
- `thickness`: number; default 0.3; range -inf to 1000; Torus tube radius.

## select_object

Make an object the active one by id.

- `id`: integer; required; range 1 to inf; An object id from get_scene.

Note: safe to repeat.

## delete_object

Remove an object from the scene by id.

- `id`: integer; required; range 1 to inf; An object id from get_scene.

Note: removes or replaces (undoable); in the window: Delete.

## sculpt_stroke

Run one brush stroke along a path of points on the surface of an object, as one undo step. Brushes: draw (build along the area normal), clay (build up to a plane), inflate (along each vertex normal), pinch, crease, flatten, smooth, grab (move the vertices around the first point by the offset to the last) and mask (paint the mask; masked vertices resist every other brush). invert carves, deflates or erases. Points closer than radius times spacing are merged into dabs; space points about radius * 0.15 apart for an even stroke. Symmetry mirrors across the object's x, y or z planes through the origin and repeats radially around y.

- `brush`: string (draw | clay | inflate | pinch | crease | flatten | smooth | grab | mask); default "draw"
- `falloff`: string (smooth | sphere | linear | sharp); default "smooth"
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `invert`: boolean; default false; Carve instead of build, deflate, unpinch, erase mask.
- `lazy`: number; default 0; range 0 to 0.95
- `points`: array of array; required; The stroke path as [x, y, z] or [x, y, z, pressure] points on or near the surface, in object space. Grab moves the vertices around the first point by the offset to the last.
- `radius`: number; default 0.2; range -inf to 10000; World units; compare with the object's bounds from get_scene.
- `spacing`: number; default 0.15; range 0.02 to 2; Distance between dabs as a fraction of the radius.
- `strength`: number; default 0.5; range 0 to 1
- `symmetry`: object; fields radial, x, y, z

## mask_invert

Invert the mask of an object (the active one when id is omitted).

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

Note: in the window: Mod+I.

## mask_clear

Remove the mask from an object so every vertex can be sculpted.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

Note: in the window: Mod+Shift+A.

## mask_blur

Soften the mask's edge by averaging each vertex with its neighbours once.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## mask_sharpen

Harden the mask's edge by pushing values away from one half.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## undo

Undo the last step (a stroke, a mask change, adding or deleting an object, a new scene).

- no parameters

Note: in the window: Mod+Z.

## redo

Redo the last undone step.

- no parameters

Note: in the window: Mod+Shift+Z.
