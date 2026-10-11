# formsculptor tools

Generated from the engine's command registry (formsculptor 0.9.0) by `./formsculptor agents sync`; do not edit by hand.

## Contents

- Engine: `get_engine_info`
- Scene: `get_scene`, `new_scene`
- Viewing: `get_view`
- Objects: `add_primitive`, `skeleton_mesh`, `curve_tube`, `lathe`, `select_object`, `delete_object`, `duplicate_object`, `rename_object`, `transform_object`, `deform_object`, `merge_objects`, `split_parts`, `boolean`
- Sculpting: `sculpt_stroke`
- Painting: `paint_fill`, `paint_clear`, `uv_unwrap`, `bake_maps`
- Masking: `mask_invert`, `mask_clear`, `mask_blur`, `mask_sharpen`, `extract_mask`, `extrude_mask`, `inset_mask`
- Topology: `subdivide`, `remesh_voxel`, `decimate`, `fill_holes`, `mesh_check`, `quad_remesh`, `project_detail`, `multires_divide`, `multires_set_level`, `multires_delete_higher`, `multires_delete_lower`, `multires_reconstruct`, `repair_mesh`
- Layers: `layer_add`, `layer_select`, `layer_set_strength`, `layer_delete`, `layer_apply`
- Files: `save_project`, `open_project`, `import_mesh`, `export_mesh`, `print_check`
- History: `undo`, `redo`

## get_engine_info

The engine's name and version. Use it to confirm the engine is reachable before other calls.

- no parameters

Note: read-only.

## get_scene

Every object in the scene with its id, name, visibility, vertex and triangle counts and bounds, and which object is active. Call it first to learn object ids.

- no parameters

Note: read-only.

## get_view

Render the scene to a PNG and return it, so you can see the model: drawn on the CPU with the window's shading (clay matcap by default, paint shown, masked areas darkened), framed to fit, with or without the window open. Check your work with it after each step that changes shape, from more than one view (front, right and top read proportions; three_quarter reads form).

- `background`: string (dark | light | transparent); default "dark"
- `frame`: string (all | active); default "all"; Fit the camera to all drawn objects or to the active one (others still drawn).
- `height`: integer; default 640; range 64 to 2048
- `ids`: array of integer; Only these objects; every visible object when omitted.
- `pitch`: number; range -90 to 90; Degrees above the horizon; overrides view.
- `shading`: string (clay | wax | pearl | bronze | gray | pbr); default "clay"; The window's matcaps, or pbr for painted roughness and metalness under three lights.
- `view`: string (three_quarter | front | back | right | left | top | bottom); default "three_quarter"; Camera direction; three_quarter is front-right and a little above, as the window opens. Front looks along -z.
- `width`: integer; default 640; range 64 to 2048
- `yaw`: number; range -360 to 360; Degrees around the vertical axis from the front; overrides view.

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

## skeleton_mesh

Build a base mesh from a skeleton of spheres ([x, y, z, radius]) joined by tapered limbs (links are index pairs), merged smoothly into one watertight, even mesh and made active: the quick way to block out a figure, animal or hand before sculpting. Unlinked spheres stay round. Raise resolution for thin limbs.

- `blend`: number; default 0.3; range 0 to 1; How softly limbs merge where they meet, as a fraction of the mean radius.
- `links`: array of array; Pairs of sphere indices joined by a tapered limb.
- `name`: string; Name for the new object; a default when omitted.
- `resolution`: integer; default 96; range 32 to 256; Voxel cells along the longest side.
- `spheres`: array of array; required; Joints as [x, y, z, radius].

## curve_tube

Sweep a round tube along a smooth curve through the given points (radius per point or overall), capped or closed into a ring, and make it active: hair strands, horns, tails, cables, straps, piping.

- `closed`: boolean; default false; Join the last point back to the first (a ring) instead of capping the ends.
- `name`: string; Name for the new object; a default when omitted.
- `points`: array of array; required; The curve as [x, y, z] or [x, y, z, radius] points; a point's radius overrides radius.
- `radius`: number; default 0.1; range 0 to inf; Tube radius where a point gives none.
- `segments`: integer; default 16; range 3 to 128; Sides around the tube.
- `smooth`: integer; default 4; range 1 to 32; Rings per curve span (a smooth spline through the points).

