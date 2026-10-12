#![allow(dead_code)]

use fsc_commands::{dispatch, Session};
use serde_json::{json, Value};

/// Runs a command that must succeed and returns its result.
pub fn call(s: &mut Session, name: &str, args: Value) -> Value {
    dispatch(s, name, &args).unwrap_or_else(|e| panic!("{name} failed: {} {}", e.code, e.message))
}

/// Runs a command that must fail and returns its error code.
pub fn fails(s: &mut Session, name: &str, args: Value) -> &'static str {
    match dispatch(s, name, &args) {
        Ok(v) => panic!("{name} should have failed but returned {v}"),
        Err(e) => e.code,
    }
}

/// Adds a primitive and returns its id.
pub fn add(s: &mut Session, kind: &str, extra: Value) -> u64 {
    let mut args = json!({"kind": kind});
    if let (Some(a), Some(e)) = (args.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            a.insert(k.clone(), v.clone());
        }
    }
    call(s, "add_primitive", args)["object"]["id"].as_u64().expect("id")
}

/// The object summary for an id.
pub fn object(s: &mut Session, id: u64) -> Value {
    call(s, "get_scene", json!({}))["objects"].as_array().expect("objects").iter().find(|o| o["id"] == id).cloned().unwrap_or_else(|| panic!("no object {id}"))
}

/// print_check's verdict for an object: (printable, problems).
pub fn printable(s: &mut Session, id: u64) -> (bool, Vec<String>) {
    let r = call(s, "print_check", json!({"id": id}));
    let problems = r["problems"].as_array().map(|a| a.iter().filter_map(|p| p.as_str().map(str::to_string)).collect()).unwrap_or_default();
    (r["printable"].as_bool().unwrap_or(false), problems)
}

/// Watertight per print_check, whatever the wall thickness.
pub fn watertight(s: &mut Session, id: u64) -> bool {
    let r = call(s, "mesh_check", json!({"id": id}));
    r["watertight"].as_bool().unwrap_or(false)
}

pub fn size(o: &Value, axis: usize) -> f64 {
    o["bounds"]["max"][axis].as_f64().unwrap() - o["bounds"]["min"][axis].as_f64().unwrap()
}

/// A fresh file path in the system's temporary folder.
pub fn temp(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("fsc-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.join(name).to_string_lossy().into_owned()
}
