# formsculptor tools

Generated from the engine's command registry (formsculptor 0.20.0) by `./formsculptor agents sync`; do not edit by hand.

## Contents

- Engine: `get_engine_info`
- Scene: `get_scene`, `new_scene`, `edit_primitive`
- Viewing: `get_view`, `record_turntable`, `pick`
- Objects: `set_visibility`, `set_folder`, `arrange_objects`, `add_primitive`, `skeleton_mesh`, `shadow_box`, `text_mesh`, `extrude_shape`, `curve_tube`, `lathe`, `select_object`, `delete_object`, `duplicate_object`, `rename_object`, `transform_object`, `deform_object`, `lattice_deform`, `merge_objects`, `split_parts`, `array_object`, `insert_mesh`, `boolean`
- History: `get_history`, `history_go_to`, `undo`, `redo`
- Topology: `cut_shape`, `subdivide`, `remesh_voxel`, `decimate`, `fill_holes`, `mesh_check`, `quad_remesh`, `project_detail`, `multires_divide`, `multires_set_level`, `multires_delete_higher`, `multires_delete_lower`, `multires_reconstruct`, `repair_mesh`
- Sculpting: `sculpt_stroke`, `alpha_load`, `vdm_stamp`, `vdm_capture`
- Painting: `paint_fill`, `paint_project`, `paint_layer_add`, `paint_layer_set`, `paint_layer_select`, `paint_layer_delete`, `paint_layer_merge`, `paint_layer_flatten`, `paint_clear`, `uv_unwrap`, `bake_maps`
- Masking: `mask_invert`, `mask_clear`, `mask_blur`, `mask_sharpen`, `mask_cavity`, `mask_by_color`, `mask_grow`, `mask_topology`, `mask_shape`, `mask_by_group`, `extract_mask`, `extrude_mask`, `inset_mask`
- Face groups: `group_from_mask`, `group_auto`, `group_clear`, `hide_faces`
- Layers: `layer_add`, `surface_noise`, `layer_select`, `layer_set_strength`, `layer_delete`, `layer_apply`
- Files: `save_project`, `open_project`, `import_mesh`, `export_mesh`, `print_check`

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
- `shading`: string (clay | wax | pearl | bronze | gray | pbr | groups); default "clay"; The window's matcaps, or pbr for painted roughness and metalness under three lights.
- `view`: string (three_quarter | front | back | right | left | top | bottom); default "three_quarter"; Camera direction; three_quarter is front-right and a little above, as the window opens. Front looks along -z.
- `width`: integer; default 640; range 64 to 2048
- `yaw`: number; range -360 to 360; Degrees around the vertical axis from the front; overrides view.

Note: read-only.

## record_turntable

Render the scene turning once around the vertical axis and write it as a looping animated PNG (plays in browsers like a GIF, in full colour): a presentation of the model from every side. Rendered headless with get_view's shading; the window's View section downloads one.

- `background`: string (dark | light | transparent); default "dark"
- `fps`: integer; default 24; range 1 to 60
- `frames`: integer; default 48; range 8 to 180; Frames for one full turn.
- `ids`: array of integer; Only these objects; every visible object when omitted.
- `path`: string; required; Where to write the animation, ending in .png (an animated PNG that browsers play in a loop).
- `pitch`: number; default 15; range -89 to 89; Degrees above the horizon.
- `shading`: string (clay | wax | pearl | bronze | gray | pbr | groups); default "clay"
- `size`: integer; default 384; range 64 to 1024; Width and height in pixels.

## pick

Cast a ray (origin and direction in world space) and report the nearest visible object hit: its id, the point, the face normal and the distance. The window uses it to pivot the camera on a double-clicked point.

- `direction`: array of number; required
- `origin`: array of number; required

Note: read-only.

## set_visibility

Hide or show objects, isolate some (hiding the rest) or show all. Hidden objects are not drawn, picked, rendered by get_view or exported by default; they stay in the scene and in saved projects.

- `action`: string (hide | show | isolate | show_all); required; hide or show the ids; isolate shows only the ids (or the active object) and hides the rest; show_all shows everything.
- `folder`: string; Instead of ids: every object in this folder.
- `ids`: array of integer; The objects; the active object when omitted.

