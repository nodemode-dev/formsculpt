//! Cloth, posing, fibers, the path-traced render, manual retopology, edge loops, bridges and the Substance handoff.

mod common;

use common::*;
use fsc_commands::Session;
use serde_json::json;

#[test]
fn cloth_drapes_over_a_sphere_and_rests_on_the_floor() {
    let mut s = Session::default();
    add(&mut s, "sphere", json!({"detail": 4}));
    let cloth = add(&mut s, "plane", json!({"detail": 48, "size": 3}));
    call(&mut s, "transform_object", json!({"id": cloth, "translate": [0, 1.5, 0]}));
    let r = call(&mut s, "cloth_drape", json!({"id": cloth, "steps": 240, "floor": -1}));
    assert!(r["contacts"].as_u64().unwrap() > 100);
    let b = &r["object"]["bounds"];
    // Draped: the top rests on the sphere (radius 1) and the hem falls to the floor.
    assert!((b["max"][1].as_f64().unwrap() - 1.0).abs() < 0.1, "{b}");
    assert!(b["min"][1].as_f64().unwrap() < -0.85, "{b}");
    // Nothing passed into the sphere.
    let mesh = &s.scene.get(cloth as u32).unwrap().mesh;
    assert!(mesh.positions.iter().all(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() > 0.98));
    let steps = call(&mut s, "get_history", json!({}))["steps"].as_array().unwrap().len();
    call(&mut s, "undo", json!({}));
    assert_eq!(call(&mut s, "get_history", json!({}))["done"].as_u64().unwrap() as usize, steps - 1);
}

#[test]
fn pose_turns_the_unmasked_part_about_the_joint() {
    let mut s = Session::default();
    add(&mut s, "cylinder", json!({"size": 0.2, "height": 2, "detail": 64}));
    assert_eq!(fails(&mut s, "pose", json!({"pivot": [0, 0, 0], "angle": 90})), "no_mask");
    call(&mut s, "mask_shape", json!({"view": "front", "shape": "box", "points": [[0, 0.5], [1, 1]], "width": 640, "height": 640}));
    let r = call(&mut s, "pose", json!({"pivot": [0, 0, 0], "axis": [0, 0, 1], "angle": 90}));
    let b = &r["object"]["bounds"];
    // The upper half now points along -x; the masked lower half stayed.
    assert!(b["min"][0].as_f64().unwrap() < -0.95 && b["max"][1].as_f64().unwrap() < 0.25 && b["min"][1].as_f64().unwrap() < -0.99, "{b}");
}

#[test]
fn fibers_grow_from_the_masked_area() {
    let mut s = Session::default();
    let head = add(&mut s, "sphere", json!({"detail": 4}));
    call(&mut s, "mask_shape", json!({"view": "front", "shape": "box", "points": [[0, 0], [1, 0.4]], "width": 640, "height": 640}));
    let r = call(&mut s, "grow_fibers", json!({"id": head, "count": 300, "droop": 0.5, "color": "#553311"}));
    assert_eq!(r["fibers"], 300);
    let fibers = r["object"]["id"].as_u64().unwrap() as u32;
    let mesh = &s.scene.get(fibers).unwrap().mesh;
    // All from the top of the head, painted.
    assert!(r["object"]["bounds"]["min"][1].as_f64().unwrap() > -0.2, "{}", r["object"]["bounds"]);
    assert!(mesh.has_paint());
    assert_eq!(call(&mut s, "get_scene", json!({}))["objects"].as_array().unwrap().len(), 2);
}

#[test]
fn path_render_lights_glow_and_shadows() {
    let mut s = Session::default();
    let ball = add(&mut s, "sphere", json!({"detail": 3, "size": 0.4}));
    call(&mut s, "paint_fill", json!({"id": ball, "color": "#ffffff", "emissive": 1}));
    let floor = add(&mut s, "plane", json!({"size": 4}));
    call(&mut s, "transform_object", json!({"id": floor, "translate": [0, -0.4, 0]}));
    let r = call(&mut s, "get_view", json!({"render": "path", "samples": 8, "bounces": 3, "width": 96, "height": 96, "view": "front"}));
    assert!(r["png_base64"].as_str().unwrap().len() > 1000);
    assert!(r["coverage"].as_f64().unwrap() > 0.1);
    assert_eq!(fails(&mut s, "get_view", json!({"render": "raytrace"})), "bad_arguments");
}

