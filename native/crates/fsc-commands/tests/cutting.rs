//! Guided quad remesh, plane clips and slices, edge bevels, UV point moves, the Substance round trip, skeleton editing
//! and the path renderer's lights, depth of field and passes.

mod common;

use common::*;
use fsc_commands::Session;
use serde_json::json;

/// Share of a quad mesh's quad edges (not the diagonals splitting each quad; on the front face) running within 12
/// degrees of a direction.
fn aligned_share(s: &Session, id: u64, dir: [f32; 3]) -> f32 {
    let mesh = &s.scene.get(id as u32).unwrap().mesh;
    let d = glam::Vec3::from(dir).normalize();
    let (mut hit, mut all) = (0, 0);
    for q in mesh.indices.chunks(6) {
        let edges: Vec<(u32, u32)> = (0..6).map(|k| (q[k], q[if k % 3 == 2 { k - 2 } else { k + 1 }])).collect();
        let diagonal = |e: &(u32, u32)| edges.iter().filter(|o| (o.0 == e.0 && o.1 == e.1) || (o.0 == e.1 && o.1 == e.0)).count() > 1;
        for e in edges.iter().filter(|e| !diagonal(e)) {
            let (a, b) = (mesh.position(e.0), mesh.position(e.1));
            if a.z < 0.45 || b.z < 0.45 {
                continue;
            }
            let e = (b - a).normalize_or_zero();
            all += 1;
            if e.dot(d).abs() > 12f32.to_radians().cos() {
                hit += 1;
            }
        }
    }
    hit as f32 / all.max(1) as f32
}

#[test]
fn guides_turn_the_quad_remesh() {
    let mut s = Session::default();
    let plain = add(&mut s, "cube", json!({"detail": 3}));
    call(&mut s, "quad_remesh", json!({"id": plain, "faces": 600}));
    let guided = add(&mut s, "cube", json!({"detail": 3}));
    call(&mut s, "quad_remesh", json!({"id": guided, "faces": 600, "guides": [[[-0.4, -0.4, 0.5], [0.0, 0.0, 0.5], [0.4, 0.4, 0.5]]]}));
    let diagonal = [1.0, 1.0, 0.0];
    let (before, after) = (aligned_share(&s, plain, diagonal), aligned_share(&s, guided, diagonal));
    assert!(after > before + 0.2, "the guided grid runs along the guide: {before} then {after}");
    assert!(watertight(&mut s, guided));
    assert_eq!(fails(&mut s, "quad_remesh", json!({"id": guided, "guides": [[[0, 0, 0], [0, 0, 0]]]})), "bad_arguments");
}

#[test]
fn clip_keeps_one_side_and_slice_makes_two() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    let r = call(&mut s, "clip_plane", json!({"id": id, "point": [0, 0.2, 0], "normal": [0, 1, 0]}));
    assert!((r["object"]["bounds"]["max"][1].as_f64().unwrap() - 0.2).abs() < 1e-3, "{}", r["object"]["bounds"]);
    assert!(watertight(&mut s, id));
    call(&mut s, "undo", json!({}));
    let r = call(&mut s, "clip_plane", json!({"id": id, "normal": [1, 0, 0], "mode": "slice"}));
    let other = r["slice"]["id"].as_u64().unwrap();
    assert!(r["object"]["bounds"]["max"][0].as_f64().unwrap() < 1e-3);
    assert!(object(&mut s, other)["bounds"]["min"][0].as_f64().unwrap() > -1e-3);
    assert!(watertight(&mut s, other));
}

#[test]
fn bevel_cuts_one_edge_flat_or_round() {
    let mut s = Session::default();
    let id = add(&mut s, "cube", json!({}));
    let size = |s: &Session| s.scene.get(id as u32).unwrap().mesh.bounds();
    let corner_gone = |s: &Session| {
        let m = &s.scene.get(id as u32).unwrap().mesh;
        // No vertex on the beveled edge (x = 0.5, y = 0.5) away from its ends.
        !m.positions.iter().any(|p| (p[0] - 0.5).abs() < 1e-4 && (p[1] - 0.5).abs() < 1e-4)
    };
    let b0 = size(&s);
    assert!((b0.max.x - 0.5).abs() < 1e-4, "a unit cube: {b0:?}");
    let r = call(&mut s, "bevel_edge", json!({"id": id, "point": [0.5, 0.5, 0.1], "width": 0.1}));
    assert!((r["edge"]["length"].as_f64().unwrap() - 1.0).abs() < 1e-3, "{}", r["edge"]);
    assert!(corner_gone(&s));
    assert!(watertight(&mut s, id));
    // The other edges stay sharp: the opposite top edge still has its corner line.
    assert!(s.scene.get(id as u32).unwrap().mesh.positions.iter().any(|p| (p[0] + 0.5).abs() < 1e-4 && (p[1] - 0.5).abs() < 1e-4));
    call(&mut s, "undo", json!({}));
    let r = call(&mut s, "bevel_edge", json!({"id": id, "point": [0.5, 0.5, 0], "width": 0.15, "mode": "round", "segments": 6}));
    assert!(r["object"]["vertices"].as_u64().unwrap() > 8);
    assert!(corner_gone(&s));
    assert!(watertight(&mut s, id));
    assert_eq!(fails(&mut s, "bevel_edge", json!({"id": id, "point": [5, 5, 5]})), "no_edge");
}

