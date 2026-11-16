//! Bad arguments are refused without touching the scene, live stroke samples that are not numbers leave every vertex
//! finite, undo after a topology, UV or level step brings a painted texture back, and a command that would swap the
//! mesh under subdivision levels is refused with the levels still working.

mod common;

use common::*;
use fsc_commands::{dispatch, live, Session};
use serde_json::{json, Value};

type Snapshot = Vec<(u32, Vec<[f32; 3]>, Vec<u32>)>;

fn snapshot(s: &Session) -> Snapshot {
    s.scene.objects.iter().map(|o| (o.id, o.mesh.positions.clone(), o.mesh.indices.clone())).collect()
}

fn steps(s: &mut Session) -> Vec<String> {
    call(s, "get_history", json!({}))["steps"].as_array().expect("steps").iter().map(|v| v.as_str().unwrap_or_default().to_string()).collect()
}

fn finite(s: &Session) -> bool {
    s.scene.objects.iter().all(|o| o.mesh.positions.iter().all(|p| p.iter().all(|c| c.is_finite())) && o.mesh.normals.iter().all(|n| n.iter().all(|c| c.is_finite())))
}

/// The command is refused and the scene, its history and the active object are as they were; returns the error code.
fn refused_and_untouched(s: &mut Session, name: &str, args: Value) -> &'static str {
    let before = snapshot(s);
    let history = steps(s);
    let active = s.scene.active;
    let code = fails(s, name, args.clone());
    assert_eq!(snapshot(s), before, "{name} {args} changed the scene");
    assert_eq!(steps(s), history, "{name} {args} added a history step");
    assert_eq!(s.scene.active, active, "{name} {args} changed the active object");
    code
}

fn bad_arguments(s: &mut Session, name: &str, args: Value) {
    let code = refused_and_untouched(s, name, args.clone());
    assert_eq!(code, "bad_arguments", "{name} {args}");
}

#[test]
fn a_stroke_with_no_points_is_refused() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 2}));
    for brush in ["draw", "grab", "smooth", "inflate"] {
        bad_arguments(&mut s, "sculpt_stroke", json!({"id": id, "brush": brush, "points": [], "radius": 0.3, "strength": 1}));
    }
    call(&mut s, "sculpt_stroke", json!({"id": id, "brush": "draw", "points": [[0, 0, 1], [0.1, 0, 1]], "radius": 0.3, "strength": 1}));
}

#[test]
fn a_stroke_point_far_too_large_is_refused() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 2}));
    for brush in ["draw", "grab"] {
        bad_arguments(&mut s, "sculpt_stroke", json!({"id": id, "brush": brush, "points": [[0, 0, 1], [1e39, 0, 0]], "radius": 0.3, "strength": 1}));
    }
    assert!(finite(&s));
}

#[test]
fn merge_with_the_same_id_twice_is_refused() {
    let mut s = Session::default();
    let a = add(&mut s, "sphere", json!({"detail": 2}));
    add(&mut s, "cube", json!({}));
    bad_arguments(&mut s, "merge_objects", json!({"ids": [a, a]}));
    assert_eq!(s.scene.objects.len(), 2);
}

#[test]
fn an_object_id_that_is_not_a_whole_number_is_refused() {
    let mut s = Session::default();
    let a = add(&mut s, "sphere", json!({"detail": 2}));
    add(&mut s, "cube", json!({}));
    for bad in [json!(a as f64 + 0.5), json!("x"), json!(4_294_967_296u64 + a)] {
        bad_arguments(&mut s, "mask_invert", json!({"id": bad}));
        bad_arguments(&mut s, "delete_object", json!({"id": bad}));
        bad_arguments(&mut s, "transform_object", json!({"id": bad, "translate": [1, 0, 0]}));
    }
    for bad in [json!(-1), json!(true), json!([a]), json!(1e300)] {
        for (name, args) in [("mask_invert", json!({"id": bad})), ("delete_object", json!({"id": bad})), ("transform_object", json!({"id": bad, "translate": [1, 0, 0]}))] {
            refused_and_untouched(&mut s, name, args);
        }
    }
    assert_eq!(s.scene.objects.len(), 2);
}

#[test]
fn a_whole_number_written_as_a_float_names_that_object() {
    let mut s = Session::default();
    let a = add(&mut s, "sphere", json!({"detail": 2}));
    let b = add(&mut s, "cube", json!({}));
    assert_eq!(s.scene.active, Some(b as u32));
    let r = call(&mut s, "mask_invert", json!({"id": a as f64}));
    assert_eq!(r["id"].as_u64(), Some(a));
}

