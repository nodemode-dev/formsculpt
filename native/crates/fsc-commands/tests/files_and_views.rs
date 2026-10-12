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
    for ext in ["obj", "stl", "ply", "glb", "gltf", "3mf", "fbx"] {
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
fn a_bridged_file_saved_elsewhere_comes_back_into_its_object() {
    let mut s = Session::default();
    let cube = add(&mut s, "cube", json!({"detail": 2}));
    let path = temp("bridge.obj");
    call(&mut s, "bridge_send", json!({"id": cube, "path": path}));
    assert!(call(&mut s, "bridge_sync", json!({}))["updated"].as_array().unwrap().is_empty(), "an unchanged file synced");
    // Another app saves a torus over the file.
    let mut other = Session::default();
    add(&mut other, "torus", json!({}));
    let theirs = temp("theirs.obj");
    call(&mut other, "export_mesh", json!({"path": theirs}));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::copy(&theirs, &path).unwrap();
    let r = call(&mut s, "bridge_sync", json!({}));
    assert_eq!(r["updated"][0]["id"], cube);
    let torus_vertices = object(&mut other, 1)["vertices"].clone();
    assert_eq!(object(&mut s, cube)["vertices"], torus_vertices);
    assert_eq!(call(&mut s, "undo", json!({}))["undone"], "Bridge Update");
    // Deleting the object ends the bridge.
    call(&mut s, "delete_object", json!({"id": cube}));
    assert!(call(&mut s, "bridge_sync", json!({}))["bridges"].as_array().unwrap().is_empty());
}

#[test]
fn fbx_keeps_objects_placement_and_paint() {
    let mut s = Session::default();
    let t = add(&mut s, "torus", json!({}));
    call(&mut s, "paint_fill", json!({"id": t, "color": "#2060ff"}));
    let sph = add(&mut s, "sphere", json!({"detail": 3}));
    call(&mut s, "transform_object", json!({"id": sph, "translate": [3, 0, 0]}));
    let path = temp("scene.fbx");
    call(&mut s, "export_mesh", json!({"path": path}));
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"Kaydara FBX Binary  \x00"));
    let mut t2 = Session::default();
    let r = call(&mut t2, "import_mesh", json!({"path": path}));
    let objs = r["objects"].as_array().unwrap();
    assert_eq!(objs.len(), 2);
    assert_eq!(objs[0]["name"], "Torus");
    assert_eq!(objs[0]["vertices"], object(&mut s, t)["vertices"]);
    assert!((objs[1]["bounds"]["min"][0].as_f64().unwrap() - 2.0).abs() < 1e-3, "the sphere lost its placement");
    let ascii = temp("ascii.fbx");
    std::fs::write(&ascii, b"; FBX 7.4.0 project file\n").unwrap();
    assert_eq!(fails(&mut t2, "import_mesh", json!({"path": ascii})), "bad_file");
}

#[test]
fn materials_are_one_undo_step_and_saved() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    assert_eq!(object(&mut s, id)["material"]["subsurface"], 0.0);
    call(&mut s, "set_material", json!({"id": id, "subsurface": 0.5, "subsurface_color": [1, 0.2, 0.1], "transmission": 0.25, "ior": 1.33}));
    let m = object(&mut s, id)["material"].clone();
    assert_eq!(m["subsurface"], 0.5);
    assert!((m["ior"].as_f64().unwrap() - 1.33).abs() < 1e-6);
    assert_eq!(fails(&mut s, "set_material", json!({"id": id, "ior": 3})), "bad_arguments");
    let path = temp("material.fsc");
    call(&mut s, "save_project", json!({"path": path}));
    let mut t = Session::default();
    call(&mut t, "open_project", json!({"path": path}));
    assert_eq!(call(&mut t, "get_scene", json!({}))["objects"][0]["material"], m);
    assert_eq!(call(&mut s, "undo", json!({}))["undone"], "Material");
    assert_eq!(object(&mut s, id)["material"]["subsurface"], 0.0);
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
