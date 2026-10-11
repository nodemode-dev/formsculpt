# formsculptor tools

Generated from the engine's command registry (formsculptor 0.5.0) by `./formsculptor agents sync`; do not edit by hand.

## Contents

- Engine: `get_engine_info`
- Scene: `get_scene`, `new_scene`
- Objects: `add_primitive`, `select_object`, `delete_object`, `duplicate_object`, `rename_object`, `transform_object`, `deform_object`, `merge_objects`, `split_parts`, `boolean`
- Sculpting: `sculpt_stroke`
- Masking: `mask_invert`, `mask_clear`, `mask_blur`, `mask_sharpen`
- Topology: `subdivide`, `remesh_voxel`, `decimate`, `fill_holes`, `mesh_check`, `multires_divide`, `multires_set_level`, `multires_delete_higher`, `multires_delete_lower`, `multires_reconstruct`
- Layers: `layer_add`, `layer_select`, `layer_set_strength`, `layer_delete`, `layer_apply`
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
- `dyntopo`: ['number', 'boolean']; Dynamic topology: split edges longer than this fraction of the radius (0.05 to 1; true means 0.25) under the brush as it moves, adding detail where you sculpt, and collapse edges under 0.4 of that length. Off when omitted. Not used by grab or mask, nor on objects with subdivision levels.
- `dyntopo_size`: number; range -inf to 10000; Constant detail: the target edge length in world units, whatever the radius. Turns dynamic topology on and overrides the dyntopo fraction.
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

## subdivide

Subdivide an object's mesh, each level multiplying the triangles by four (Loop subdivision, which rounds the shape; smooth false splits flat). The mask carries over. This replaces the mesh in one step; multires_divide keeps the lower levels to step back to.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `levels`: integer; default 1; range 1 to 4; Each level multiplies the triangles by four.
- `smooth`: boolean; default true; Loop subdivision (smooths the shape); false splits flat.

## remesh_voxel

