//! Behaviour of the modeling commands, measured the way the HASHOUT logs describe them.

mod common;

use common::*;
use fsc_commands::Session;
use serde_json::json;

#[test]
fn every_primitive_is_closed_and_faces_out() {
    let mut s = Session::default();
    for kind in ["sphere", "uv_sphere", "cube", "cylinder", "cone", "torus"] {
        let id = add(&mut s, kind, json!({}));
        let (ok, problems) = printable(&mut s, id);
        assert!(ok, "{kind} is not printable: {problems:?}");
    }
    // The plane is open by nature, but it must face up: a draw stroke raises it.
    let plane = add(&mut s, "plane", json!({"size": 2, "detail": 48}));
    call(&mut s, "sculpt_stroke", json!({"id": plane, "brush": "draw", "points": [[-0.3, 0, 0], [0.3, 0, 0]], "radius": 0.3, "strength": 1}));
    let o = object(&mut s, plane);
    assert!(o["bounds"]["max"][1].as_f64().unwrap() > 0.05, "a draw stroke did not raise the plane");
    assert!(o["bounds"]["min"][1].as_f64().unwrap() > -1e-4, "a draw stroke pushed the plane down");
}

#[test]
fn a_stroke_is_one_undo_step_and_redo_restores_it() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 5}));
    let before = object(&mut s, id)["bounds"]["max"][2].as_f64().unwrap();
    call(&mut s, "sculpt_stroke", json!({"id": id, "brush": "inflate", "points": [[0, 0, 1]], "radius": 0.5, "strength": 1}));
    let raised = object(&mut s, id)["bounds"]["max"][2].as_f64().unwrap();
    assert!(raised > before + 1e-3);
    assert_eq!(call(&mut s, "undo", json!({}))["undone"], "Inflate Stroke");
    assert!((object(&mut s, id)["bounds"]["max"][2].as_f64().unwrap() - before).abs() < 1e-6);
    call(&mut s, "redo", json!({}));
    assert!((object(&mut s, id)["bounds"]["max"][2].as_f64().unwrap() - raised).abs() < 1e-6);
}

#[test]
fn booleans_are_watertight_and_blend_fuses() {
    let mut sharp = Session::default();
    let mut soft = Session::default();
    let mut necks = Vec::new();
    for (s, blend) in [(&mut sharp, 0.0), (&mut soft, 0.3)] {
        let a = add(s, "sphere", json!({"detail": 4}));
        let b = add(s, "cylinder", json!({"size": 0.3, "height": 2.4}));
        call(s, "transform_object", json!({"id": b, "rotate": [0, 0, 90], "translate": [1.2, 0, 0]}));
        let r = call(s, "boolean", json!({"a": a, "b": b, "op": "union", "blend": blend, "resolution": 96}));
        let id = r["id"].as_u64().unwrap();
        assert!(watertight(s, id));
        assert_eq!(call(s, "get_scene", json!({}))["objects"].as_array().unwrap().len(), 1);
        // The rod's top surface just outside the sphere: the fillet raises it.
        let hit = call(s, "pick", json!({"origin": [1.08, 5, 0], "direction": [0, -1, 0]}));
        necks.push(hit["point"][1].as_f64().unwrap());
    }
    assert!((necks[0] - 0.3).abs() < 0.03, "the sharp union's rod top is at {}", necks[0]);
    assert!(necks[1] > necks[0] + 0.03, "the blend did not fillet the joint: {necks:?}");
}

