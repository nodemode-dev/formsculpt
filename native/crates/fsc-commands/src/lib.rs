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
    pub run: fn(&Value) -> Result<Value, CommandError>,
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

fn no_args() -> Value {
    json!({"type": "object", "properties": {}, "required": [], "additionalProperties": false})
}

fn engine_info(_: &Value) -> Result<Value, CommandError> {
    Ok(json!({"engine": "formsculptor", "version": fsc_core::engine_version()}))
}

pub static COMMANDS: &[Command] = &[Command {
    id: "system.info",
    mcp: "get_engine_info",
    group: Group::System,
    label: "Engine Info",
    description: "The engine's name and version. Use it to confirm the engine is reachable before other calls.",
    keys: &[],
    hints: Hints { read_only_hint: true, destructive_hint: false, idempotent_hint: true, open_world_hint: false },
    schema: no_args,
    run: engine_info,
}];

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.id == name || c.mcp == name)
}

pub fn dispatch(name: &str, args: &Value) -> Result<Value, CommandError> {
    let command = find(name).ok_or_else(|| CommandError::new("unknown_command", format!("no command named {name}")))?;
    (command.run)(args)
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
