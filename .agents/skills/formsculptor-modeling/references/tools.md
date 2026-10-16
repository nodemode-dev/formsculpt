# formsculptor tools

Generated from the engine's command registry (formsculptor 0.39.0) by `./formsculptor agents sync`; do not edit by hand.

## Contents

- Engine: `get_engine_info`
- Scene: `get_scene`, `new_scene`, `edit_primitive`
- Viewing: `get_view`, `record_turntable`, `record_animation`, `record_history`, `pick`
- Objects: `set_visibility`, `set_folder`, `arrange_objects`, `add_primitive`, `skeleton_mesh`, `shadow_box`, `edit_build`, `text_mesh`, `extrude_shape`, `curve_tube`, `sample_curve`, `curve_ik`, `curve_from_border`, `curve_bridge`, `lathe`, `select_object`, `delete_object`, `cloth_drape`, `pose`, `grow_fibers`, `pose_joint`, `groom_fibers`, `duplicate_object`, `rename_object`, `transform_object`, `deform_object`, `warp_along_curve`, `lattice_deform`, `merge_objects`, `split_parts`, `array_object`, `insert_mesh`, `insert_along`, `boolean`, `selection_set_save`, `selection_set_restore`, `selection_set_delete`, `selection_set_list`, `channel_copy`, `channel_paste`, `channel_clear`
- History: `get_history`, `history_go_to`, `undo`, `redo`
- Painting: `set_material`, `paint_fill`, `paint_project`, `paint_layer_add`, `paint_layer_set`, `paint_layer_select`, `paint_layer_delete`, `paint_layer_merge`, `paint_layer_flatten`, `paint_clear`, `uv_unwrap`, `uv_move_point`, `get_uv_layout`, `uv_transform`, `uv_relax`, `uv_pack`, `uv_density`, `uv_equalize`, `bake_maps`
- Topology: `bevel_edges`, `cut_shape`, `retopo_face`, `insert_edge_loop`, `delete_faces`, `bridge`, `clip_plane`, `bevel_edge`, `subdivide`, `remesh_voxel`, `decimate`, `fill_holes`, `mesh_check`, `quad_remesh`, `project_detail`, `multires_divide`, `multires_set_level`, `multires_delete_higher`, `multires_delete_lower`, `multires_reconstruct`, `repair_mesh`
- Sculpting: `sculpt_stroke`, `alpha_load`, `vdm_stamp`, `vdm_capture`
- Files: `send_to_substance`, `substance_sync`, `save_project`, `open_project`, `import_mesh`, `export_mesh`, `bridge_send`, `bridge_sync`, `print_check`
- Masking: `mask_invert`, `mask_clear`, `mask_blur`, `mask_sharpen`, `mask_cavity`, `mask_by_color`, `mask_grow`, `mask_topology`, `mask_shape`, `mask_by_group`, `extract_mask`, `extrude_mask`, `inset_mask`
- Face groups: `group_from_mask`, `group_from_outline`, `group_paint`, `group_auto`, `group_clear`, `hide_faces`
- Layers: `layer_add`, `surface_noise`, `layer_select`, `layer_set_strength`, `layer_delete`, `layer_apply`

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

