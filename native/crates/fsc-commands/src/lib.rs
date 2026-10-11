mod scene_cmds;

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
