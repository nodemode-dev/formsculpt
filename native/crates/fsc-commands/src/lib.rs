mod file_cmds;
mod history;
mod layer_cmds;
mod multires_cmds;
mod object_cmds;
mod scene_cmds;
mod sculpt_cmds;
mod topo_cmds;

pub use history::{Edit, History};
pub use file_cmds::{export_data, import_data, open_data, save_data};
pub use sculpt_cmds::{parse_brush, push_stroke};

use fsc_core::Scene;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    System,
    Scene,
    Object,
    Sculpt,
    Mask,
    Group,
    Topo,
    Layer,
    Paint,
    View,
    File,
    History,
}

#[derive(Debug, Clone, Copy, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Hints {
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub idempotent_hint: bool,
    pub open_world_hint: bool,
}

pub struct Command {
    pub id: &'static str,
    pub mcp: &'static str,
    pub group: Group,
    pub label: &'static str,
    pub description: &'static str,
    pub keys: &'static [&'static str],
    pub hints: Hints,
    pub schema: fn() -> Value,
    pub run: fn(&mut Session, &Value) -> Result<Value, CommandError>,
}

#[derive(Debug, Default)]
pub struct Session {
    pub scene: Scene,
    pub history: History,
    pub stroke: Option<(fsc_core::ObjectId, fsc_sculpt::Stroke)>,
    pub loads: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
}

impl CommandError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

pub(crate) fn no_args() -> Value {
    json!({"type": "object", "properties": {}, "required": [], "additionalProperties": false})
}

fn engine_info(_: &mut Session, _: &Value) -> Result<Value, CommandError> {
    Ok(json!({"engine": "formsculptor", "version": fsc_core::engine_version()}))
}

pub(crate) const READ: Hints = Hints { read_only_hint: true, destructive_hint: false, idempotent_hint: true, open_world_hint: false };
pub(crate) const WRITE: Hints = Hints { read_only_hint: false, destructive_hint: false, idempotent_hint: false, open_world_hint: false };
pub(crate) const DESTROY: Hints = Hints { read_only_hint: false, destructive_hint: true, idempotent_hint: false, open_world_hint: false };