#[test]
fn cut_shape_makes_a_through_hole_and_trim_keeps_the_inside() {
    let mut s = Session::default();
    let cube = add(&mut s, "cube", json!({}));
    let square = json!([[-0.2, -0.2, 0.9], [0.2, -0.2, 0.9], [0.2, 0.2, 0.9], [-0.2, 0.2, 0.9]]);
    call(&mut s, "cut_shape", json!({"id": cube, "outline": square, "direction": [0, 0, -1], "resolution": 96}));
    assert!(watertight(&mut s, cube));
    let hit = call(&mut s, "pick", json!({"origin": [0, 0, 5], "direction": [0, 0, -1]}));
    assert_eq!(hit["hit"], false, "a ray through the cut still hits the cube");
    call(&mut s, "undo", json!({}));
    call(&mut s, "cut_shape", json!({"id": cube, "outline": square, "direction": [0, 0, -1], "keep": "inside", "resolution": 96}));
    let o = object(&mut s, cube);
    assert!(size(&o, 0) < 0.5 && size(&o, 2) > 0.95, "trim kept the wrong part: {o}");
    assert_eq!(fails(&mut s, "cut_shape", json!({"id": cube, "outline": [[0, 0, 0], [1, 0, 0]], "direction": [0, 0, -1]})), "bad_arguments");
}

#[test]
fn text_and_shapes_are_closed_solids() {
    let mut s = Session::default();
    let r = call(&mut s, "text_mesh", json!({"text": "FS 21", "size": 1}));
    let id = r["object"]["id"].as_u64().unwrap();
    assert!(watertight(&mut s, id));
    assert!((size(&r["object"], 1) - 1.0).abs() < 0.3);
    assert_eq!(fails(&mut s, "text_mesh", json!({"text": "a@b"})), "build_failed");
    let ring: Vec<[f64; 2]> = (0..48).map(|t| { let a = t as f64 / 48.0 * std::f64::consts::TAU; [a.cos(), a.sin()] }).collect();
    let hole: Vec<[f64; 2]> = ring.iter().map(|p| [p[0] * 0.5, p[1] * 0.5]).collect();
    let r = call(&mut s, "extrude_shape", json!({"outlines": [ring, hole], "depth": 0.3, "round": 0.05}));
    let washer = r["object"]["id"].as_u64().unwrap();
    assert!(watertight(&mut s, washer));
    call(&mut s, "set_visibility", json!({"action": "isolate", "ids": [washer]}));
    assert_eq!(call(&mut s, "pick", json!({"origin": [0, 0, 5], "direction": [0, 0, -1]}))["hit"], false, "the washer has no hole");
}

#[test]
fn vector_displacement_overhangs_and_capture_round_trips() {
    let mut s = Session::default();
    let plane = add(&mut s, "plane", json!({"size": 2, "detail": 160}));
    call(&mut s, "vdm_stamp", json!({"id": plane, "vdm": "hook", "point": [0, 0, 0], "normal": [0, 1, 0], "radius": 0.6}));
    let o = object(&mut s, plane);
    assert!(o["bounds"]["max"][1].as_f64().unwrap() > 0.5);
    assert_eq!(fails(&mut s, "vdm_stamp", json!({"vdm": "nope", "point": [0, 0, 0], "normal": [0, 1, 0]})), "not_found");
    // Capture a bump sculpted on a fresh plane, then stamp it on a sphere's pole.
    let p2 = add(&mut s, "plane", json!({"size": 2, "detail": 64}));
    assert_eq!(fails(&mut s, "vdm_capture", json!({"id": p2, "name": "bump"})), "nothing_sculpted");
    call(&mut s, "sculpt_stroke", json!({"id": p2, "brush": "draw", "points": [[-0.3, 0, 0], [0.3, 0, 0]], "radius": 0.3, "strength": 1}));
    call(&mut s, "vdm_capture", json!({"id": p2, "name": "bump"}));
    let sph = add(&mut s, "sphere", json!({"detail": 6}));
    call(&mut s, "vdm_stamp", json!({"id": sph, "vdm": "bump", "point": [0, 1, 0], "normal": [0, 1, 0], "radius": 0.4}));
    assert!(object(&mut s, sph)["bounds"]["max"][1].as_f64().unwrap() > 1.01);
}