## set_folder

File objects under a named folder in the scene list (one level), or take them out with an empty name, so a scene of many parts stays readable; set_visibility can then hide, show or isolate a whole folder. Saved in projects. One undo step.

- `folder`: string; required; The folder name; an empty string takes them out of any folder.
- `ids`: array of integer; The objects to file; the active object when omitted.

## arrange_objects

Line objects up on an axis by their bounds' min, centre or max (at the first object's), or distribute them with equal gaps between the outermost two. Objects with subdivision levels move with all their levels. One undo step.

- `axis`: string (x | y | z); default "x"
- `ids`: array of integer; The objects to arrange; every object when omitted.
- `mode`: string (min | center | max | distribute); default "center"; min, center and max line the objects' bounds up on the axis (at the first object's); distribute spaces them evenly between the outermost two, with equal gaps between their bounds.

## get_history

Every undo step, oldest first, by name, and how many of them are done; the rest are undone and can be redone.

- no parameters

Note: read-only.

## history_go_to

Undo or redo until exactly `done` steps of get_history's list are done: jump back to any earlier state, or forward again.

- `done`: integer; required; range 0 to inf; How many steps of get_history's list should be done; earlier numbers undo, later ones redo.

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

## edit_primitive

Change a primitive's size, detail, height or thickness after making it, as long as nothing else has changed it since (no stroke, transform, remesh or level): get_scene shows primitive with its settings while it is still editable. Use it to settle proportions and resolution before sculpting. One undo step.

- `detail`: integer; range 0 to 2048; Subdivisions for sphere (0 to 9, default 5), divisions for cube and plane (default 16), segments for the round kinds (default 64).
- `height`: number; default 2; range -inf to 1000; Cylinder and cone height.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `size`: number; default 1; range -inf to 1000; Radius for sphere, uv_sphere, cylinder, cone and the torus ring; edge length for cube and plane.
- `thickness`: number; default 0.3; range -inf to 1000; Torus tube radius.

## skeleton_mesh

Build a base mesh from a skeleton of spheres ([x, y, z, radius]) joined by tapered limbs (links are index pairs), merged smoothly into one watertight, even mesh and made active: the quick way to block out a figure, animal or hand before sculpting. Unlinked spheres stay round. Raise resolution for thin limbs.

- `blend`: number; default 0.3; range 0 to 1; How softly limbs merge where they meet, as a fraction of the mean radius.
- `links`: array of array; Pairs of sphere indices joined by a tapered limb.
- `name`: string; Name for the new object; a default when omitted.
- `resolution`: integer; default 96; range 32 to 256; Voxel cells along the longest side.
- `spheres`: array of array; required; Joints as [x, y, z, radius].

## shadow_box

Build a base mesh from two or three outlines: the solid whose silhouette from the front is `front` (x, y), from the side `side` (z, y) and from above `top` (x, z), each extruded and intersected, with rounded edges. Fast blocking for props, heads in profile, vehicles, furniture: draw the side profile and the front shape and sculpt from there. A new watertight object.

- `blend`: number; default 0.05; range 0 to 0.5; Rounds the edges where the outlines meet, as a fraction of the solid's size.
- `front`: array of array; The outline seen from the front, as [x, y] points in world units (a closed polygon).
- `name`: string; Name for the new object; a default when omitted.
- `resolution`: integer; default 128; range 32 to 256; Voxel cells along the longest side.
- `side`: array of array; The outline seen from the side (the right view), as [z, y] points.
- `top`: array of array; The outline seen from above, as [x, z] points.

## text_mesh

Make solid 3D text: the words in a rounded single-stroke font (capitals, digits, common punctuation; newlines for more lines), `size` tall, `weight` thick and `depth` deep, centred on the origin and facing the front. One watertight object, ready to sculpt, boolean into a base (engraved or raised lettering on a plinth) or print.

