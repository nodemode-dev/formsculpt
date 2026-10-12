//! Projects, file formats and the headless renders.

mod common;

use common::*;
use fsc_commands::Session;
use serde_json::json;

#[test]
fn a_project_reopens_exactly() {
    let mut s = Session::default();
    let id = add(&mut s, "torus", json!({}));
    call(&mut s, "paint_fill", json!({"id": id, "color": "#2060ff"}));
    call(&mut s, "layer_add", json!({"id": id, "name": "Detail"}));
    call(&mut s, "sculpt_stroke", json!({"id": id, "brush": "draw", "points": [[1, 0, 0.3]], "radius": 0.3, "strength": 1}));
    let path = temp("roundtrip.fsc");
    call(&mut s, "save_project", json!({"path": path}));
    let before = call(&mut s, "get_scene", json!({}));
    let mut t = Session::default();
    call(&mut t, "open_project", json!({"path": path}));
    let after = call(&mut t, "get_scene", json!({}));
    let (a, b) = (&before["objects"][0], &after["objects"][0]);
    for key in ["name", "vertices", "triangles", "bounds", "layers"] {
        assert_eq!(a[key], b[key], "{key} changed across save and open");
    }
}

#[test]
fn every_format_round_trips_the_triangles() {
    let mut s = Session::default();
    let id = add(&mut s, "cube", json!({"detail": 4}));
    let tris = object(&mut s, id)["triangles"].clone();
    for ext in ["obj", "stl", "ply", "glb", "gltf", "3mf"] {
        let path = temp(&format!("cube.{ext}"));
        call(&mut s, "export_mesh", json!({"path": path, "ids": [id]}));
        let r = call(&mut s, "import_mesh", json!({"path": path}));
        assert_eq!(r["objects"][0]["triangles"], tris, "{ext} changed the triangle count");
    }
}

#[test]
fn gltf_is_written_with_its_bin_and_needs_it_to_read() {
    let mut s = Session::default();
    add(&mut s, "sphere", json!({"detail": 3}));
    call(&mut s, "paint_fill", json!({"color": "#ff8800"}));
    let path = temp("split.gltf");
    let r = call(&mut s, "export_mesh", json!({"path": path}));
    let bin = r["bin"].as_str().unwrap().to_string();
    let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(doc["buffers"][0]["uri"], "split.bin");
    assert!(doc["meshes"][0]["primitives"][0]["attributes"]["COLOR_0"].is_u64(), "paint was not written");
    std::fs::rename(&bin, format!("{bin}.away")).unwrap();
    assert_eq!(fails(&mut s, "import_mesh", json!({"path": path})), "bad_file");
    std::fs::rename(format!("{bin}.away"), &bin).unwrap();
    call(&mut s, "import_mesh", json!({"path": path}));
}

#[test]
fn get_view_renders_the_scene() {
    let mut s = Session::default();
    add(&mut s, "sphere", json!({"detail": 4}));
    let r = call(&mut s, "get_view", json!({"view": "front", "width": 128, "height": 128}));
    assert_eq!(r["mime"], "image/png");
    assert!(r["coverage"].as_f64().unwrap() > 0.1);
    assert!(r["png_base64"].as_str().unwrap().len() > 1000);
}

#[test]
fn animations_leave_the_scene_and_history_as_they_were() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 4}));
    call(&mut s, "layer_add", json!({"id": id, "name": "Swell"}));
    call(&mut s, "sculpt_stroke", json!({"id": id, "brush": "inflate", "points": [[0, 1, 0]], "radius": 0.6, "strength": 1}));
    let scene = call(&mut s, "get_scene", json!({}));
    let history = call(&mut s, "get_history", json!({}));
    let keys = json!([{"time": 0, "yaw": 0, "layers": [{"id": id, "layer": 0, "strength": 0}]}, {"time": 1, "yaw": 180, "zoom": 1.5, "layers": [{"id": id, "layer": 0, "strength": 1.5}]}]);
    let r = call(&mut s, "record_animation", json!({"path": temp("anim.png"), "keys": keys, "fps": 8, "size": 64}));
    assert_eq!(r["frames"], 8);
    let r = call(&mut s, "record_history", json!({"path": temp("history.png"), "size": 64}));
    assert!(r["frames"].as_u64().unwrap() >= 3);
    call(&mut s, "record_turntable", json!({"path": temp("turn.png"), "frames": 8, "size": 64}));
    let after = call(&mut s, "get_scene", json!({}));
    assert_eq!(scene["objects"][0]["bounds"], after["objects"][0]["bounds"]);
    assert_eq!(scene["objects"][0]["layers"], after["objects"][0]["layers"]);
    assert_eq!(call(&mut s, "get_history", json!({})), history);
    assert_eq!(fails(&mut s, "record_animation", json!({"path": temp("x.png"), "keys": [{"time": 0}]})), "bad_arguments");
}