pub static COMMANDS: &[Command] = &[
    Command {
        id: "system.info",
        mcp: "get_engine_info",
        group: Group::System,
        label: "Engine Info",
        description: "The engine's name and version. Use it to confirm the engine is reachable before other calls.",
        keys: &[],
        hints: READ,
        schema: no_args,
        run: engine_info,
    },
    Command {
        id: "scene.get",
        mcp: "get_scene",
        group: Group::Scene,
        label: "Scene Summary",
        description: "Every object in the scene with its id, name, visibility, vertex and triangle counts and bounds, and which object is active. Call it first to learn object ids.",
        keys: &[],
        hints: READ,
        schema: no_args,
        run: scene_cmds::get_scene,
    },
    Command {
        id: "scene.new",
        mcp: "new_scene",
        group: Group::Scene,
        label: "New Scene",
        description: "Remove every object and start an empty scene.",
        keys: &["Mod+Alt+N"],
        hints: DESTROY,
        schema: no_args,
        run: scene_cmds::new_scene,
    },
    Command {
        id: "object.add_primitive",
        mcp: "add_primitive",
        group: Group::Object,
        label: "Add Primitive",
        description: "Add a primitive mesh centred at the origin and make it active. Kinds: sphere (an icosphere; subdivisions 0 to 9, each level four times the triangles; 6 gives 81,920), uv_sphere, cube (divisions per edge), cylinder, cone, torus and plane. Returns the new object's summary.",
        keys: &[],
        hints: WRITE,
        schema: scene_cmds::add_primitive_schema,
        run: scene_cmds::add_primitive,
    },
    Command {
        id: "object.select",
        mcp: "select_object",
        group: Group::Object,
        label: "Select Object",
        description: "Make an object the active one by id.",
        keys: &[],
        hints: Hints { read_only_hint: false, destructive_hint: false, idempotent_hint: true, open_world_hint: false },
        schema: scene_cmds::id_schema,
        run: scene_cmds::select_object,
    },
    Command {
        id: "object.delete",
        mcp: "delete_object",
        group: Group::Object,
        label: "Delete Object",
        description: "Remove an object from the scene by id.",
        keys: &["Delete"],
        hints: DESTROY,
        schema: scene_cmds::id_schema,
        run: scene_cmds::delete_object,
    },
    Command {
        id: "sculpt.stroke",
        mcp: "sculpt_stroke",
        group: Group::Sculpt,
        label: "Sculpt Stroke",
        description: "Run one brush stroke along a path of points on the surface of an object, as one undo step. Brushes: draw (build along the area normal), clay (build up to a plane), inflate (along each vertex normal), pinch, crease, flatten, smooth, grab (move the vertices around the first point by the offset to the last) and mask (paint the mask; masked vertices resist every other brush). invert carves, deflates or erases. Points closer than radius times spacing are merged into dabs; space points about radius * 0.15 apart for an even stroke. Symmetry mirrors across the object's x, y or z planes through the origin and repeats radially around y.",
        keys: &[],
        hints: WRITE,
        schema: sculpt_cmds::stroke_schema,
        run: sculpt_cmds::sculpt_stroke,
    },
    Command {
        id: "mask.invert",
        mcp: "mask_invert",
        group: Group::Mask,
        label: "Invert Mask",
        description: "Invert the mask of an object (the active one when id is omitted).",
        keys: &["Mod+I"],
        hints: WRITE,
        schema: sculpt_cmds::id_opt_schema,
        run: sculpt_cmds::mask_invert,
    },
    Command {
        id: "mask.clear",
        mcp: "mask_clear",
        group: Group::Mask,
        label: "Clear Mask",
        description: "Remove the mask from an object so every vertex can be sculpted.",
        keys: &["Mod+Shift+A"],
        hints: WRITE,
        schema: sculpt_cmds::id_opt_schema,
        run: sculpt_cmds::mask_clear,
    },
    Command {
        id: "mask.blur",
        mcp: "mask_blur",
        group: Group::Mask,
        label: "Blur Mask",
        description: "Soften the mask's edge by averaging each vertex with its neighbours once.",
        keys: &[],
        hints: WRITE,
        schema: sculpt_cmds::id_opt_schema,
        run: sculpt_cmds::mask_blur,
    },
    Command {
        id: "mask.sharpen",
        mcp: "mask_sharpen",
        group: Group::Mask,
        label: "Sharpen Mask",
        description: "Harden the mask's edge by pushing values away from one half.",
        keys: &[],
        hints: WRITE,
        schema: sculpt_cmds::id_opt_schema,
        run: sculpt_cmds::mask_sharpen,
    },
    Command {
        id: "topo.subdivide",
        mcp: "subdivide",
        group: Group::Topo,
        label: "Subdivide",
        description: "Subdivide an object's mesh, each level multiplying the triangles by four (Loop subdivision, which rounds the shape; smooth false splits flat). The mask carries over. This replaces the mesh in one step; multires_divide keeps the lower levels to step back to.",
        keys: &[],
        hints: WRITE,
        schema: topo_cmds::subdivide_schema,
        run: topo_cmds::subdivide,
    },
    Command {
        id: "topo.remesh_voxel",
        mcp: "remesh_voxel",
        group: Group::Topo,
        label: "Voxel Remesh",
        description: "Rebuild an object as an even, closed mesh by sampling its surface on a voxel grid (like DynaMesh or Nomad's voxel remesher). Fixes stretched triangles after heavy grab or inflate strokes and merges intersecting parts. resolution is voxels along the longest side. The mask is cleared.",
        keys: &["Mod+R"],
        hints: WRITE,
        schema: topo_cmds::remesh_schema,
        run: topo_cmds::remesh_voxel,
    },
    Command {
        id: "topo.decimate",
        mcp: "decimate",
        group: Group::Topo,
        label: "Decimate",
        description: "Reduce an object's triangle count while keeping its shape (quadric error edge collapse), by ratio or to a target count; borders are kept by default. The mask is cleared.",
        keys: &[],
        hints: WRITE,
        schema: topo_cmds::decimate_schema,
        run: topo_cmds::decimate,
    },
    Command {
        id: "topo.fill_holes",
        mcp: "fill_holes",
        group: Group::Topo,
        label: "Fill Holes",
        description: "Close every open boundary loop of an object with a fan of triangles. Reports how many holes were filled.",
        keys: &[],
        hints: WRITE,
        schema: sculpt_cmds::id_opt_schema,
        run: topo_cmds::fill_holes,
    },
    Command {
        id: "topo.check",
        mcp: "mesh_check",
        group: Group::Topo,
        label: "Check Mesh",
        description: "Report an object's vertex, triangle and edge counts, open boundary edges, non-manifold edges and whether it is watertight (needed for printing and booleans).",
        keys: &[],
        hints: READ,
        schema: sculpt_cmds::id_opt_schema,
        run: topo_cmds::mesh_check,
    },
    Command {
        id: "multires.divide",
        mcp: "multires_divide",
        group: Group::Topo,
        label: "Divide",
        description: "Add subdivision levels on top of an object (Loop subdivision, each level four times the triangles) and keep the lower levels, so you can block out on a low level and detail on a high one. The object must be at its highest level. Objects with levels refuse dyntopo, remesh, decimate, subdivide and fill holes until the levels are deleted.",
        keys: &["Mod+D"],
        hints: WRITE,
        schema: multires_cmds::divide_schema,
        run: multires_cmds::divide,
    },
    Command {
        id: "multires.set_level",
        mcp: "multires_set_level",
        group: Group::Topo,
        label: "Set Level",
        description: "Move an object to another subdivision level by number or one step up or down. Changes made on a lower level carry up to the higher levels with their detail kept; the mask carries both ways.",
        keys: &["D", "Shift+D"],
        hints: WRITE,
        schema: multires_cmds::level_schema,
        run: multires_cmds::set_level,
    },
    Command {
        id: "multires.delete_higher",
        mcp: "multires_delete_higher",
        group: Group::Topo,
        label: "Delete Higher",
        description: "Delete the subdivision levels above the current one. Undoable.",
        keys: &[],
        hints: DESTROY,
        schema: sculpt_cmds::id_opt_schema,
        run: multires_cmds::delete_higher,
    },
    Command {
        id: "multires.delete_lower",
        mcp: "multires_delete_lower",
        group: Group::Topo,
        label: "Delete Lower",
        description: "Delete the subdivision levels below the current one, making it the base. Deleting both lower and higher levels leaves a plain mesh that dyntopo and remeshing accept. Undoable.",
        keys: &[],
        hints: DESTROY,
        schema: sculpt_cmds::id_opt_schema,
        run: multires_cmds::delete_lower,
    },
    Command {
        id: "multires.reconstruct",
        mcp: "multires_reconstruct",
        group: Group::Topo,
        label: "Reconstruct Lower",
        description: "Rebuild one subdivision level below the lowest one, for a mesh with the regular one-to-four structure of a subdivided mesh (one made by subdivide, or left after multires_delete_lower). Repeat to rebuild more. Refuses with not_subdivided for other meshes (remeshed, decimated, sculpted with dyntopo or imported irregular meshes). Undoable.",
        keys: &[],
        hints: WRITE,
        schema: sculpt_cmds::id_opt_schema,
        run: multires_cmds::reconstruct,
    },
    Command {
        id: "object.duplicate",
        mcp: "duplicate_object",
        group: Group::Object,
        label: "Duplicate",
        description: "Copy an object's current mesh and mask into a new object, optionally moved by an offset. Subdivision levels and layers are not copied.",
        keys: &[],
        hints: WRITE,
        schema: object_cmds::duplicate_schema,
        run: object_cmds::duplicate,
    },
    Command {
        id: "object.rename",
        mcp: "rename_object",
        group: Group::Object,
        label: "Rename",
        description: "Give an object a new name.",
        keys: &[],
        hints: WRITE,
        schema: object_cmds::rename_schema,
        run: object_cmds::rename,
    },
    Command {
        id: "object.transform",
        mcp: "transform_object",
        group: Group::Object,
        label: "Transform",
        description: "Move, rotate (degrees about x, y, z) and scale an object's vertices around its bounds centre. Masked vertices stay put in proportion to the mask, so a mask works as topology masking. One undo step.",
        keys: &[],
        hints: WRITE,
        schema: object_cmds::transform_schema,
        run: object_cmds::transform,
    },
    Command {
        id: "object.deform",
        mcp: "deform_object",
        group: Group::Object,
        label: "Deform",
        description: "Bend, twist or taper an object along an axis of its bounds. Masked vertices are held in proportion to the mask. One undo step.",
        keys: &[],
        hints: WRITE,
        schema: object_cmds::deform_schema,
        run: object_cmds::deform,
    },
    Command {
        id: "object.merge",
        mcp: "merge_objects",
        group: Group::Object,
        label: "Merge",
        description: "Join several objects into the first one's mesh (the parts stay separate shells; use boolean union to fuse them).",
        keys: &[],
        hints: WRITE,
        schema: object_cmds::merge_schema,
        run: object_cmds::merge,
    },
    Command {
        id: "object.split",
        mcp: "split_parts",
        group: Group::Object,
        label: "Split Parts",
        description: "Split an object into one object per connected part of its mesh.",
        keys: &[],
        hints: WRITE,
        schema: sculpt_cmds::id_opt_schema,
        run: object_cmds::split,
    },
    Command {
        id: "object.boolean",
        mcp: "boolean",
        group: Group::Object,
        label: "Boolean",
        description: "Union, difference (a minus b) or intersection of two objects through a voxel grid, giving one watertight object; the inputs are replaced unless keep_inputs. Detail finer than a voxel is smoothed, so raise resolution for sharp results. Both inputs should be closed meshes (mesh_check).",
        keys: &[],
        hints: WRITE,
        schema: object_cmds::boolean_schema,
        run: object_cmds::boolean,
    },
    Command {
        id: "layer.add",
        mcp: "layer_add",
        group: Group::Layer,
        label: "Add Layer",
        description: "Add a sculpt layer to an object and make it the one strokes record into. A layer stores the strokes made while it is active as offsets, so its strength can later fade, exaggerate or invert them.",
        keys: &[],
        hints: WRITE,
        schema: layer_cmds::add_schema,
        run: layer_cmds::add,
    },
    Command {
        id: "layer.select",
        mcp: "layer_select",
        group: Group::Layer,
        label: "Select Layer",
        description: "Choose which layer strokes record into, or null for the base mesh.",
        keys: &[],
        hints: WRITE,
        schema: layer_cmds::select_schema,
        run: layer_cmds::select,
    },
    Command {
        id: "layer.strength",
        mcp: "layer_set_strength",
        group: Group::Layer,
        label: "Layer Strength",
        description: "Set a layer's strength (0 hides it, 1 is as sculpted, up to 2, negative inverts).",
        keys: &[],
        hints: WRITE,
        schema: layer_cmds::strength_schema,
        run: layer_cmds::strength,
    },
    Command {
        id: "layer.delete",
        mcp: "layer_delete",
        group: Group::Layer,
        label: "Delete Layer",
        description: "Remove a layer and the shape it adds.",
        keys: &[],
        hints: DESTROY,
        schema: layer_cmds::index_only_schema,
        run: layer_cmds::delete,
    },
    Command {
        id: "layer.apply",
        mcp: "layer_apply",
        group: Group::Layer,
        label: "Apply Layer",
        description: "Bake a layer into the mesh at its current strength and remove it. While an object has layers, topology changes, dyntopo and level changes refuse with has_layers.",
        keys: &[],
        hints: WRITE,
        schema: layer_cmds::index_only_schema,
        run: layer_cmds::apply,
    },
    Command {
        id: "file.save",
        mcp: "save_project",
        group: Group::File,
        label: "Save Project",
        description: "Save the whole scene to a .fsc project file (objects, masks, subdivision levels and sculpt layers), overwriting the path. Opening it again restores the scene exactly.",
        keys: &["Mod+S"],
        hints: DESTROY,
        schema: file_cmds::save_schema,
        run: file_cmds::save,
    },
    Command {
        id: "file.open",
        mcp: "open_project",
        group: Group::File,
        label: "Open Project",
        description: "Replace the scene with a .fsc project. Clears the undo history; save first if the current scene matters.",
        keys: &["Mod+O"],
        hints: DESTROY,
        schema: file_cmds::save_schema,
        run: file_cmds::open,
    },
    Command {
        id: "file.import",
        mcp: "import_mesh",
        group: Group::File,
        label: "Import Mesh",
        description: "Add the meshes in an OBJ, STL, PLY, glTF/GLB or 3MF file as new objects (one per OBJ object, glTF node or 3MF object). glTF must be a GLB or carry its buffers inside. One undo step.",
        keys: &[],
        hints: WRITE,
        schema: file_cmds::import_schema,
        run: file_cmds::import,
    },
    Command {
        id: "file.export",
        mcp: "export_mesh",
        group: Group::File,
        label: "Export Mesh",
        description: "Write objects to OBJ, STL, PLY, GLB or 3MF, chosen by the path's extension, overwriting it. GLB keeps sculpt layers as morph targets at their strengths; STL and PLY join the objects into one mesh.",
        keys: &["Mod+E"],
        hints: DESTROY,
        schema: file_cmds::export_schema,
        run: file_cmds::export,
    },
    Command {
        id: "file.print_check",
        mcp: "print_check",
        group: Group::File,
        label: "Print Check",
        description: "Check an object for 3D printing: watertight, non-manifold edges, inside-out, size, volume and area in millimetres, and wall thickness sampled by casting inward from up to 4000 vertices. Lists the problems and whether it is printable.",
        keys: &[],
        hints: READ,
        schema: file_cmds::print_schema,
        run: file_cmds::print_check,
    },
    Command {
        id: "file.repair",
        mcp: "repair_mesh",
        group: Group::Topo,
        label: "Repair Mesh",
        description: "Make an object closed and manifold: split vertices where surfaces only touch, fill open holes, drop degenerate triangles and unused vertices. One undo step; the mask is cleared.",
        keys: &[],
        hints: WRITE,
        schema: sculpt_cmds::id_opt_schema,
        run: file_cmds::repair,
    },
    Command {
        id: "history.undo",
        mcp: "undo",
        group: Group::History,
        label: "Undo",
        description: "Undo the last step (a stroke, a mask change, adding or deleting an object, a new scene, a topology change, a level change).",
        keys: &["Mod+Z"],
        hints: WRITE,
        schema: no_args,
        run: sculpt_cmds::undo,
    },
    Command {
        id: "history.redo",
        mcp: "redo",
        group: Group::History,
        label: "Redo",
        description: "Redo the last undone step.",
        keys: &["Mod+Shift+Z"],
        hints: WRITE,
        schema: no_args,
        run: sculpt_cmds::redo,
    },
];

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.id == name || c.mcp == name)
}

pub fn dispatch(session: &mut Session, name: &str, args: &Value) -> Result<Value, CommandError> {
    let command = find(name).ok_or_else(|| CommandError::new("unknown_command", format!("no command named {name}")))?;
    let args = if args.is_null() { json!({}) } else { args.clone() };
    if !args.is_object() {
        return Err(CommandError::new("bad_arguments", "arguments must be a JSON object"));
    }
    (command.run)(session, &args)
}

pub fn catalogue() -> Value {
    let commands: Vec<Value> = COMMANDS
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.mcp,
                "group": c.group,
                "label": c.label,
                "description": c.description,
                "keys": c.keys,
                "inputSchema": (c.schema)(),
                "annotations": c.hints,
            })
        })
        .collect();
    json!({"engine": "formsculptor", "version": fsc_core::engine_version(), "commands": commands})
}
