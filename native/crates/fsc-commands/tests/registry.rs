//! The registry is the one place a command is declared; the MCP server, the window and the agent skills all read it.

use std::collections::HashSet;

use fsc_commands::{catalogue, dispatch, Session, COMMANDS};
use serde_json::{json, Value};

#[test]
fn ids_and_names_are_unique_and_well_formed() {
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for c in COMMANDS {
        assert!(ids.insert(c.id), "duplicate id {}", c.id);
        assert!(names.insert(c.mcp), "duplicate MCP name {}", c.mcp);
        assert!(c.id.contains('.'), "{} is not a dotted id", c.id);
        assert!(c.mcp.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'), "{} is not snake_case", c.mcp);
        assert!(c.mcp.len() <= 64, "{} is too long for an MCP tool name", c.mcp);
        assert!(!c.label.is_empty() && !c.description.is_empty(), "{} lacks a label or description", c.id);
    }
}

#[test]
fn every_schema_is_a_closed_object() {
    for c in COMMANDS {
        let s: Value = (c.schema)();
        assert_eq!(s["type"], "object", "{} schema is not an object", c.mcp);
        assert!(s["properties"].is_object(), "{} schema has no properties", c.mcp);
        if let Some(required) = s["required"].as_array() {
            for r in required {
                let key = r.as_str().expect("required names are strings");
                assert!(s["properties"].get(key).is_some(), "{} requires {key}, which it does not declare", c.mcp);
            }
        }
        assert_eq!(s["additionalProperties"], false, "{} schema accepts unknown arguments", c.mcp);
    }
}

#[test]
fn no_em_dashes_in_what_agents_read() {
    let text = catalogue().to_string();
    assert!(!text.contains('\u{2014}'), "the catalogue contains an em dash");
}

#[test]
fn catalogue_lists_every_command() {
    let cat = catalogue();
    let listed = cat["commands"].as_array().expect("commands").len();
    assert_eq!(listed, COMMANDS.len());
    assert_eq!(cat["engine"], "formsculptor");
}

#[test]
fn unknown_commands_and_bad_arguments_are_refused() {
    let mut s = Session::default();
    assert_eq!(dispatch(&mut s, "no_such_command", &json!({})).unwrap_err().code, "unknown_command");
    assert_eq!(dispatch(&mut s, "get_scene", &json!([1, 2])).unwrap_err().code, "bad_arguments");
}

#[test]
fn read_only_commands_leave_the_history_alone() {
    let mut s = Session::default();
    dispatch(&mut s, "add_primitive", &json!({"kind": "sphere", "detail": 3})).unwrap();
    let before = dispatch(&mut s, "get_history", &json!({})).unwrap();
    for c in COMMANDS.iter().filter(|c| c.hints.read_only_hint) {
        // Read-only commands that need arguments may refuse; none may add a step.
        let _ = dispatch(&mut s, c.mcp, &json!({}));
    }
    assert_eq!(dispatch(&mut s, "get_history", &json!({})).unwrap(), before);
}