#[test]
fn insert_along_with_a_tiny_size_is_refused() {
    let mut s = Session::default();
    add(&mut s, "sphere", json!({"detail": 2}));
    let before = snapshot(&s);
    let history = steps(&mut s);
    let path = json!([[0, 0, 1, 0, 0, 1], [1, 0, 1, 0, 0, 1]]);
    assert!(dispatch(&mut s, "insert_along", &json!({"path": path, "size": 1e-8})).is_err());
    assert_eq!(snapshot(&s), before);
    assert_eq!(steps(&mut s), history);
    call(&mut s, "insert_along", json!({"path": path, "size": 0.2}));
}

#[test]
fn insert_along_without_merge_is_one_undo_step() {
    let mut s = Session::default();
    add(&mut s, "sphere", json!({"detail": 2}));
    let history = steps(&mut s).len();
    call(&mut s, "insert_along", json!({"path": [[0, 0, 1, 0, 0, 1], [1, 0, 1, 0, 0, 1]], "size": 0.05, "merge": false}));
    assert!(s.scene.objects.len() > 3, "expected several copies");
    assert_eq!(steps(&mut s).len(), history + 1);
    call(&mut s, "undo", json!({}));
    assert_eq!(s.scene.objects.len(), 1);
}

#[test]
fn paint_layer_commands_after_a_topology_change_are_refused() {
    for (name, args) in [("paint_layer_set", json!({"index": 0, "opacity": 0.5})), ("paint_layer_delete", json!({"index": 0})), ("paint_layer_merge", json!({"index": 0}))] {
        let mut s = Session::default();
        add(&mut s, "sphere", json!({"detail": 2}));
        call(&mut s, "paint_layer_add", json!({}));
        call(&mut s, "remesh_voxel", json!({"resolution": 16}));
        let before = snapshot(&s);
        let code = fails(&mut s, name, args);
        assert_eq!(code, "no_layer", "{name}");
        assert_eq!(snapshot(&s), before, "{name}");
    }
}

#[test]
fn live_stroke_samples_that_are_not_numbers_leave_every_vertex_finite() {
    let n = f32::NAN;
    let inf = f32::INFINITY;
    let bad: [([f32; 3], [f32; 3], f32); 6] = [
        ([0., 0., 5.], [0., 0., -1.], n),
        ([n, 0., 5.], [0., 0., -1.], 1.),
        ([0., 0., 5.], [0., n, -1.], 1.),
        ([0., 0., 5.], [0., 0., 0.], 1.),
        ([inf, 0., 5.], [0., 0., -1.], 1.),
        ([0., 0., 5.], [0., 0., -1.], inf),
    ];
    for brush in ["draw", "grab", "smooth", "inflate", "snake_hook"] {
        for (origin, dir, pressure) in bad {
            for first in [true, false] {
                let mut s = Session::default();
                let id = add(&mut s, "sphere", json!({"detail": 3}));
                live::stroke_begin(&mut s, id as u32, &json!({"brush": brush, "radius": 0.4, "strength": 1})).expect("begin");
                if !first {
                    let _ = live::stroke_sample(&mut s, [-0.3, 0., 5.], [0., 0., -1.], 1.0, 0.0);
                }
                let _ = live::stroke_sample(&mut s, origin, dir, pressure, 0.0);
                let _ = live::stroke_sample(&mut s, [0.3, 0., 5.], [0., 0., -1.], 1.0, 0.0);
                live::stroke_end(&mut s);
                assert!(finite(&s), "{brush}: origin {origin:?} dir {dir:?} pressure {pressure} (first sample: {first}) left a vertex that is not finite");
            }
        }
    }
}

fn sphere(s: &mut Session) -> u64 {
    add(s, "sphere", json!({"detail": 2}))
}

fn textured() -> (Session, u64) {
    let mut s = Session::default();
    let id = sphere(&mut s);
    call(&mut s, "uv_unwrap", json!({"id": id}));
    call(&mut s, "texture_create", json!({"id": id, "size": 256}));
    call(&mut s, "paint_fill", json!({"id": id, "color": "#ff0000", "target": "texture"}));
    (s, id)
}

fn picture(s: &mut Session, id: u64) -> String {
    call(s, "get_texture", json!({"id": id, "size": 64}))["png_base64"].as_str().expect("png").to_string()
}