## lathe

Spin an outline of [radius, y] points around the vertical axis into a closed solid and make it active: vases, bottles, cups, columns, turned legs, chess pieces.

- `name`: string; Name for the new object; a default when omitted.
- `profile`: array of array; required; The outline as [radius, y] points from bottom to top, spun around the y axis. A radius of 0 closes to a point; open ends are capped flat.
- `segments`: integer; default 48; range 3 to 256; Steps around the axis.

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

- `brush`: string (draw | clay | inflate | pinch | crease | flatten | smooth | grab | mask | paint); default "draw"; paint colours the surface instead of moving it, using color, roughness and metalness.
- `color`: ['string', 'array']; Paint brush: #rrggbb or [r, g, b] from 0 to 1.
- `dyntopo`: ['number', 'boolean']; Dynamic topology: split edges longer than this fraction of the radius (0.05 to 1; true means 0.25) under the brush as it moves, adding detail where you sculpt, and collapse edges under 0.4 of that length. Off when omitted. Not used by grab or mask, nor on objects with subdivision levels.
- `dyntopo_size`: number; range -inf to 10000; Constant detail: the target edge length in world units, whatever the radius. Turns dynamic topology on and overrides the dyntopo fraction.
- `falloff`: string (smooth | sphere | linear | sharp); default "smooth"
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `invert`: boolean; default false; Carve instead of build, deflate, unpinch, erase mask.
- `lazy`: number; default 0; range 0 to 0.95
- `metalness`: number; range 0 to 1; Paint brush: paint this metalness.
- `points`: array of array; required; The stroke path as [x, y, z] or [x, y, z, pressure] points on or near the surface, in object space. Grab moves the vertices around the first point by the offset to the last.
- `radius`: number; default 0.2; range -inf to 10000; World units; compare with the object's bounds from get_scene.
- `roughness`: number; range 0 to 1; Paint brush: paint this roughness (only the channels given are painted).
- `spacing`: number; default 0.15; range 0.02 to 2; Distance between dabs as a fraction of the radius.
- `strength`: number; default 0.5; range 0 to 1
- `symmetry`: object; fields radial, x, y, z

## paint_fill

Paint a whole object with a colour, roughness or metalness (only the channels given); masked areas are protected in proportion to the mask. For painting by hand use sculpt_stroke with brush paint. Paint follows subdivision levels, dyntopo, remesh, booleans and export (PLY, OBJ, GLB).

- `color`: ['string', 'array']; #rrggbb or [r, g, b] from 0 to 1.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `metalness`: number; range 0 to 1
- `roughness`: number; range 0 to 1

## paint_clear

Reset an object's paint to the default clay colour, roughness 0.5 and metalness 0.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## uv_unwrap

Give an object UV coordinates: faces are grouped into charts by the direction they face, each chart is projected flat and the charts are packed into the unit square keeping their relative sizes. Needed before bake_maps; topology changes (remesh, dyntopo, subdivide, booleans) clear the UVs. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## bake_maps

Bake an unwrapped object's maps: colour and roughness-metal from paint, a tangent-space normal map and a 32-bit displacement map from the detail of its highest subdivision level (or another object with source_id), by casting along each texel's normal. The maps go into GLB export with UVs and tangents, and into PNG and EXR files when path is given. Typical flow: multires_set_level 0, uv_unwrap, bake_maps, export_mesh to .glb.

- `id`: integer; range 1 to inf; The object to bake onto (it needs UVs from uv_unwrap); the active object when omitted.
- `path`: string; A folder to also write NAME_color.png, NAME_roughness_metal.png, NAME_normal.png and NAME_displacement.exr into.
- `size`: integer (256 | 512 | 1024 | 2048 | 4096); default 1024; Texture width and height in pixels.
- `source_id`: integer; range 1 to inf; Bake detail from this object (a high-resolution sculpt) instead; by default the object's highest subdivision level, or the object itself.

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

## quad_remesh