- `aperture`: number; default 0; range 0 to 1; Path render: depth of field lens size as a fraction of the framed size; 0 keeps everything sharp, 0.05 is a gentle blur.
- `background`: string (dark | light | transparent); default "dark"
- `bounces`: integer; default 4; range 1 to 8; Path render: how many times light may bounce.
- `focus`: number; range 0 to inf; Path render: distance from the camera that stays sharp, in world units; the framed centre when omitted.
- `frame`: string (all | active); default "all"; Fit the camera to all drawn objects or to the active one (others still drawn).
- `height`: integer; default 640; range 64 to 2048
- `ids`: array of integer; Only these objects; every visible object when omitted.
- `lights`: array of object; Path render: directional lights added to the key light (a rim or fill).
- `pass`: string (beauty | depth | normal | ao); default "beauty"; Path render: the lit image, or a compositing pass (depth near white to far black, world normal as colour, ambient occlusion).
- `pitch`: number; range -90 to 90; Degrees above the horizon; overrides view.
- `render`: string (preview | path); default "preview"; preview draws as the window does (fast); path is a path-traced production render: sky and sun light with soft shadows, light bouncing between surfaces, glossy metal and smooth paint, glowing and see-through paint (seconds to minutes; samples and bounces trade time for noise).
- `samples`: integer; default 32; range 1 to 1024; Path render: paths per pixel; 32 is a quick look, 256 a clean final image.
- `shading`: string (clay | wax | pearl | bronze | gray | pbr | groups); default "clay"; The window's matcaps, or pbr for painted roughness and metalness under three lights.
- `smooth`: integer; default 0; range 0 to 3; Dynamic subdivision: draw each object Loop-subdivided this many levels (the window's Smooth preview); the meshes are not changed.
- `sun`: boolean; default true; Path render: keep the window's key light.
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

## record_animation

Timeline: render keyframed camera moves (yaw, pitch, zoom) and sculpt layer strengths (morphs such as a smile or a blink) to a looping animated PNG, eased between keys. Keys are {time in seconds, yaw, pitch, zoom, layers: [{id, layer, strength}]}; a value a key leaves out carries on from the keys around it. The scene is left as it was. Use it for a camera fly-around, a reveal or a before-and-after morph.

- `background`: string (dark | light | transparent); default "dark"
- `ease`: string (smooth | linear); default "smooth"
- `fps`: integer; default 24; range 1 to 60
- `ids`: array of integer; Only these objects; every visible object when omitted.
- `keys`: array of object; required; Keyframes. Each value holds from its key to the next key that sets it, eased between them; values a key leaves out carry on from the keys around it.
- `path`: string; required; Where to write the animation, ending in .png (an animated PNG that browsers play in a loop).
- `shading`: string (clay | wax | pearl | bronze | gray | pbr | groups); default "clay"
- `size`: integer; default 384; range 64 to 1024; Width and height in pixels.

## record_history

Undo history movie: render the model at every step of the undo history from the start to now, one frame per step, to a looping animated PNG (optionally turning the camera across it), then leave the scene and history where they were. Shows how a sculpt was built up.

- `background`: string (dark | light | transparent); default "dark"
- `fps`: integer; default 4; range 1 to 30; Undo steps shown per second.
- `path`: string; required; Where to write the movie, ending in .png (an animated PNG).
- `shading`: string (clay | wax | pearl | bronze | gray | pbr | groups); default "clay"
- `size`: integer; default 384; range 64 to 1024
- `turn`: number; default 0; range -720 to 720; Degrees the camera turns over the whole movie, so the build is seen from around it.
- `view`: string (front | back | left | right | top | bottom | three_quarter); default "three_quarter"

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

## set_material

Set how light passes into and through an object in lit shading: subsurface scattering (skin, wax, jade, marble: light enters, picks up subsurface_color and glows through thin parts within subsurface_radius) and transmission with refraction (glass, water, resin, at an ior). Given values change; the rest stay. Saved in projects; one undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `ior`: number; range 1 to 2.5; Index of refraction: water 1.33, glass 1.5, diamond 2.4.
- `subsurface`: number; range 0 to 1; How much light enters and comes back out tinted: skin 0.3 to 0.6, wax and jade 0.8.
- `subsurface_color`: array of number; The tint light picks up inside, linear RGB (skin is a warm red).
- `subsurface_radius`: number; range -inf to 1000; How far light travels inside, in world units; thin parts such as ears glow when lit from behind.
- `transmission`: number; range 0 to 1; How much light passes through, refracted: 1 for clear glass or water.

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

## edit_build

Remake a tube (curve_tube), lathe or skeleton from changed settings while it has not been sculpted or otherwise changed since: a new curve or profile, single points moved by index (move) or given sharp, round, radius or twist (set_points), and for tubes the interp, radius_mode, sides, profile, twist, spiral, spiral_offset, cap and hole as well as the radius, segments, smoothing or closure. For a skeleton, add grows a new sphere linked to the nearest joint and drag moves the joint nearest a point (the ZSphere workflow), then the base mesh is remade. get_scene shows editable_build on objects that can still be edited. One undo step.

- `add`: object; Skeleton: add a sphere at point, linked to the nearest sphere (ZSphere style: click beside a joint to grow a limb).; fields point, radius
- `blend`: number; range 0 to 1; Skeleton: how softly limbs merge.
- `cap`: string (flat | round | none); Tube: the end caps.
- `closed`: boolean; Tube: close the curve into a ring.
- `drag`: object; Skeleton: move the sphere nearest from to to.; fields from, to
- `hole`: number; range 0 to 1000000; Tube: wall thickness of a hollow pipe; 0 for solid.
- `id`: integer; range 1 to inf; A tube, lathe or skeleton; the active object when omitted.
- `interp`: string (catmull_rom | bspline | linear); Tube or lathe: how the path runs between points.
- `links`: array of array; Skeleton: new links.
- `move`: array of object; Tube: move single curve points by index (to [x, y, z] or [x, y, z, radius]); lathe: move profile points (to [radius, y]); skeleton: replace spheres (to [x, y, z, radius]).
- `points`: array of value; Tube: a new curve, as [x, y, z], [x, y, z, radius] or objects {p, radius, twist, sharp, round}.
- `profile`: value; Lathe: a new outline, as [radius, y] or {p, sharp, round}. Tube: a cross-section preset or a closed polygon of [x, y] points.
- `radius`: number; range -inf to 1000000; Tube: the radius for points without their own.
- `radius_end`: number; range 0 to inf; Tube: the far-end radius for start_end.
- `radius_mode`: string (per_point | uniform | start_end); Tube: how radii are chosen.
- `resolution`: integer; range 32 to 256; Skeleton: voxel cells along the longest side.
- `segments`: integer; range 3 to 256; Steps around the tube or the lathe's axis.
- `set_points`: array of object; Tube or lathe: change flags of single points by index without touching their positions: sharp (corner), round (fillet radius on a sharp point), radius and twist in degrees (tube only). null clears a field.
- `sides`: integer; range 3 to 128; Tube: sides around the circle profile.
- `smooth`: integer; range 1 to 32; Tube: curve steps between points.
- `spheres`: array of array; Skeleton: new joints as [x, y, z, radius].
- `spiral`: number; range -1000 to 1000; Tube: turns of the cross-section around the path.
- `spiral_offset`: number; range -1000000 to 1000000; Tube: distance of the cross-section from the path.
- `twist`: number; range -36000 to 36000; Tube: total roll along the path in degrees.

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

## bevel_edges

Round every edge sharper than `radius`: convex edges (a box's corners and rims), concave ones (where parts meet) or both, while flat faces stay in place, as one undo step. A volumetric bevel for hard-surface pieces, booleans and shadow boxes; the object is remeshed watertight at resolution. The radius must span at least a voxel and a half.

- `edges`: string (convex | concave | both); default "convex"; convex rounds outer edges (a box's corners), concave fills inner ones (where a handle meets a cup), both does each.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `radius`: number; range -inf to 1000; The round's radius in world units; a twentieth of the object's largest side when omitted.
- `resolution`: integer; default 192; range 32 to 320; Voxels along the longest side; the object is remeshed at this resolution.

## cut_shape

Hard-surface cut: extrude a closed outline straight through the object along `direction` (the way it is seen) and take away what it covers, or with keep inside keep only that, as one undo step. Lasso a notch, a vent, a window or a bevelled slot; trim a block to a profile. round softens the cut edges. The object is remeshed watertight at resolution; points from pick or a view's frame give the outline.

- `direction`: array of number; required; The way the outline is seen, from the eye into the scene; the cut goes all the way through the object along it.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `keep`: string (outside | inside); default "outside"; outside removes what the outline covers (a lasso cut, a hole, a notch); inside keeps only what it covers (trimming a block to a profile).
- `outline`: array of array; required; A closed outline as [x, y, z] world points, drawn as seen along direction (only their positions across the direction count).
- `resolution`: integer; default 192; range 32 to 320; Voxels along the longest side; the object is remeshed at this resolution.
- `round`: number; default 0; range 0 to 100; Rounds the cut edges by this many world units; 0 leaves them sharp.

## curve_tube

Sweep a cross-section along a curve through the given points and make it active: hair strands, horns, tails, cables, straps, piping, ropes, springs, pipes. Points can be smooth or sharp corners, and a sharp point with round becomes a circular fillet; interp picks catmull_rom, bspline or linear; radius is per point, uniform or tapered start to end; profile is circle, square, triangle, star, flat or your own polygon; twist and spiral with spiral_offset make helices and rope; cap is flat, round or none; hole makes a hollow pipe. Remembered, so edit_build can change any of it later (set_points flips one point's sharp or round). Use sample_curve to see the exact path first.

- `cap`: string (flat | round | none); default "flat"; The ends of an open tube: flat disc, rounded dome, or left open.
- `closed`: boolean; default false; Join the last point back to the first (a ring) instead of capping the ends.
- `hole`: number; default 0; range 0 to 1000000; Above 0 makes a hollow pipe with this wall thickness (world units); the ends show the wall.
- `interp`: string (catmull_rom | bspline | linear); default "catmull_rom"; How the path runs between points; sharp points split it into sections interpolated on their own.
- `name`: string; Name for the new object; a default when omitted.
- `points`: array of value; required; The curve as [x, y, z] or [x, y, z, radius] points, or objects {p: [x, y, z], radius, twist (degrees), sharp, round}. sharp makes the point a corner; sharp with round above 0 turns the corner into a circular fillet of that world radius. A point's radius overrides radius.
- `profile`: value; default "circle"; The cross-section: a preset, or a closed polygon of [x, y] points (or {p, sharp, round}) in units of the radius; points default to corners, sharp false makes one smooth.
- `radius`: number; default 0.1; range 0 to inf; Tube radius where a point gives none (and the start radius for radius_mode start_end).
- `radius_end`: number; range 0 to inf; start_end: the radius at the far end; radius when omitted.
- `radius_mode`: string (per_point | uniform | start_end); default "per_point"; per_point uses each point's radius; uniform uses radius everywhere; start_end tapers linearly by length from radius to radius_end.
- `segments`: integer; default 16; range 3 to 128; Sides around the tube (the circle profile).
- `sides`: integer; range 3 to 128; Same as segments; wins when both are given.
- `smooth`: integer; default 4; range 1 to 32; Rings per curve span (also the finish of fillet arcs).
- `spiral`: number; default 0; range -1000 to 1000; Turns the cross-section and its spiral_offset make around the path from start to end.
- `spiral_offset`: number; default 0; range -1000000 to 1000000; Distance of the cross-section's centre from the path; with spiral turns it makes a helix, rope strand or spring.
- `twist`: number; default 0; range -36000 to 36000; Total roll of the cross-section along the path, in degrees (adds to each point's twist).

## sample_curve

Evaluate a curve without changing the scene: control points (smooth, or sharp corners with optional round fillet radius) go in, a polyline comes out with positions, unit tangents, radii, twists (degrees), arc-length fractions t, the total length and the indices of crisp corners. interp is catmull_rom (centripetal, through the points), bspline or linear; closed loops the curve; per_span sets samples between points, or spacing resamples evenly by arc length. This is the one place curve corners and fillets are computed: draw the result as the preview, and curve_tube, array curve mode and quad_remesh guides follow the same curve.

- `closed`: boolean; default false; Loop the last point back to the first. The result does not repeat the first sample at the end.
- `interp`: string (catmull_rom | bspline | linear); default "catmull_rom"; catmull_rom passes through every smooth point (centripetal, no cusps); bspline approximates with a smoother curve; linear is a polyline. Sharp points split the curve into sections that are interpolated on their own.
- `per_span`: integer; default 8; range 1 to 64; Samples per span between control points (also sets how finely fillet arcs are cut).
- `points`: array of value; required; The curve's control points: [x, y, z], [x, y, z, radius], or objects {p, radius, twist, sharp, round}. A sharp point is a corner; sharp with round above 0 becomes a circular fillet of that world radius.
- `radius`: number; default 1; range 0 to inf; Radius for points that give none.
- `spacing`: number; range 0 to inf; When given, resample by arc length instead: samples this many world units apart (corners always kept).

Note: read-only.

## curve_ik

Drag one control point of a curve to a target and let the rest follow, without changing the scene: the points are a chain of fixed-length segments (the lengths between neighbours are kept), solved with FABRIK. pin_start keeps the first point fixed (default on), pin_end keeps the last fixed; with neither pinned the chain trails behind the dragged point like a rope. If the target is out of reach the chain stretches straight toward it as far as it can and reached is false. Points use the same formats as sample_curve and come back with new positions and everything else (radius, twist, sharp, round) unchanged, plus error (distance left to the target) and the segment lengths. A closed curve is solved as open. Feed the result to curve_tube, sample_curve or edit_build.

- `index`: integer; required; range 0 to inf; Which point to drag, counting from 0.
- `iterations`: integer; default 32; range 1 to 200; Most FABRIK passes to run.
- `pin_end`: boolean; default false; Keep the last point where it is.
- `pin_start`: boolean; default true; Keep the first point where it is.
- `points`: array of value; required; The chain's control points, in the same formats as sample_curve. They are treated as an open chain of fixed-length segments; a closed curve is solved as open, so give the points without repeating the first at the end.
- `target`: array of number; required; [x, y, z] to drag that point to.

Note: read-only.

## curve_from_border

Turn the border of a set of faces into curve control points, without changing the scene. The set is the faces in one face group (source group, with group giving the id) or the faces whose three vertices have a mean mask of 0.5 or more (source mask). A border edge has exactly one face inside the set, so open mesh edges of the set count too. Border edges are chained into closed loops and open chains, longest first; loop picks one (0 is the longest). The chosen border is cut into points control points (default 16, 3 to 256) evenly spaced by arc length, each optionally lifted along the surface normal by offset. Returns points as [{p: [x, y, z]}, ...], closed, loops (how many borders were found) and length (the arc length of the chosen border). Refuses with no_border when the set is empty or covers a whole closed mesh. Feed the points to sample_curve, curve_tube, curve_ik or edit_build, passing closed when the border is closed.

- `group`: integer; range 0 to inf; The face group id; required when source is group.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `loop`: integer; default 0; range 0 to inf; Which border to take, counting from 0 in order of length (0 is the longest).
- `offset`: number; default 0; Lift each point along the surface normal by this distance, in world units.
- `points`: integer; default 16; range 3 to 256; How many control points to place, evenly spaced by arc length.
- `source`: string (group | mask); default "group"; Which faces make the set: those in one face group, or those whose three vertices have a mean mask of 0.5 or more.

Note: read-only.

## curve_bridge

Loft a cross-section loop along a path into a new object (ZBrush's Curve Bridge and Snake Curve) as one undo step. from is the start loop: the longest border of a face set, either one face group (source group, with group) or the masked faces (source mask). With to (same shape) the bridge ends on that second loop; the path is the straight line between the two loop centres, or the curve through points (same formats as sample_curve). Without to the end section is the start loop scaled about its centre by end_scale (default 0.3), and the path runs along the loop's average normal for length world units (default four times the loop's mean radius) unless points is given: a tapering snake out of a mask or group border. Both loops are cut into samples points (8 to 256, default 48), made to correspond (same winding, start index turned to the nearest fit), and swept along the path on rotation-minimising frames in sections rings (4 to 512, default 48), so the bridge meets each loop exactly at its ends. The result is an open triangle tube with both ends left open. offset lifts the loops along the surface normal. Returns the new object plus sections and samples. Refuses with no_border when a loop is missing and with bad_arguments when the border is open, since a bridge needs closed loops.

- `end_scale`: number; default 0.3; range 0.001 to 100; Without to: the end section is the start loop scaled about its centre by this factor.
- `from`: object; required; The start loop: the longest border of this face set.; fields group, source
- `id`: integer; range 1 to inf; The source object; the active object when omitted.
- `length`: number; range -inf to 1000000; Without to and points: how far to extrude along the normal, in world units. Four times the loop's mean radius when omitted.
- `name`: string; Name for the new object; a default when omitted.
- `offset`: number; default 0; Lift each loop along the surface normal by this distance, in world units.
- `points`: array of value; The path between the loops, in the same point formats as sample_curve. Without it the path is the straight line between the two loop centres, or without to a straight line along the start loop's average normal.
- `samples`: integer; default 48; range 8 to 256; Points around each loop.
- `sections`: integer; default 48; range 4 to 512; Rings along the path.
- `to`: object; The end loop: the longest border of this face set. Without it the end section is the start loop scaled by end_scale (a tapering extrusion).; fields group, source

## lathe

Spin an outline of [radius, y] points around the vertical axis into a closed solid and make it active: vases, bottles, cups, columns, turned legs, chess pieces. Outline points are corners by default; give {p: [radius, y], sharp, round} to round a corner with a fillet of that radius or sharp false to make a point a smooth knot.

- `interp`: string (catmull_rom | bspline | linear); default "catmull_rom"; How the outline runs through smooth points.
- `name`: string; Name for the new object; a default when omitted.
- `profile`: array of value; required; The outline as [radius, y] points from bottom to top, spun around the y axis; or objects {p: [radius, y], sharp, round}. Points are corners by default; sharp false makes one a smooth knot, and a sharp point with round above 0 gets a circular fillet of that radius. A radius of 0 closes to a point; open ends are capped flat.
- `segments`: integer; default 48; range 3 to 256; Steps around the axis.
- `smooth`: integer; default 6; range 1 to 32; Rows per outline span between smooth points and around fillets.

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
- `emissive`: number; range 0 to 1; Paint brush: paint this glow in the paint colour.
- `falloff`: string (smooth | sphere | linear | sharp); default "smooth"
- `falloff_curve`: array of number; A drawn falloff instead of a named one: weights evenly spaced from the centre (first) to the rim (last), read linearly between. [1, 1, 1, 0] is a flat-topped brush with a hard edge; [0, 1, 0] a ring.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `invert`: boolean; default false; Carve instead of build, deflate, unpinch, erase mask.
- `lazy`: number; default 0; range 0 to 0.95; Lazy rope: the brush trails the path on a rope of lazy times three radii and moves only when it pulls taut, which steadies a line but stops short of the path's end by the rope length.
- `metalness`: number; range 0 to 1; Paint brush: paint this metalness.
- `noise_scale`: number; default 0; range 0 to 10000; Noise brush: feature size in world units; a fifth of the radius when 0.
- `opacity`: number; range 0 to 1; Paint brush: paint this opacity (1 solid, 0 clear).
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
- `emissive`: number; range 0 to 1; How much the surface glows in its own colour, unlit (lamps, lava, eyes).
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `metalness`: number; range 0 to 1
- `opacity`: number; range 0 to 1; 1 is solid; lower lets what is behind show through (drawn dithered in the window, as alpha in GLB and baked colour maps).
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

Give an object UV coordinates: faces are grouped into charts by the direction they face, each chart is laid flat (by default with least-squares conformal maps, which keep angles; method project is a straight box projection) and the charts are packed into the unit square keeping their relative sizes. Reports stretch, the area-weighted mean and worst angle distortion (1 is none). Needed before bake_maps; topology changes (remesh, dyntopo, subdivide, booleans) clear the UVs. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `method`: string (conformal | project); default "conformal"; conformal flattens each chart by least-squares conformal maps (angles kept, little stretch); project is the straight box projection (fastest, stretches faces at an angle).
- `seams`: string (direction | groups); default "direction"; Where charts are cut: direction groups faces by the way they face; groups cuts along face group borders, so groups painted with the Group brush or made by group_auto place the seams (a group that is a closed shell is split further by direction).

## cloth_drape

Simulate an object as cloth falling under gravity onto the other objects (or chosen colliders, and an optional floor) until it settles: edges keep their length, the cloth cannot pass through what it lands on, and masked vertices stay pinned (hang a curtain by masking its top edge). Typical: add_primitive plane with detail 64 above a model, then cloth_drape. One undo step.

- `colliders`: array of integer; Objects the cloth lands on and cannot pass through; every other visible object when omitted.
- `floor`: number; A floor height the cloth rests on; none when omitted.
- `gravity`: array of number; Direction and strength of the pull in world units per second squared; [0, -9.8, 0] when omitted.
- `id`: integer; range 1 to inf; The cloth: usually a plane (add_primitive kind plane, detail 48 to 96) placed above what it should fall on; the active object when omitted.
- `steps`: integer; default 180; range 1 to 2000; Simulation steps of 1/60 s; more lets it fall further and settle.
- `stiffness`: number; default 0.9; range 0.05 to 1; How firmly edges keep their length (1 is stiff cotton, lower is stretchy).
- `thickness`: number; range -inf to 100; How far the cloth stays off colliders; 1 percent of the cloth's size when omitted.

## pose

Pose a part as with ZBrush's Transpose: mask what must stay (the body), and the unmasked part (an arm) turns about a joint pivot by an angle around an axis; partly masked vertices turn partly, and falloff spreads the bend smoothly near the joint. One undo step.

- `angle`: number; required; range -360 to 360; Degrees to turn, counter-clockwise about the axis.
- `axis`: array of number; The hinge direction; [0, 0, 1] when omitted.
- `falloff`: number; default 0; range 0 to 100; Spread the bend over this distance from the pivot (world units), so the joint bends smoothly instead of hinging.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `pivot`: array of number; required; The joint to turn about (a shoulder, an elbow, a neck), in world space.

## grow_fibers

Grow hair, fur or grass as fibers from an object's masked area (or all of it): a new object of thin tapered tubes placed at random over the surface, leaving along the normal and drooping toward gravity, with optional clumping into locks and a colour. One undo step.

- `clump`: number; default 0; range 0 to 1; How much tips gather toward their neighbours' tips (locks of hair).
- `color`: ['string', 'array']; Paint the fibers this colour (#rrggbb or [r, g, b]).
- `count`: integer; default 1500; range 1 to 50000
- `droop`: number; default 0.4; range 0 to 1; How much fibers bend toward gravity (0 stands straight out, 1 hangs).
- `id`: integer; range 1 to inf; The object to grow from; the active object when omitted. Fibers grow from its masked area, or everywhere without a mask.
- `length`: number; range -inf to 1000; Fiber length; 15 percent of the object's size when omitted.
- `radius`: number; range -inf to 100; Root radius (fibers taper to a third at the tip); a fortieth of the length when omitted.
- `seed`: integer; default 1; range 0 to inf
- `segments`: integer; default 5; range 2 to 16; Points along each fiber.

## retopo_face

Manual retopology: add one quad (four corners) or triangle (three) to a retopology object, the corners snapped onto the sculpt's surface and joined to existing corners within the weld distance, facing out. Build a clean quad mesh over a sculpt face by face (eye loops, mouth loops, joints), then multires_divide it and project_detail from the sculpt. The first call makes the object; pass its id as into after that.

- `into`: integer; range 1 to inf; The retopology object to add to (from an earlier call's id); a new one when omitted.
- `points`: array of array; required; Three or four corners of the new face, in order around it, near the target's surface.
- `target`: integer; required; range 1 to inf; The sculpt to retopologize; the points snap onto its surface.
- `weld`: number; range -inf to 100; Corners within this distance of an existing vertex reuse it, so faces join; 1.5 percent of the target's size when omitted.

## insert_edge_loop

Insert an edge loop into a quad mesh: the ring of quads crossing the edge nearest point is split down the middle all the way around (or to an open border), for more detail where a joint bends or a crease runs. Needs quads (quad_remesh or retopo_face first). One undo step.

- `id`: integer; range 1 to inf; A quad mesh (quad_remesh, a retopology object or a quad import); the active object when omitted.
- `point`: array of number; required; A point on or near the edge the new loop crosses (from pick); the loop runs across it, around the ring of quads.

## delete_faces

Delete the hidden faces (hide_faces) or the fully masked ones, opening the mesh there, and drop vertices left unused: to open holes for bridge, or remove a part. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `which`: string (hidden | masked); default "hidden"; Delete the hidden faces (hide_faces), or the faces whose corners are all masked.

## bridge

Bridge two open borders with a band of quads (segments rows), joining them into one surface: two tube ends into a sleeve, a hole to another hole. The borders need the same number of vertices; with more than two borders, near_a and near_b choose them. One undo step.

- `id`: integer; range 1 to inf; An object with open borders (merge two open objects, or open holes with delete_faces); the active object when omitted.
- `near_a`: array of number; A point near the first border, when the object has more than two.
- `near_b`: array of number; A point near the second border.
- `segments`: integer; default 4; range 1 to 64; Rows of quads along the bridge.

## send_to_substance

Write a Substance 3D Painter package to a folder: the mesh to texture as OBJ with UVs (level 0 when the object has subdivision levels), the high-resolution mesh to bake detail from, any baked maps under Painter's names, and a README with the steps. The object needs UVs (uv_unwrap).

- `folder`: string; required; Where to write the package (created if missing).
- `id`: integer; range 1 to inf; The object to texture; the active object when omitted. It needs UVs (uv_unwrap).

## pose_joint

Pose an object made by skeleton_mesh by its own skeleton: turn one joint (by index, or the joint nearest point) about an axis, and every bone beyond it follows with the mesh around it, weighted automatically by distance to each bone so the bend blends across the joint (softness). Works after sculpting; detail rides along. One undo step.

- `angle`: number; required; range -360 to 360; Degrees to turn.
- `axis`: array of number; default [0, 0, 1]; The hinge axis.
- `id`: integer; range 1 to inf; An object made by skeleton_mesh (sculpted or not); the active object when omitted.
- `joint`: integer; range 0 to inf; The joint (sphere index) to turn; everything beyond it in the skeleton follows.
- `point`: array of number; Choose the joint nearest this point instead (a click).
- `softness`: number; default 0.6; range 0.05 to 4; How widely the bend blends across the joint, as a fraction of the joint's radius.

## groom_fibers

Groom fibers made by grow_fibers along a stroke: comb bends them toward the stroke's direction, cut trims them where it passes, curl twists them into spirals, lengthen grows them; strands keep their segment lengths when combed or curled. One undo step.

- `id`: integer; range 1 to inf; A fibers object from grow_fibers; the active object when omitted.
- `mode`: string (comb | cut | curl | lengthen); required; comb bends fibers along the stroke, cut trims them where the stroke passes, curl twists them into spirals, lengthen grows them.
- `points`: array of array; required; The stroke as world points over the fibers (comb follows its direction).
- `radius`: number; range 0 to inf; Reach of the stroke in world units; a third of a fiber's length when omitted.
- `strength`: number; default 0.6; range 0 to 1

## substance_sync

Bring textures exported from Substance 3D Painter back: [name]_BaseColor.png in the package folder (and _Roughness and _Metallic when beside it) is sampled at each vertex's UV into vertex paint, and the colour map replaces the baked colour map for GLB export. The round trip to send_to_substance. One undo step.

- `color`: string; The base colour PNG; [name]_BaseColor.png in folder when omitted.
- `folder`: string; required; The folder send_to_substance wrote and Painter exported back into.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## clip_plane

Cut an object flat along a plane with an exact boolean: keep removes the side the normal points to (ZBrush ClipCurve, a planar cut for hard-surface parts), slice keeps both halves as two capped objects. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `mode`: string (keep | slice); default "keep"; keep removes the far side (ZBrush ClipCurve); slice keeps both halves as two capped objects.
- `normal`: array of number; required; The plane's normal: the side it points to is cut away.
- `point`: array of number; A point on the cutting plane; the object's centre when omitted.

## bevel_edge

Bevel the straight run of sharp edges nearest a point: chamfer cuts a flat face and round cuts a fillet of the given width into both faces, exactly (the cut is a boolean with a prism, so the rest of the mesh is untouched). Use round_edges for every edge at once. One undo step.

- `angle`: number; default 30; range 5 to 170; Faces meeting at more than this many degrees count as a sharp edge.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `mode`: string (chamfer | round); default "chamfer"; chamfer cuts one flat face; round cuts a fillet.
- `point`: array of number; required; A point on or near the edge to bevel (a click on the model); the straight run of sharp edges through it is beveled.
- `segments`: integer; default 6; range 2 to 32; Round: steps across the fillet.
- `width`: number; range 0 to inf; How far the bevel reaches into each face, in world units; 4 percent of the object's size when omitted.

## uv_move_point

Drag UV vertices by hand: the UV point nearest at goes to to, or with radius every corner within it follows with a smooth falloff (kept to the island under at). For fixing stretch that uv_transform's whole-island moves cannot. Clears baked maps. One undo step.

- `at`: array of number; required; The UV point [u, v] to grab.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `island`: boolean; default true; Keep the grab to the island under at.
- `radius`: number; default 0; range 0 to 1; UV corners within this distance follow with a smooth falloff; 0 moves only the corners at the nearest point (one UV vertex).
- `to`: array of number; required; Where it goes.

## get_uv_layout

Picture an unwrapped object's UV layout (each island in its own colour, v up) and list its islands' bounds, the share of the square in use and the angle distortion. Give at [u, v] to highlight the island there. Use it to look before moving islands with uv_transform.

- `at`: array of number; A UV point [u, v] (0 to 1, v up); the island under it is drawn brighter and reported as selected.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `island`: integer; range 0 to inf; An island's number to draw brighter, instead of at.
- `size`: integer; default 512; range 64 to 2048; Width and height of the picture in pixels.

Note: read-only.

## uv_transform

Move, turn or scale one UV island, chosen by a UV point on it (at) or its number from get_uv_layout: scale and rotate about its centre, then translate. Scale an island up to give its part of the model more texture resolution, then uv_pack. Clears baked maps. One undo step.

- `at`: array of number; A UV point [u, v] on the island to move (get_uv_layout lists island bounds).
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `island`: integer; range 0 to inf; The island's number from get_uv_layout, instead of at.
- `rotate`: number; range -360 to 360; Turn counter-clockwise by degrees about the island's centre.
- `scale`: number; range -inf to 100; Scale about the island's centre.
- `translate`: array of number; Move by [du, dv].

## uv_relax

Smooth an unwrapped object's UVs to reduce stretch. Each point moves toward the average of its neighbours for a number of passes (iterations, strength). Mode conformal weights neighbours by the cotangents of the 3D triangles so UV angles follow the 3D shape; smooth is a plain average. Island borders stay put (keep_border) and so do the vertices in pinned; give island [a vertex index] to relax only the island holding that vertex. Reports the angle distortion before and after (1 is none) and how many vertices moved. Clears baked maps. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `island`: integer; range 0 to inf; A vertex index: relax only the island or islands that vertex belongs to.
- `iterations`: integer; default 30; range 1 to 500; Relaxation passes.
- `keep_border`: boolean; default true; Keep the points on an island's border (seams and open edges) where they are.
- `mode`: string (smooth | conformal); default "conformal"; smooth is a plain average over the mesh connectivity; conformal weights neighbours by the cotangents of the 3D triangles, which brings the UV angles toward the 3D shape.
- `pinned`: array of integer; Vertex indices whose UVs stay put.
- `strength`: number; default 0.5; range 0 to 1; How far each point moves toward the average of its neighbours on each pass.

## uv_pack

Pack an object's UV islands back into the unit square keeping their shapes, turns and relative sizes, after islands were moved or scaled. Clears baked maps. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

## uv_density

Read the texel density of each UV island in pixels per world unit for a texture of a given size (texture, default 2048): the island's faces, density, UV area and surface area, plus the area-weighted mean, the lowest, the highest and their ratio (spread). A spread of 1 means every part of the model gets the same texture detail. Read-only; use uv_equalize to even it out.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `texture`: integer; default 2048; range 64 to 16384; Texture width and height in pixels that the density is measured for.

Note: read-only.

## uv_equalize

Scale each UV island about its centre so every island has the same texel density: target in pixels per world unit for a 2048 pixel texture, or the area-weighted mean when omitted. Packs the islands into the unit square afterwards unless pack is false, which keeps their sizes relative to each other but can change all densities by one factor. Returns the density mean, min, max and spread before and after. Clears baked maps. One undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `pack`: boolean; default true; Pack the islands into the unit square afterwards so they do not overlap. Packing keeps the islands' sizes relative to each other, so it can change all densities by the same factor.
- `target`: number; range 0 to inf; Density to give every island, in pixels per world unit for a 2048 pixel texture; the area-weighted mean of the islands when omitted.

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

## group_from_outline

Slice Curve: put the faces inside a screen-space outline into a face group (a new one, or `group`), so a face group can be cut along a curve drawn on a view. A face is inside when the middle of the face falls inside the outline; the others keep their groups, and a mesh without groups starts with every face in group 0. Shape is lasso (the outline, three points or more) or box (two corners). Points are in the coordinates of the get_view picture for the same view, width and height, or give view_proj for the window's camera. With front_only true only faces turned toward the camera are taken, otherwise the outline goes through the whole object. Returns the group number used and how many faces went in; refuses with nothing_inside when no face is inside. One undo step.

- `front_only`: boolean; default false; Take only faces that face the camera; false takes every face inside the outline, through the whole object.
- `group`: integer; range 0 to inf; The group number to give the faces; one above the highest existing group when omitted.
- `height`: integer; default 640; range 1 to 8192
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `pitch`: number; range -90 to 90; Degrees; overrides view, as in get_view.
- `points`: array of array; required; The outline in picture coordinates as [x, y], x from 0 (left) to 1 (right), y from 0 (top) to 1 (bottom), in the picture get_view returns for the same view, width and height.
- `shape`: string (box | lasso); default "lasso"; box takes two corner points; lasso takes the outline, three points or more.
- `view`: string (three_quarter | front | back | right | left | top | bottom); default "front"; The get_view camera the points refer to (framing every visible object, as get_view does by default).
- `view_proj`: array of number; Instead of a view: a column-major view-projection matrix (the window's camera); the points are then that view's picture coordinates.
- `width`: integer; default 640; range 1 to 8192; The picture's width, for its aspect; as in get_view.
- `yaw`: number; range -360 to 360; Degrees; overrides view, as in get_view.

## group_paint

Group brush: paint the faces along a stroke into one face group (a new one, or `group`), as one undo step. points are [x, y, z] on the surface (pick gives them) and radius the brush's reach in world units. Use it to mark an eyelid, a panel or a seam by hand where group_auto's angle, parts, paint and uv rules do not fit.

- `group`: integer; range 0 to inf; Paint this existing group number; a new group when omitted.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `points`: array of array; required; The stroke over the surface as [x, y, z] points (pick gives them), in order.
- `radius`: number; required; range -inf to 1000; The brush radius in world units; faces whose middle lies within it of the stroke are painted.

## group_auto

Replace an object's face groups with automatic ones: by angle (a new group wherever neighbouring faces turn by more than the angle, so a cube gets six) or by parts (each loose piece its own group), by paint (connected faces of one painted colour) or by uv (each UV island). One undo step.

- `angle`: number; default 30; range 1 to 179
- `by`: string (angle | parts | paint | uv); default "angle"; angle starts a new group wherever neighbouring faces turn by more than `angle` degrees (the planes of a hard-surface model, the sides of a cube); parts gives each loose piece its own group; paint groups connected faces of one painted colour (within `tolerance`); uv gives each UV island its own group (from the object's UVs, or an automatic unwrap when it has none).
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `tolerance`: number; default 0.12; range 0.001 to 1; paint: how far apart two colours (0 to 1 per channel) may be and still share a group.

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
- `guides`: array of array; Guide curves as lists of [x, y, z] points or objects {p, sharp, round} (smoothed through catmull_rom, corners respected; drawn with the Curve stroke tool, or along a limb): the quads' edge loops are turned to run along their mean direction, as ZRemesher's curves steer it. Without guides the grid follows the world axes.
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

## warp_along_curve

Bend an object along a curve: its long axis (or the given axis) is laid onto the curve's arc length and each cross-section is carried along the curve with a twist-free frame, so a straight bar becomes a tail, a horn or a ring. stretch fits the object's length to the whole curve. Curve points use the same formats as sample_curve (sharp corners and fillets respected). Masked vertices are held in proportion to the mask. One undo step.

- `axis`: string (x | y | z); The object's axis that runs along the curve; its longest side when omitted.
- `closed`: boolean; default false
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `interp`: string (catmull_rom | bspline | linear); default "catmull_rom"
- `points`: array of value; required; The curve to follow, in the same point formats as sample_curve (sharp and round respected).
- `stretch`: boolean; default true; Fit the object's length to the curve's full length. Off keeps the object's length (in world units) from the curve's start, running on straight past the end.
- `twist`: number; default 0; range -36000 to 36000; Extra roll about the curve in degrees, spread over its length.

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

Repeat an object in a line (offset per copy) or around an axis (radial, evenly over 360 degrees or over an angle): fence posts, teeth, petals, gear teeth, spines. Copies become separate objects, or one merged object with merge. They are copies, not live instances: editing the original later does not change them. Also along a curve (curve mode with points). scale_start and scale_end ramp a uniform scale from the first copy to the last, and jitter adds a repeatable random offset, turn and scale to each copy from a seed (position in world units, rotation in degrees, scale as a fraction); both are refused in mirror mode, and linked copies keep the ramp and jitter.

- `align`: boolean; default true; Curve: turn each copy so the original's +x follows the path.
- `angle`: number; default 360; range -360 to 360; Radial: the total turn in degrees; 360 spaces the copies evenly around the full circle.
- `axis`: string (x | y | z); default "y"; Radial: the axis the copies turn around (through center).
- `center`: array of number; Radial: the point the axis passes through; the world origin when omitted.
- `closed`: boolean; default false; Curve: loop the path back to its first point.
- `count`: integer; default 4; range 2 to 200; Copies in all, the original included.
- `id`: integer; range 1 to inf; The object to repeat; the active object when omitted.
- `interp`: string (catmull_rom | bspline | linear); default "catmull_rom"; Curve: how the path runs between points.
- `jitter`: object; Deterministic random variation per copy (the original is untouched); the same seed gives the same result. Not allowed with mirror.; fields position, rotation, scale, seed
- `linked`: boolean; default false; Linked copies (instances) follow the original: sculpt, paint or remesh it and every copy updates after the change. Changing a copy directly unlinks it.
- `merge`: boolean; default false; Join the copies with the original into one object instead of adding separate objects.
- `mode`: string (linear | radial | mirror | curve); default "linear"; linear and radial repeat count copies; mirror adds one copy reflected across the plane square to axis through center; curve places count copies evenly along points.
- `offset`: array of number; Linear: the step from one copy to the next, in world units; one bounds width along x when omitted.
- `points`: array of value; Curve: the path, as [x, y, z] points or objects {p, sharp, round} (corners and fillets as in sample_curve); each copy's bounds centre sits on it.
- `scale_end`: number; default 1; range 0.01 to 100; Uniform scale of the last copy; copies in between ramp linearly. Not allowed with mirror.
- `scale_start`: number; default 1; range 0.01 to 100; Uniform scale of the first copy in the ramp (about the original's bounds centre). Not allowed with mirror.

## insert_mesh

Place a primitive, or a copy of another object, at a point on a surface, standing along the normal there and sunk in by a fraction of its height: eyes, horns, bolts, buttons. Use pick to get the point and normal under a view direction. The new object becomes active.

- `kind`: string (sphere | cube | cylinder | cone | torus); default "sphere"; A primitive to insert, unless source_id is given.
- `normal`: array of number; The surface normal there; the new mesh stands along it. Up when omitted.
- `point`: array of number; required; Where to place it, usually a point on a surface (pick gives one).
- `sink`: number; default 0.25; range -1 to 1; How far it sinks into the surface, as a fraction of its height (0 sits on top, 0.5 half buried).
- `size`: number; default 0.2; range -inf to 10000; The inserted mesh's largest dimension, in world units.
- `source_id`: integer; range 1 to inf; Insert a copy of this object instead (a horn, a bolt, an eye).

## insert_along

String copies of a primitive or another object along a stroke over a surface, as an insert-mesh brush does: each stands on the surface normal, sunk in, evenly spaced and turned to follow the stroke. Rows of scales, studs and rivets, spikes down a spine, stitches, chain links, teeth along a jaw. path is [x, y, z, nx, ny, nz] points (pick gives the point and normal); merge makes one object (the default).

- `align`: boolean; default true; Turn each copy so its x follows the stroke (a chain, a row of scales).
- `kind`: string (sphere | cube | cylinder | cone | torus); default "sphere"; A primitive to insert, unless source_id is given.
- `merge`: boolean; default true; Make all copies one object; false gives one object per copy.
- `path`: array of array; required; The stroke over the surface as [x, y, z, nx, ny, nz] points with their normals (pick gives both), in order.
- `sink`: number; default 0.25; range -1 to 1; How far each copy sinks into the surface, as a fraction of its height.
- `size`: number; default 0.2; range -inf to 10000; Each copy's largest dimension, in world units.
- `source_id`: integer; range 1 to inf; Insert copies of this object instead (scales, studs, spikes, chain links, teeth).
- `spacing`: number; default 1.2; range 0.1 to 20; Distance between copies along the stroke, as a multiple of size.

## boolean

Union, difference (a minus b) or intersection of two objects, giving one watertight object; the inputs are replaced unless keep_inputs. By default the cut is exact: the surfaces are split along their true intersection, so every detail, sharp edge and flat face is kept and paint follows. Both inputs must be closed (mesh_check; repair_mesh if not), or it falls back to the voxel method and says why in fallback. method voxel (and any blend) rebuilds the result through a grid at resolution, which smooths detail finer than a voxel but mends open input. The result reports the method used.

- `a`: integer; required; range 1 to inf; The object to keep or cut from.
- `b`: integer; required; range 1 to inf; The object to add, cut away or intersect with.
- `blend`: number; default 0; range 0 to 100; Fillets the seam over this many world units: a union fuses the two with a soft web where they meet (an arm into a shoulder, a horn into a head), a difference leaves a rounded cut edge. 0 is a sharp boolean.
- `keep_inputs`: boolean; default false; Keep a and b instead of replacing them with the result.
- `method`: string (exact | voxel); default "exact"; exact cuts the two surfaces along their true intersection, keeping every detail, sharp edges and flat faces (both must be closed; repair_mesh first if not); voxel rebuilds the result through a grid, which also mends open or self-crossing input and is what blend uses. An exact boolean on input that is not closed falls back to voxel and says so.
- `op`: string (union | difference | intersection); default "union"
- `resolution`: integer; default 160; range 16 to 320; Voxel method: voxels along the longest side of both objects together; higher keeps sharper detail.

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

Add the meshes in an OBJ, STL, PLY, glTF/GLB, FBX (binary 7.x), USD (text .usda) or 3MF file as new objects (one per OBJ object, glTF node, FBX model, USD Mesh prim or 3MF object), placed by their transforms (USD prim transforms are not applied) with vertex colours as paint. A .gltf's .bin is read from beside it. One undo step.

- `name`: string; Name for the new object; the file name when omitted.
- `path`: string; required; A file path on this computer.

## export_mesh

Write objects to OBJ, STL, PLY, GLB, glTF, FBX, USD (.usda: one Mesh prim per object with displayColor, displayOpacity and st) or 3MF, chosen by the path's extension, overwriting it. GLB, glTF and FBX keep sculpt layers as morph targets (blend shapes in FBX) at their strengths, paint as vertex colours and a material, and baked maps as textures; a .gltf is written with its buffer as a .bin of the same name beside it. STL and PLY join the objects into one mesh.

- `ids`: array of integer; Objects to write; every visible object when omitted. STL and PLY join them into one mesh.
- `path`: string; required; A file path on this computer.

Note: removes or replaces (undoable); in the window: Mod+E.

## bridge_send

Hand an object to another app through a file (OBJ, GLB, PLY, STL or 3MF): it is written to path and remembered, so when the other app (Blender, Maya, a retopology or UV tool) saves over the file, bridge_sync brings the edit back into the same object. A two-way bridge with any app that opens and saves the format.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `path`: string; required; The file to hand over, by extension: .obj, .glb, .fbx, .ply, .stl or .3mf (OBJ is the most widely read; FBX for Maya and 3ds Max).

Note: removes or replaces (undoable).

## bridge_sync

Bring back every bridged file another app has saved since it was sent or last synced, replacing its object's mesh (same object and name, one undo step each). The window syncs on its own every few seconds while bridges are open; agents call this after the other app saves. Lists the open bridges.

- no parameters

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

## selection_set_save

Keep a named snapshot of an object's face groups, mask, vertex paint, sculpt layer strengths and UVs (all it has, or the channels listed) to bring back later with selection_set_restore, as ZBrush's Selection Sets do. Saving under an existing name replaces that set. Up to 64 sets per object; they are saved in projects. Returns the stored channels and the number of values in each.

- `channels`: array of string; What to keep: face groups, the mask, vertex paint (the composited colour, roughness, metalness, opacity and glow), sculpt layer strengths and UVs. All the channels the object has when omitted.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `name`: string; required; The set's name; saving under an existing name replaces that set.

## selection_set_restore

Put a saved selection set back on the object: the channels it holds, or the subset listed. Groups and UVs need the same face count as when saved; mask, paint and sculpt layer strengths need the same vertex count (layers also the same number of layers). If any chosen channel no longer fits, nothing is changed and the error says why. Restoring paint flattens the object's paint layers into the saved colours. One undo step.

- `channels`: array of string; Restore only these channels, which the set must hold. Every channel the set holds when omitted.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `name`: string; required; The set to restore.

Note: removes or replaces (undoable).

## selection_set_delete

Delete a saved selection set from the object. The object itself is untouched, but deleting a set is not an undo step.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.
- `name`: string; required; The set to delete.

Note: removes or replaces (undoable).

## selection_set_list

The object's selection sets: each name, its channels, the number of values in each, the vertex and face counts it was saved with, and whether it can be restored now (restorable) or only some channels (restorable_channels) after the topology changed.

- `id`: integer; range 1 to inf; Object id; the active object when omitted.

Note: read-only.

## channel_copy

Copy an object's face groups, mask or vertex paint into the engine's clipboard (one channel at a time, replacing what was there) to paste onto another object with channel_paste. Changes no object.

- `channel`: string (groups | mask | paint); required; What to copy: the face groups (per face), the mask or the vertex paint (per vertex, as composited).
- `id`: integer; range 1 to inf; Object id; the active object when omitted.

Note: safe to repeat.

## channel_paste

Paste the copied channel onto an object with the same count: groups need the same face count, mask and paint the same vertex count, otherwise nothing is changed and the error gives both counts. Pasting paint replaces the object's paint and flattens its paint layers. One undo step.

- `channel`: string (groups | mask | paint); The channel the clipboard must hold; whatever was copied when omitted.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.

Note: removes or replaces (undoable).

## channel_clear

Clear an object's face groups (every face to group 0), mask (every vertex to 0) or vertex paint (back to unpainted, dropping the paint layers). Does nothing, with changed false, when the channel is already clear. One undo step.

- `channel`: string (groups | mask | paint); required; groups puts every face in group 0; mask sets every vertex to 0; paint removes all vertex paint and the paint layer stack, back to unpainted.
- `id`: integer; range 1 to inf; Object id; the active object when omitted.

Note: removes or replaces (undoable).