- `depth`: number; range -inf to 100; How deep the letters are along z; a quarter of the size when omitted.
- `name`: string; Name for the new object; a default when omitted.
- `resolution`: integer; range 48 to 512; Voxel cells along the longest side; chosen so each stroke is about six cells wide when omitted.
- `size`: number; default 1; range -inf to 100; Capital height in world units.
- `text`: string; required; The words; a newline starts another line. Letters are drawn as capitals; the font has A-Z, 0-9, space and . , : - + ! ? ' /.
- `weight`: number; default 0.14; range 0.04 to 0.4; Stroke width as a fraction of the size.

## extrude_shape

Make a solid from flat vector outlines: closed [x, y] outlines seen from the front, outlines inside others cut holes, pushed out to `depth` with edges rounded by `round`. Logos, badges, cookie-cutter shapes, panels and plates. One watertight object.

- `depth`: number; range -inf to 100; Thickness along z; a fifth of the shape's width when omitted.
- `name`: string; Name for the new object; a default when omitted.
- `outlines`: array of array; required; Closed outlines as lists of [x, y] points in world units, seen from the front; an outline inside another cuts a hole (a ring, a letter O, a window).
- `resolution`: integer; range 48 to 512; Voxel cells along the longest side; 160 to 512 by the shape's detail when omitted.
- `round`: number; default 0; range 0 to 50; Rounds the edges by this many world units (at most half the depth).

## cut_shape

Hard-surface cut: extrude a closed outline straight through the object along `direction` (the way it is seen) and take away what it covers, or with keep inside keep only that, as one undo step. Lasso a notch, a vent, a window or a bevelled slot; trim a block to a profile. round softens the cut edges. The object is remeshed watertight at resolution; points from pick or a view's frame give the outline.

- `direction`: array of number; required; The way the outline is seen, from the eye into the scene; the cut goes all the way through the object along it.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `keep`: string (outside | inside); default "outside"; outside removes what the outline covers (a lasso cut, a hole, a notch); inside keeps only what it covers (trimming a block to a profile).
- `outline`: array of array; required; A closed outline as [x, y, z] world points, drawn as seen along direction (only their positions across the direction count).
- `resolution`: integer; default 192; range 32 to 320; Voxels along the longest side; the object is remeshed at this resolution.
- `round`: number; default 0; range 0 to 100; Rounds the cut edges by this many world units; 0 leaves them sharp.

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

Run one brush stroke along a path of points on the surface of an object, as one undo step. Brushes: draw (build along the area normal), clay (build up to a plane), inflate (along each vertex normal), pinch, crease, flatten, smooth, grab (move the vertices around the first point by the offset to the last) and mask (paint the mask; masked vertices resist every other brush); also relax, planar, trim, snake_hook, noise, paint and smudge (drag the paint colour along the stroke). alpha shapes every dab with a gray stamp (a built-in or one from alpha_load), turned by alpha_angle; stroke dots keeps the stamp upright (alpha_up), roll turns it along the stroke, and a single point stamps it once. invert carves, deflates or erases. Points closer than radius times spacing are merged into dabs; space points about radius * 0.15 apart for an even stroke. Symmetry mirrors across the object's x, y or z planes through the origin and repeats radially around y.