#[test]
fn manual_retopology_builds_a_joined_quad_mesh() {
    let mut s = Session::default();
    let target = add(&mut s, "sphere", json!({"detail": 4}));
    let r = call(&mut s, "retopo_face", json!({"target": target, "points": [[-0.3, 0.3, 1.1], [0.3, 0.3, 1.1], [0.3, -0.3, 1.1], [-0.3, -0.3, 1.1]]}));
    let retopo = r["id"].as_u64().unwrap();
    assert_eq!(call(&mut s, "get_scene", json!({}))["active"].as_u64().unwrap(), target, "the sculpt stays active");
    // A second quad sharing an edge reuses its two corners.
    let r = call(&mut s, "retopo_face", json!({"target": target, "into": retopo, "points": [[0.3, 0.3, 1.1], [0.8, 0.3, 0.8], [0.8, -0.3, 0.8], [0.3, -0.3, 1.1]]}));
    assert_eq!(r["faces"], 2);
    assert_eq!(r["quads"], true);
    assert_eq!(r["object"]["vertices"], 6);
    // Corners lie on the sphere and face outward.
    let mesh = &s.scene.get(retopo as u32).unwrap().mesh;
    assert!(mesh.positions.iter().all(|p| ((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - 1.0).abs() < 0.02));
    assert!(mesh.normals.iter().zip(&mesh.positions).all(|(n, p)| n[0] * p[0] + n[1] * p[1] + n[2] * p[2] > 0.0));
}

#[test]
fn edge_loops_split_a_ring_and_bridges_join_borders() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    assert_eq!(fails(&mut s, "insert_edge_loop", json!({"id": id, "point": [1, 0, 0]})), "not_quads");
    call(&mut s, "quad_remesh", json!({"id": id, "faces": 600}));
    let before = object(&mut s, id)["triangles"].as_u64().unwrap();
    let r = call(&mut s, "insert_edge_loop", json!({"id": id, "point": [1, 0, 0]}));
    let split = r["quads_split"].as_u64().unwrap();
    assert!(split >= 8, "a ring around the sphere, not one quad: {split}");
    assert_eq!(r["object"]["triangles"].as_u64().unwrap(), before + 2 * split);
    assert!(watertight(&mut s, id));
    // Bridge: a cylinder with both caps deleted is a tube with two borders; bridge them back into a ring... here,
    // two separate planes stacked make two borders of equal count that a bridge joins into a closed band.
    let mut s = Session::default();
    let a = add(&mut s, "plane", json!({"detail": 4}));
    let b = add(&mut s, "plane", json!({"detail": 4}));
    call(&mut s, "transform_object", json!({"id": b, "translate": [0, 1, 0], "rotate": [180, 0, 0]}));
    let merged = call(&mut s, "merge_objects", json!({"ids": [a, b]}));
    let m = merged["object"]["id"].as_u64().or(merged["id"].as_u64()).unwrap();
    let r = call(&mut s, "bridge", json!({"id": m, "segments": 3}));
    assert_eq!(r["open_borders_left"], 0);
    assert!(watertight(&mut s, m));
}

#[test]
fn delete_faces_opens_the_mesh() {
    let mut s = Session::default();
    let id = add(&mut s, "cube", json!({}));
    assert_eq!(fails(&mut s, "delete_faces", json!({"id": id})), "nothing_hidden");
    call(&mut s, "mask_shape", json!({"view": "front", "shape": "box", "points": [[0, 0], [1, 0.35]], "width": 640, "height": 640}));
    let r = call(&mut s, "delete_faces", json!({"id": id, "which": "masked"}));
    assert!(r["deleted"].as_u64().unwrap() > 0);
    assert!(!watertight(&mut s, id));
}

#[test]
fn substance_package_has_low_high_maps_and_steps() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    let dir = temp("substance");
    assert_eq!(fails(&mut s, "send_to_substance", json!({"folder": dir})), "no_uvs");
    call(&mut s, "multires_divide", json!({"id": id}));
    call(&mut s, "multires_set_level", json!({"id": id, "level": 0}));
    call(&mut s, "uv_unwrap", json!({"id": id}));
    call(&mut s, "bake_maps", json!({"id": id, "size": 512}));
    call(&mut s, "multires_set_level", json!({"id": id, "level": 1}));
    let r = call(&mut s, "send_to_substance", json!({"id": id, "folder": dir}));
    let files: Vec<String> = r["files"].as_array().unwrap().iter().map(|f| f.as_str().unwrap().rsplit('/').next().unwrap().to_string()).collect();
    for want in ["Sphere_low.obj", "Sphere_high.obj", "Sphere_BaseColor.png", "Sphere_Normal.png", "Sphere_README.txt"] {
        assert!(files.iter().any(|f| f == want), "{want} missing from {files:?}");
    }
    assert!(r["high_triangles"].as_u64().unwrap() == 4 * r["low_triangles"].as_u64().unwrap());
    let low = std::fs::read_to_string(std::path::Path::new(&dir).join("Sphere_low.obj")).unwrap();
    assert!(low.contains("\nvt ") && low.contains("/"), "the low mesh carries UVs");
}