Rebuild an object as an even, closed mesh by sampling its surface on a voxel grid (like DynaMesh or Nomad's voxel remesher). Fixes stretched triangles after heavy grab or inflate strokes and merges intersecting parts. resolution is voxels along the longest side. The mask is cleared.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `resolution`: integer; default 128; range 8 to 320; Voxels along the object's longest side; higher keeps more detail and makes more triangles.

Note: in the window: Mod+R.

## decimate

Reduce an object's triangle count while keeping its shape (quadric error edge collapse), by ratio or to a target count; borders are kept by default. The mask is cleared.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `preserve_borders`: boolean; default true
- `ratio`: number; default 0.5; range -inf to 1; Fraction of triangles to keep.
- `triangles`: integer; range 4 to inf; Target triangle count; overrides ratio.

## fill_holes

Close every open boundary loop of an object with a fan of triangles. Reports how many holes were filled.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## mesh_check

Report an object's vertex, triangle and edge counts, open boundary edges, non-manifold edges and whether it is watertight (needed for printing and booleans).

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

Note: read-only.

## multires_divide

Add subdivision levels on top of an object (Loop subdivision, each level four times the triangles) and keep the lower levels, so you can block out on a low level and detail on a high one. The object must be at its highest level. Objects with levels refuse dyntopo, remesh, decimate, subdivide and fill holes until the levels are deleted.

- `count`: integer; default 1; range 1 to 4; Levels to add; each multiplies the triangles by four.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `smooth`: boolean; default true; Loop subdivision (smooths the shape); false splits flat.

Note: in the window: Mod+D.

## multires_set_level

Move an object to another subdivision level by number or one step up or down. Changes made on a lower level carry up to the higher levels with their detail kept; the mask carries both ways.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `level`: integer; range 0 to inf; The level to sculpt on, 0 being the lowest; get_scene reports level and levels.
- `step`: string (up | down); One level higher or lower instead of a level number.

Note: in the window: D, Shift+D.

## multires_delete_higher

Delete the subdivision levels above the current one. Undoable.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

Note: removes or replaces (undoable).

## multires_delete_lower

Delete the subdivision levels below the current one, making it the base. Deleting both lower and higher levels leaves a plain mesh that dyntopo and remeshing accept. Undoable.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

Note: removes or replaces (undoable).

## multires_reconstruct

Rebuild one subdivision level below the lowest one, for a mesh with the regular one-to-four structure of a subdivided mesh (one made by subdivide, or left after multires_delete_lower). Repeat to rebuild more. Refuses with not_subdivided for other meshes (remeshed, decimated, sculpted with dyntopo or imported irregular meshes). Undoable.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## duplicate_object

Copy an object's current mesh and mask into a new object, optionally moved by an offset. Subdivision levels and layers are not copied.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `offset`: array of number; Move the copy by this much; default none.

## rename_object

Give an object a new name.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `name`: string; required

## transform_object

Move, rotate (degrees about x, y, z) and scale an object's vertices around its bounds centre. Masked vertices stay put in proportion to the mask, so a mask works as topology masking. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `rotate`: array of number; Degrees about x, y and z (applied in that order) around the object's bounds centre.
- `scale`: ['number', 'array']; A factor or [x, y, z] factors around the bounds centre.
- `translate`: array of number; World offset.

## deform_object

Bend, twist or taper an object along an axis of its bounds. Masked vertices are held in proportion to the mask. One undo step.

- `amount`: number; range -2 to 2; Taper: change in width from one end to the other (0.5 is half again wider at the positive end).
- `angle`: number; range -720 to 720; Bend and twist: total degrees across the object's length.
- `axis`: string (x | y | z); default "y"; The axis the deformation runs along, over the object's bounds.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `kind`: string (bend | twist | taper); required
- `toward`: string (x | y | z); default "x"; Bend only: the direction the axis curves toward.

## merge_objects

Join several objects into the first one's mesh (the parts stay separate shells; use boolean union to fuse them).

- `ids`: array of integer; required; Objects to join into the first one, which keeps its name.

## split_parts

Split an object into one object per connected part of its mesh.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## boolean

Union, difference (a minus b) or intersection of two objects through a voxel grid, giving one watertight object; the inputs are replaced unless keep_inputs. Detail finer than a voxel is smoothed, so raise resolution for sharp results. Both inputs should be closed meshes (mesh_check).

- `a`: integer; required; range 1 to inf; The object to keep or cut from.
- `b`: integer; required; range 1 to inf; The object to add, cut away or intersect with.
- `keep_inputs`: boolean; default false; Keep a and b instead of replacing them with the result.
- `op`: string (union | difference | intersection); default "union"
- `resolution`: integer; default 160; range 16 to 320; Voxels along the longest side of both objects together; higher keeps sharper detail.

## layer_add

Add a sculpt layer to an object and make it the one strokes record into. A layer stores the strokes made while it is active as offsets, so its strength can later fade, exaggerate or invert them.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `name`: string

## layer_select

Choose which layer strokes record into, or null for the base mesh.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `index`: ['integer', 'null']; required; range 0 to inf; The layer strokes record into; null records into the base mesh.

## layer_set_strength

Set a layer's strength (0 hides it, 1 is as sculpted, up to 2, negative inverts).

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `index`: integer; required; range 0 to inf; Layer position, 0 first; get_scene lists each object's layers.
- `strength`: number; range -1 to 2; 0 hides the layer, 1 is as sculpted, negative inverts it.

## layer_delete

Remove a layer and the shape it adds.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `index`: integer; required; range 0 to inf; Layer position, 0 first; get_scene lists each object's layers.

Note: removes or replaces (undoable).

## layer_apply

Bake a layer into the mesh at its current strength and remove it. While an object has layers, topology changes, dyntopo and level changes refuse with has_layers.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `index`: integer; required; range 0 to inf; Layer position, 0 first; get_scene lists each object's layers.

## undo

Undo the last step (a stroke, a mask change, adding or deleting an object, a new scene, a topology change, a level change).

- no parameters

Note: in the window: Mod+Z.

## redo

Redo the last undone step.

- no parameters

Note: in the window: Mod+Shift+Z.