- `alpha`: string; A grayscale alpha shaping each dab (white pushes, black does not): none, square, noise, cracks, rings, star, stripes, dots, or a name loaded with alpha_load. A one-point stroke with an alpha is a stamp.
- `alpha_angle`: number; default 0; range -360 to 360; Turns the alpha, in degrees.
- `alpha_up`: array of number; The world direction the alpha's top faces (project it onto the surface); +y when omitted. The window sends the view's up.
- `brush`: string (draw | clay | inflate | pinch | crease | flatten | smooth | grab | mask | paint | relax | planar | trim | snake_hook | noise | smudge); default "draw"; paint colours the surface instead of moving it, using color, roughness and metalness. relax evens the triangles without shrinking the form (smooth also wears it down); planar flattens to the plane under the first dab of the stroke; trim cuts down only what stands above each dab's plane (invert fills hollows instead); snake_hook pulls the area under the first point along the path, so give points that leave the surface (add dyntopo to keep detail along the pull); noise roughens along the normal with value noise of noise_scale.
- `color`: ['string', 'array']; Paint brush: #rrggbb or [r, g, b] from 0 to 1.
- `dyntopo`: ['number', 'boolean']; Dynamic topology: split edges longer than this fraction of the radius (0.05 to 1; true means 0.25) under the brush as it moves, adding detail where you sculpt, and collapse edges under 0.4 of that length. Off when omitted. Not used by grab or mask, nor on objects with subdivision levels.
- `dyntopo_size`: number; range -inf to 10000; Constant detail: the target edge length in world units, whatever the radius. Turns dynamic topology on and overrides the dyntopo fraction.
- `falloff`: string (smooth | sphere | linear | sharp); default "smooth"
- `falloff_curve`: array of number; A drawn falloff instead of a named one: weights evenly spaced from the centre (first) to the rim (last), read linearly between. [1, 1, 1, 0] is a flat-topped brush with a hard edge; [0, 1, 0] a ring.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `invert`: boolean; default false; Carve instead of build, deflate, unpinch, erase mask.
- `lazy`: number; default 0; range 0 to 0.95; Lazy rope: the brush trails the path on a rope of lazy times three radii and moves only when it pulls taut, which steadies a line but stops short of the path's end by the rope length.
- `metalness`: number; range 0 to 1; Paint brush: paint this metalness.
- `noise_scale`: number; default 0; range 0 to 10000; Noise brush: feature size in world units; a fifth of the radius when 0.
- `points`: array of array; required; The stroke path as [x, y, z] or [x, y, z, pressure] points on or near the surface, in object space. Grab moves the vertices around the first point by the offset to the last.
- `pressure_gamma`: number; default 1; range 0.2 to 5; Pen pressure curve: pressure is raised to this power (above 1 needs a firmer press, below 1 a lighter one). Per-point pressure is the fourth number of a point.
- `pressure_size`: boolean; default false; Pressure scales the radius (down to a tenth).
- `pressure_strength`: boolean; default true; Pressure scales strength.
- `radius`: number; default 0.2; range -inf to 10000; World units; compare with the object's bounds from get_scene.
- `roughness`: number; range 0 to 1; Paint brush: paint this roughness (only the channels given are painted).
- `spacing`: number; default 0.15; range 0.02 to 2; Distance between dabs as a fraction of the radius.
- `spray`: number; default 0.6; range 0.05 to 2; Spray stroke: how far dabs scatter from the path, in radii.
- `strength`: number; default 0.5; range 0 to 1
- `stroke`: string (dots | roll | spray); default "dots"; dots keeps the alpha's orientation; roll turns it along the stroke's direction (chains, stitches, scales along a line); spray scatters each dab up to spray radii off the path, turned at random and at a random share of the strength (pores, gravel, speckle). Spray is seeded, so the same stroke repeats exactly.
- `symmetry`: object; fields radial, x, y, z

## alpha_load