#[test]
fn uv_points_move_with_falloff() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    assert_eq!(fails(&mut s, "uv_move_point", json!({"at": [0.5, 0.5], "to": [0.6, 0.5]})), "no_uvs");
    call(&mut s, "uv_unwrap", json!({"id": id}));
    let before = s.scene.get(id as u32).unwrap().mesh.uvs.clone();
    let at = before[0];
    let r = call(&mut s, "uv_move_point", json!({"id": id, "at": at, "to": [at[0] + 0.01, at[1]]}));
    let one = r["corners_moved"].as_u64().unwrap();
    assert!((1..12).contains(&one), "one UV vertex: {one}");
    let r = call(&mut s, "uv_move_point", json!({"id": id, "at": at, "to": [at[0] + 0.01, at[1]], "radius": 0.08}));
    assert!(r["corners_moved"].as_u64().unwrap() > one);
    call(&mut s, "undo", json!({}));
    call(&mut s, "undo", json!({}));
    assert_eq!(s.scene.get(id as u32).unwrap().mesh.uvs, before);
}

#[test]
fn substance_textures_come_back_as_paint() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    call(&mut s, "uv_unwrap", json!({"id": id}));
    let dir = temp("substance-sync");
    call(&mut s, "send_to_substance", json!({"id": id, "folder": dir}));
    assert_eq!(fails(&mut s, "substance_sync", json!({"id": id, "folder": dir})), "io_failed");
    // What Painter exports: a red base colour and a mid roughness.
    let red = fsc_paint::png_rgba(64, &vec![[255, 0, 0, 255]; 64 * 64]).unwrap();
    std::fs::write(std::path::Path::new(&dir).join("Sphere_BaseColor.png"), red).unwrap();
    let r = call(&mut s, "substance_sync", json!({"id": id, "folder": dir}));
    assert_eq!(r["vertices_painted"].as_u64().unwrap(), r["object"]["vertices"].as_u64().unwrap());
    let mesh = &s.scene.get(id as u32).unwrap().mesh;
    assert!(mesh.paint.iter().all(|p| p[0] > 0.99 && p[1] < 0.01));
}

#[test]
fn skeleton_grows_and_moves_joints() {
    let mut s = Session::default();
    let r = call(&mut s, "skeleton_mesh", json!({"spheres": [[0, 0, 0, 0.3], [0, 1, 0, 0.25]], "links": [[0, 1]], "resolution": 48}));
    let id = r["object"]["id"].as_u64().unwrap();
    let r = call(&mut s, "edit_build", json!({"id": id, "add": {"point": [0.8, 1.1, 0]}}));
    assert_eq!(r["changes"]["linked_to"], 1);
    assert!(r["object"]["bounds"]["max"][0].as_f64().unwrap() > 0.9, "the new arm reaches out: {}", r["object"]["bounds"]);
    let r = call(&mut s, "edit_build", json!({"id": id, "drag": {"from": [0, 0, 0], "to": [0, -1, 0]}}));
    assert_eq!(r["changes"]["moved"], 0);
    assert!(r["object"]["bounds"]["min"][1].as_f64().unwrap() < -1.2);
    assert_eq!(r["spheres"].as_array().unwrap().len(), 3);
    call(&mut s, "undo", json!({}));
    assert!(object(&mut s, id)["bounds"]["min"][1].as_f64().unwrap() > -0.4);
}

#[test]
fn path_render_passes_lights_and_focus() {
    let mut s = Session::default();
    add(&mut s, "sphere", json!({"detail": 3}));
    let base = json!({"render": "path", "samples": 4, "bounces": 2, "width": 64, "height": 64, "view": "front"});
    let with = |extra: serde_json::Value| {
        let mut a = base.clone();
        for (k, v) in extra.as_object().unwrap() {
            a[k] = v.clone();
        }
        a
    };
    for pass in ["depth", "normal", "ao"] {
        let r = call(&mut s, "get_view", with(json!({"pass": pass})));
        assert!(r["coverage"].as_f64().unwrap() > 0.2, "{pass}");
    }
    call(&mut s, "get_view", with(json!({"sun": false, "lights": [{"from": [-1, 1, 1], "color": [1, 0.5, 0.2], "intensity": 4}], "aperture": 0.05, "focus": 2.5})));
    assert_eq!(fails(&mut s, "get_view", with(json!({"pass": "albedo"}))), "bad_arguments");
}