Rebuild an object as an even, quad-dominant mesh near a target face count: quads from a voxel grid, relaxed along the original surface. OBJ export writes them as quads. The grid makes edge flow follow the axes rather than the form (no guide curves). One undo step; the mask is cleared, paint is carried over.

- `faces`: integer; default 4000; range 100 to 400000; Target number of quads.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `relax`: integer; default 6; range 0 to 30; Smoothing passes that even out the quads while staying on the original surface.

## project_detail

Move an object's vertices along their normals onto another object's surface (within distance), carrying its detail over: for example from an old sculpt onto a quad-remeshed or subdivided copy. Masked vertices are held. One undo step.

- `distance`: number; range 0 to inf; Furthest a vertex may move, in world units; 5 percent of the object's size when omitted.
- `id`: integer; range 1 to inf; The object to move; the active object when omitted.
- `source_id`: integer; required; range 1 to inf; The object whose surface detail is taken.
- `strength`: number; default 1; range 0 to 1

## extract_mask

Make a new closed object from the masked part of an object, thickened along its normals: a shell for clothing, armour or straps. The original is unchanged.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `thickness`: number; range -10000 to 10000; Shell thickness along the surface normal, in world units; 2 percent of the object's size when omitted. Negative grows inward.

## extrude_mask

Push the masked faces out along their normals (or in, with a negative distance), joining them to the rest with new side faces. The mesh stays closed. One undo step.

- `distance`: number; range -10000 to 10000; How far the masked faces move along their normals, in world units; 3 percent of the object's size when omitted. Negative pushes in.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## inset_mask

Pull the border of the masked faces inward to make a rim of new faces, optionally pushing the inset area by depth (a panel, socket or button). One undo step.

- `amount`: number; default 0.3; range 0 to 1; How far the masked area's border pulls inward, as a fraction of the border faces.
- `depth`: number; default 0; range -10000 to 10000; Also move the inset faces along their normals by this much.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.

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

## save_project

Save the whole scene to a .fsc project file (objects, masks, subdivision levels and sculpt layers), overwriting the path. Opening it again restores the scene exactly.

- `path`: string; required; A file path on this computer.

Note: removes or replaces (undoable); in the window: Mod+S.

## open_project

Replace the scene with a .fsc project. Clears the undo history; save first if the current scene matters.

- `path`: string; required; A file path on this computer.

Note: removes or replaces (undoable); in the window: Mod+O.

## import_mesh

Add the meshes in an OBJ, STL, PLY, glTF/GLB or 3MF file as new objects (one per OBJ object, glTF node or 3MF object). glTF must be a GLB or carry its buffers inside. One undo step.

- `name`: string; Name for the new object; the file name when omitted.
- `path`: string; required; A file path on this computer.

## export_mesh

Write objects to OBJ, STL, PLY, GLB or 3MF, chosen by the path's extension, overwriting it. GLB keeps sculpt layers as morph targets at their strengths; STL and PLY join the objects into one mesh.

- `ids`: array of integer; Objects to write; every visible object when omitted. STL and PLY join them into one mesh.
- `path`: string; required; A file path on this computer.

Note: removes or replaces (undoable); in the window: Mod+E.

## print_check

Check an object for 3D printing: watertight, non-manifold edges, inside-out, size, volume and area in millimetres, and wall thickness sampled by casting inward from up to 4000 vertices. Lists the problems and whether it is printable.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `min_wall_mm`: number; default 1.0; range 0 to inf; Thinnest wall the printer can make.
- `units_per_mm`: number; default 0.1; range 0 to inf; Scene units in one millimetre (0.1 means the default 2-unit sphere prints 20 mm across).

Note: read-only.

## repair_mesh

Make an object closed and manifold: split vertices where surfaces only touch, fill open holes, drop degenerate triangles and unused vertices. One undo step; the mask is cleared.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## undo

Undo the last step (a stroke, a mask change, adding or deleting an object, a new scene, a topology change, a level change).

- no parameters

Note: in the window: Mod+Z.

## redo

Redo the last undone step.

- no parameters

Note: in the window: Mod+Shift+Z.