Load a gray PNG as a named brush alpha: white stamps fully and black not at all. Give its name as alpha to sculpt_stroke (or pick it in the window's Brush section) to shape every dab with it, beside the built-in alphas square, noise, cracks, rings, star, stripes and dots. Alphas last for the engine session.

- `name`: string; required; The name brushes use for it in their alpha setting; loading the same name again replaces it.
- `path`: string; A PNG file on the engine's machine, instead of png_base64.
- `png_base64`: string; The alpha as a base64 PNG; white stamps fully, black not at all (colour is averaged to gray).
- `size`: integer; range 16 to 1024; The square resolution the alpha is resampled to (256 by default).

## vdm_stamp

Stamp a vector displacement map onto the surface at a point and normal (from pick), as one undo step. Unlike an alpha, a vector map moves the surface sideways and back over itself, so one stamp makes a curled horn, a hook or a mushroom with an overhanging cap. Built in: horn, hook, mushroom; vdm_capture makes more from a sculpted plane. radius is half the stamp's width, angle turns it about the normal, strength scales it (negative presses it in), and masked vertices are held. The surface needs enough vertices under the stamp for the shape to show: divide or remesh first if it is coarse.

- `angle`: number; default 0; range -360 to 360; Turns the stamp about the normal, in degrees (which way a horn leans).
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `normal`: array of number; required; The surface normal there (pick gives it); the stamp rises along it.
- `point`: array of number; required; The stamp's centre on the surface (pick gives one).
- `radius`: number; range -inf to 1000; Half the stamp's width in world units; a tenth of the object's largest side when omitted.
- `strength`: number; default 1; range -3 to 3; Scales every offset; negative presses the shape in.
- `vdm`: string; required; A built-in map (horn, hook, mushroom) or one made with vdm_capture.

## vdm_capture

Capture the sculpting on a plane as a named vector displacement map for vdm_stamp. Make the plane with add_primitive (kind plane, detail 64 or more), sculpt a shape in its middle with the edges left flat (strokes without dynamic topology, so its vertices stay a grid), then capture; the plane's up is the map's out and its x the map's across. Maps last for the engine session.

- `id`: integer; range 1 to inf; A plane made with add_primitive (kind plane) and then sculpted; the active object when omitted.
- `name`: string; required; The name vdm_stamp uses for it; capturing the same name again replaces it.

## paint_fill

Paint a whole object with a colour, roughness or metalness (only the channels given); masked areas are protected in proportion to the mask. For painting by hand use sculpt_stroke with brush paint. Paint follows subdivision levels, dyntopo, remesh, booleans and export (PLY, OBJ, GLB).

- `color`: ['string', 'array']; #rrggbb or [r, g, b] from 0 to 1.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `metalness`: number; range 0 to 1
- `roughness`: number; range 0 to 1

## paint_project

Paint an image onto the surface: from a view like a slide projector (only onto surfaces that face it and are not hidden behind others, fading at grazing angles), or tiled triplanar over the whole object. Goes into the active paint layer when there is one; masked areas are protected; the image's alpha and opacity set how strongly it replaces the paint. Vertex paint, so detail is limited by the mesh's resolution: subdivide first for fine images. One undo step.

- `height`: number; default 640; range 1 to 8192
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `mode`: string (view | triplanar); default "view"; view: projected from a camera like a slide projector, onto the surfaces facing it and not hidden behind others; triplanar: tiled over the whole surface from the three axis planes (fabric, stone, skin texture).
- `opacity`: number; default 1; range 0 to 1; How strongly the image replaces the paint (its own alpha multiplies this).
- `path`: string; A PNG file on the engine's machine, instead of png_base64.
- `pitch`: number
- `png_base64`: string; The image as a base64 PNG.
- `scale`: number; range -inf to 10000; triplanar: the image's size in world units; a quarter of the object's size when omitted.
- `view`: string (front | back | right | left | top | bottom | three_quarter); default "front"; view mode: the named camera, framed as get_view frames it, so the image lands where it would sit over a get_view picture of the same view and aspect.
- `view_proj`: array of number; view mode: a column-major view-projection matrix instead of a named view (the window sends its camera's); the image covers the whole view.
- `width`: number; default 640; range 1 to 8192
- `yaw`: number

## paint_layer_add

Add a paint layer above the active one and paint into it from now on: the paint brush, paint_fill and smudge go to the active paint layer (invert erases it), paint_clear empties it. Each layer has a blend (normal, multiply, screen, overlay, add) and an opacity, and is composited over the base and the layers below, so a pattern, dirt or tint can be changed or removed later without touching the rest. The first layer turns the existing paint into the base. A change of topology (remesh, dyntopo, subdivide) flattens the stack.

- `blend`: string (normal | multiply | screen | overlay | add); default "normal"
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `name`: string
- `opacity`: number; default 1; range 0 to 1

## paint_layer_set

Change a paint layer: name, blend, opacity, visibility, or move it up or down the stack. get_scene lists each object's paint_layers (bottom first) and active_paint_layer (null is the base).

- `blend`: string (normal | multiply | screen | overlay | add)
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `index`: integer; range 0 to inf; Layer number from the bottom (0); the active layer when omitted.
- `move`: string (up | down); Move the layer one place up or down the stack.
- `name`: string
- `opacity`: number; range 0 to 1
- `visible`: boolean

## paint_layer_select

Choose which paint layer brushes and fills paint into; index null paints the base under every layer. Not an undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `index`: ['integer', 'null']; range 0 to inf; Layer number from the bottom (0); for paint_layer_select, null paints the base.

## paint_layer_delete

Remove a paint layer and what was painted in it.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `index`: ['integer', 'null']; range 0 to inf; Layer number from the bottom (0); for paint_layer_select, null paints the base.

Note: removes or replaces (undoable).

## paint_layer_merge

Bake a paint layer, with its blend and opacity, into the layer below it (or the base), keeping how it looks.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `index`: ['integer', 'null']; range 0 to inf; Layer number from the bottom (0); for paint_layer_select, null paints the base.

## paint_layer_flatten

Bake every visible paint layer into plain paint and remove the stack (hidden layers are dropped).

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

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

## mask_cavity

Mask by curvature: creases and hollows (mode cavity) or ridges and peaks (mode peaks), with thresholds taken from the object's own range, so it suits both soft and detailed sculpts. Use it to paint grime into crevices, hold detail while smoothing, or sharpen ridges. One undo step.

- `contrast`: number; default 0.5; range 0 to 1; 0 masks broad curvature softly, 1 only the sharpest creases or ridges.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `mode`: string (cavity | peaks); default "cavity"; cavity masks creases and hollows; peaks masks ridges and raised edges.
- `op`: string (replace | add | subtract | intersect); default "replace"; How the new mask meets the current one: replace it, add (keep the larger), subtract, or intersect (keep the smaller).

## mask_by_color

Mask the vertices painted near a colour (within tolerance, soft-edged): sculpt or paint just the areas of one colour. Needs vertex paint. One undo step.

- `color`: ['string', 'array']; required; The painted colour to mask: #rrggbb or [r, g, b] from 0 to 1.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `op`: string (replace | add | subtract | intersect); default "replace"; How the new mask meets the current one: replace it, add (keep the larger), subtract, or intersect (keep the smaller).
- `tolerance`: number; default 0.15; range 0.001 to 1; How far a colour may differ (0 to 1 across the colour cube) and still be masked; the edge is soft over the last half.

## mask_grow

Grow the mask outward by rings of edges, or shrink it with negative steps. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `steps`: integer; default 1; range -50 to 50; Rings of edges to grow the mask by; negative shrinks it.

## mask_topology

Mask outward from a point by distance measured along the surface, not through space: a finger is masked without the finger beside it, an eyelid without the eye. Fades out toward the reach unless falloff is false. One undo step.

- `distance`: number; required; range -inf to 10000; How far to reach, measured along the surface's edges (not straight through space), in world units.
- `falloff`: boolean; default true; Fade the mask out toward the reach; false masks the whole reach fully.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `op`: string (replace | add | subtract | intersect); default "replace"; How the new mask meets the current one: replace it, add (keep the larger), subtract, or intersect (keep the smaller).
- `point`: array of number; required; Where to start, in object space; the nearest vertex is used.

## mask_shape

Mask everything inside a box, ellipse or lasso drawn on a view, through the whole object (back faces too). Points are in the coordinates of the get_view picture for the same view, width and height, so you can look with get_view and then mask what you see. Adds to the mask by default. One undo step.

- `height`: integer; default 640; range 1 to 8192
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `op`: string (replace | add | subtract | intersect); default "add"; How the new mask meets the current one: replace it, add (keep the larger), subtract, or intersect (keep the smaller).
- `pitch`: number; range -90 to 90; Degrees; overrides view, as in get_view.
- `points`: array of array; required; Picture coordinates as [x, y], x from 0 (left) to 1 (right), y from 0 (top) to 1 (bottom), in the picture get_view returns for the same view, width and height.
- `shape`: string (box | ellipse | lasso); default "box"; box and ellipse take two corner points; lasso takes the outline, three points or more.
- `view`: string (three_quarter | front | back | right | left | top | bottom); default "front"; The get_view camera the points refer to (framing every visible object, as get_view does by default).
- `view_proj`: array of number; Instead of a view: a column-major view-projection matrix (the window's camera); the points are then that view's picture coordinates.
- `width`: integer; default 640; range 1 to 8192; The picture's width, for its aspect; as in get_view.
- `yaw`: number; range -360 to 360; Degrees; overrides view, as in get_view.

## group_from_mask

Make the masked faces a new face group. Face groups divide a mesh into named regions (a head's eyes, lips and ears; a robot's panels) to mask, hide or isolate in one step. Groups survive strokes, masks and subdivision levels; remesh, booleans and dynamic topology drop them. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## group_auto

Replace an object's face groups with automatic ones: by angle (a new group wherever neighbouring faces turn by more than the angle, so a cube gets six) or by parts (each loose piece its own group). One undo step.

- `angle`: number; default 30; range 1 to 179
- `by`: string (angle | parts); default "angle"; angle starts a new group wherever neighbouring faces turn by more than `angle` degrees (the planes of a hard-surface model, the sides of a cube); parts gives each loose piece its own group.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## group_clear

Remove an object's face groups. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## mask_by_group

Mask one face group, named by number or by a point on it (get_scene reports how many groups an object has). Combines with the current mask by op. One undo step.

- `group`: integer; range 0 to inf; The face group to mask.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `op`: string (replace | add | subtract | intersect); default "replace"
- `point`: array of number; Or: mask the group of the face nearest this point.

## hide_faces

Hide, show or isolate faces chosen by face group or by the mask, invert what is hidden, or show all. Hidden faces are not drawn, cannot be hit by a stroke or a pick, and the vertices inside them do not move, so you can work on what is behind them. One undo step.

- `action`: string (hide | show | isolate | invert | show_all); required; hide or show the chosen faces, isolate them (hide the rest), invert what is hidden, or show every face.
- `by`: string (group | mask); default "group"; Choose faces by face group (group or point) or by the mask (faces whose corners are at least half masked).
- `group`: integer; range 0 to inf
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `point`: array of number; Choose the group of the face nearest this point.

## subdivide

Subdivide an object's mesh, each level multiplying the triangles by four (Loop subdivision, which rounds the shape; smooth false splits flat). The mask carries over. This replaces the mesh in one step; multires_divide keeps the lower levels to step back to.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `levels`: integer; default 1; range 1 to 4; Each level multiplies the triangles by four.
- `sharp_border`: boolean; default false; Keep open borders where they are while the rest smooths (a plane keeps its corners, a cut shell its rim).
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

## lattice_deform

Reshape an object with a lattice: a box of control points around its bounds (2 to 6 per axis, 3 by default); move some of them and the surface follows smoothly (free-form deformation), held by the mask. Good for broad proportion changes: widen a jaw, push a skull back, bulge a belly, squash a whole prop. One undo step; the reply gives the lattice's box.

- `divisions`: array of integer; default [3, 3, 3]; Control points along x, y and z. Point [i, j, k] starts at min + (i / (nx - 1), j / (ny - 1), k / (nz - 1)) * size of the object's bounds grown by 2 percent of their largest side.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `moves`: array of object; required

## merge_objects

Join several objects into the first one's mesh (the parts stay separate shells; use boolean union to fuse them).

- `ids`: array of integer; required; Objects to join into the first one, which keeps its name.

## split_parts

Split an object into one object per connected part of its mesh.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## array_object

Repeat an object in a line (offset per copy) or around an axis (radial, evenly over 360 degrees or over an angle): fence posts, teeth, petals, gear teeth, spines. Copies become separate objects, or one merged object with merge. They are copies, not live instances: editing the original later does not change them.

- `align`: boolean; default true; Curve: turn each copy so the original's +x follows the path.
- `angle`: number; default 360; range -360 to 360; Radial: the total turn in degrees; 360 spaces the copies evenly around the full circle.
- `axis`: string (x | y | z); default "y"; Radial: the axis the copies turn around (through center).
- `center`: array of number; Radial: the point the axis passes through; the world origin when omitted.
- `count`: integer; default 4; range 2 to 200; Copies in all, the original included.
- `id`: integer; range 1 to inf; The object to repeat; the active object when omitted.
- `linked`: boolean; default false; Linked copies (instances) follow the original: sculpt, paint or remesh it and every copy updates after the change. Changing a copy directly unlinks it.
- `merge`: boolean; default false; Join the copies with the original into one object instead of adding separate objects.
- `mode`: string (linear | radial | mirror | curve); default "linear"; linear and radial repeat count copies; mirror adds one copy reflected across the plane square to axis through center; curve places count copies evenly along points.
- `offset`: array of number; Linear: the step from one copy to the next, in world units; one bounds width along x when omitted.
- `points`: array of array; Curve: the path, as [x, y, z] points; each copy's bounds centre sits on it.

## insert_mesh

Place a primitive, or a copy of another object, at a point on a surface, standing along the normal there and sunk in by a fraction of its height: eyes, horns, bolts, buttons. Use pick to get the point and normal under a view direction. The new object becomes active.

- `kind`: string (sphere | cube | cylinder | cone | torus); default "sphere"; A primitive to insert, unless source_id is given.
- `normal`: array of number; The surface normal there; the new mesh stands along it. Up when omitted.
- `point`: array of number; required; Where to place it, usually a point on a surface (pick gives one).
- `sink`: number; default 0.25; range -1 to 1; How far it sinks into the surface, as a fraction of its height (0 sits on top, 0.5 half buried).
- `size`: number; default 0.2; range -inf to 10000; The inserted mesh's largest dimension, in world units.
- `source_id`: integer; range 1 to inf; Insert a copy of this object instead (a horn, a bolt, an eye).

## boolean

Union, difference (a minus b) or intersection of two objects through a voxel grid, giving one watertight object; the inputs are replaced unless keep_inputs. Detail finer than a voxel is smoothed, so raise resolution for sharp results. Both inputs should be closed meshes (mesh_check).

- `a`: integer; required; range 1 to inf; The object to keep or cut from.
- `b`: integer; required; range 1 to inf; The object to add, cut away or intersect with.
- `blend`: number; default 0; range 0 to 100; Fillets the seam over this many world units: a union fuses the two with a soft web where they meet (an arm into a shoulder, a horn into a head), a difference leaves a rounded cut edge. 0 is a sharp boolean.
- `keep_inputs`: boolean; default false; Keep a and b instead of replacing them with the result.
- `op`: string (union | difference | intersection); default "union"
- `resolution`: integer; default 160; range 16 to 320; Voxels along the longest side of both objects together; higher keeps sharper detail.

## layer_add

Add a sculpt layer to an object and make it the one strokes record into. A layer stores the strokes made while it is active as offsets, so its strength can later fade, exaggerate or invert them.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `name`: string

## surface_noise

Cover the object with noise along its normals (skin pores, stone, bark, hammered metal): a pattern (value, ridged, turbulence, cells, or an alpha tiled from three sides), octaves, gain and a stretch per axis, held by the mask, in a new sculpt layer: change its strength with layer_set_strength to fade or invert it, layer_apply to bake it, layer_delete to remove it. One undo step.

- `alpha`: string; The alpha pattern's alpha: a built-in name or one loaded with alpha_load.
- `amount`: number; range -10000 to 10000; Height of the bumps along the normals in world units (negative sinks them); 0.4 percent of the object's size when omitted.
- `gain`: number; default 0.5; range 0.1 to 0.9; How strongly each finer layer shows.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `name`: string; default "Noise"
- `octaves`: integer; default 3; range 1 to 8; Layers of finer detail on top of the base size.
- `pattern`: string (value | ridged | turbulence | cells | alpha); default "value"; value: soft fractal bumps (skin, stone); ridged: sharp crests (rock, bark, veins); turbulence: creased troughs (wrinkles, erosion); cells: rounded cells (scales, hammered metal, craters with a negative amount); alpha: an alpha tiled over the surface from three sides (fabric, tiles, a pattern loaded with alpha_load).
- `scale`: number; range -inf to 10000; Feature size in world units; 2 percent of the object's size when omitted.
- `seed`: integer; default 0; range 0 to 1000000; Another seed gives another pattern.
- `stretch`: array of number; Feature size multipliers along x, y and z (stretch wood grain or strata along one axis).

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