#[test]
fn undo_after_a_topology_uv_or_level_step_brings_the_painted_texture_back() {
    for (name, args) in [
        ("remesh_voxel", json!({"resolution": 16})),
        ("uv_pack", json!({})),
        ("multires_divide", json!({})),
        ("subdivide", json!({})),
        ("decimate", json!({"ratio": 0.5})),
        ("uv_unwrap", json!({"method": "project"})),
        ("uv_relax", json!({})),
    ] {
        let (mut s, id) = textured();
        let before = picture(&mut s, id);
        let history = steps(&mut s).len();
        let mut a = args.clone();
        a["id"] = json!(id);
        call(&mut s, name, a);
        call(&mut s, "undo", json!({}));
        assert_eq!(object(&mut s, id)["texture_size"], 256, "{name}: undo did not bring the texture back");
        assert_eq!(picture(&mut s, id), before, "{name}: the texture after undo is not the painted one");
        assert_eq!(steps(&mut s).len(), history + 1, "{name}: undo dropped a step");
        call(&mut s, "undo", json!({}));
        call(&mut s, "redo", json!({}));
        call(&mut s, "redo", json!({}));
        call(&mut s, "undo", json!({}));
        assert_eq!(picture(&mut s, id), before, "{name}: undo after redo lost the texture");
    }
}

#[test]
fn undo_of_a_level_change_brings_the_texture_painted_on_the_lower_level_back() {
    let mut s = Session::default();
    let id = sphere(&mut s);
    call(&mut s, "multires_divide", json!({"id": id}));
    call(&mut s, "multires_set_level", json!({"id": id, "level": 0}));
    call(&mut s, "uv_unwrap", json!({"id": id}));
    call(&mut s, "texture_create", json!({"id": id, "size": 256}));
    call(&mut s, "paint_fill", json!({"id": id, "color": "#00ff00", "target": "texture"}));
    let before = picture(&mut s, id);
    call(&mut s, "multires_set_level", json!({"id": id, "level": 1}));
    call(&mut s, "undo", json!({}));
    assert_eq!(picture(&mut s, id), before);
}

fn divided() -> (Session, u64) {
    let mut s = Session::default();
    let id = sphere(&mut s);
    call(&mut s, "multires_divide", json!({"id": id}));
    (s, id)
}

fn levels_still_step(s: &mut Session, id: u64, what: &str) {
    call(s, "multires_set_level", json!({"id": id, "level": 0}));
    call(s, "multires_set_level", json!({"id": id, "level": 1}));
    assert!(finite(s), "{what}");
}

#[test]
fn retopology_into_an_object_with_levels_is_refused_and_the_levels_still_step() {
    let (mut s, id) = divided();
    let before = snapshot(&s);
    let face = json!({"target": id, "into": id, "points": [[0, 0, 1], [0.3, 0, 0.95], [0, 0.3, 0.95]], "weld": 0.0001});
    assert_eq!(fails(&mut s, "retopo_face", face), "has_levels");
    assert_eq!(fails(&mut s, "retopo_patch", json!({"target": id, "into": id, "patch": "grid", "center": [0, 0, 1]})), "has_levels");
    assert_eq!(snapshot(&s), before);
    levels_still_step(&mut s, id, "retopology");
}

#[test]
fn grooming_fibers_that_have_levels_is_refused_and_the_levels_still_step() {
    let mut s = Session::default();
    sphere(&mut s);
    call(&mut s, "grow_fibers", json!({"count": 20}));
    let fibers = s.scene.objects.iter().map(|o| o.id).max().unwrap() as u64;
    call(&mut s, "multires_divide", json!({"id": fibers}));
    let code = fails(&mut s, "groom_fibers", json!({"id": fibers, "mode": "cut", "points": [[0, 0, 1.1], [0, 0.2, 1.1]], "radius": 2}));
    assert_eq!(code, "has_levels");
    levels_still_step(&mut s, fibers, "groom");
}

#[test]
fn a_bridge_file_changed_outside_does_not_replace_a_mesh_with_levels() {
    let (mut s, id) = divided();
    let cube = add(&mut s, "cube", json!({"detail": 2}));
    let path = temp("levels-bridge.obj");
    call(&mut s, "bridge_send", json!({"id": id, "path": path}));
    let before = snapshot(&s);
    call(&mut s, "export_mesh", json!({"ids": [cube], "path": path}));
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options().write(true).open(&path).unwrap().set_modified(later).unwrap();
    let sync = call(&mut s, "bridge_sync", json!({}));
    assert!(sync["problems"].as_array().is_some_and(|p| !p.is_empty()), "no problem reported: {sync}");
    assert!(sync["updated"].as_array().is_none_or(|u| u.is_empty()), "the mesh was replaced: {sync}");
    assert_eq!(snapshot(&s), before);
    levels_still_step(&mut s, id, "bridge");
}