#[test]
fn insert_along_spaces_copies_on_the_stroke() {
    let mut s = Session::default();
    add(&mut s, "sphere", json!({"detail": 4}));
    let path: Vec<[f64; 6]> = (0..=40).map(|i| { let a = i as f64 * std::f64::consts::PI / 40.0; [a.cos(), 0.0, a.sin(), a.cos(), 0.0, a.sin()] }).collect();
    let r = call(&mut s, "insert_along", json!({"path": path, "kind": "cone", "size": 0.15, "spacing": 1.5}));
    // Half the unit circle is about 3.14 long; one copy every 0.225.
    let copies = r["copies"].as_u64().unwrap();
    assert!((13..=15).contains(&copies), "{copies} copies");
    let r = call(&mut s, "insert_along", json!({"path": &path[..10], "kind": "cube", "size": 0.1, "merge": false}));
    assert_eq!(r["ids"].as_array().unwrap().len() as u64, r["copies"].as_u64().unwrap());
}

#[test]
fn face_groups_by_paint_uv_and_brush() {
    let mut s = Session::default();
    let sph = add(&mut s, "sphere", json!({"detail": 5}));
    call(&mut s, "paint_fill", json!({"id": sph, "color": "#888888"}));
    call(&mut s, "sculpt_stroke", json!({"id": sph, "brush": "paint", "color": "#ff0000", "points": [[0, 0, 1], [0, 0.3, 0.95]], "radius": 0.25, "strength": 1}));
    call(&mut s, "sculpt_stroke", json!({"id": sph, "brush": "paint", "color": "#00ff00", "points": [[0, 0, -1]], "radius": 0.2, "strength": 1}));
    assert_eq!(call(&mut s, "group_auto", json!({"id": sph, "by": "paint"}))["groups"], 3);
    let cube = add(&mut s, "cube", json!({}));
    assert_eq!(call(&mut s, "group_auto", json!({"id": cube, "by": "uv"}))["groups"], 6);
    assert_eq!(call(&mut s, "group_auto", json!({"id": cube, "by": "angle"}))["groups"], 6);
    assert_eq!(fails(&mut s, "group_auto", json!({"id": cube, "by": "paint"})), "no_paint");
    let r = call(&mut s, "group_paint", json!({"id": sph, "points": [[1, 0, 0], [0.7, 0.7, 0]], "radius": 0.15}));
    assert!(r["faces"].as_u64().unwrap() > 50);
    assert_eq!(r["object"]["groups"], 4);
}

#[test]
fn arrays_mirror_and_linked_copies_follow() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    call(&mut s, "transform_object", json!({"id": id, "translate": [1, 0, 0]}));
    call(&mut s, "array_object", json!({"id": id, "mode": "mirror", "axis": "x"}));
    let objs = call(&mut s, "get_scene", json!({}))["objects"].clone();
    let copy = objs.as_array().unwrap().last().unwrap();
    let cx = (copy["bounds"]["max"][0].as_f64().unwrap() + copy["bounds"]["min"][0].as_f64().unwrap()) / 2.0;
    assert!((cx + 1.0).abs() < 1e-3, "mirror copy centred at {cx}");
    let src = add(&mut s, "sphere", json!({"detail": 3}));
    call(&mut s, "array_object", json!({"id": src, "count": 3, "linked": true}));
    let linked: Vec<u64> = call(&mut s, "get_scene", json!({}))["objects"].as_array().unwrap().iter().filter(|o| o["instance_of"] == src).map(|o| o["id"].as_u64().unwrap()).collect();
    assert_eq!(linked.len(), 2);
    let rev = object(&mut s, linked[0])["revision"].as_u64().unwrap();
    call(&mut s, "subdivide", json!({"id": src}));
    let copy = object(&mut s, linked[0]);
    assert!(copy["revision"].as_u64().unwrap() > rev);
    assert_eq!(copy["vertices"], object(&mut s, src)["vertices"]);
}

#[test]
fn shadow_box_needs_two_overlapping_outlines() {
    let mut s = Session::default();
    let rect = json!([[-0.5, 0], [0.5, 0], [0.5, 1], [-0.5, 1]]);
    let r = call(&mut s, "shadow_box", json!({"front": rect, "side": rect, "resolution": 64}));
    assert!(watertight(&mut s, r["object"]["id"].as_u64().unwrap()));
    assert_eq!(fails(&mut s, "shadow_box", json!({"front": rect})), "build_failed");
}
