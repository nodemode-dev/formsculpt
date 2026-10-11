mod history;
mod multires_cmds;
mod scene_cmds;
mod sculpt_cmds;
mod topo_cmds;

pub use history::{Edit, History};
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
