//! Posing a skeleton-built mesh by its joints, and grooming grown fibers.

mod common;

use common::*;
use fsc_commands::Session;
use serde_json::json;

#[test]
fn a_joint_turns_the_limb_beyond_it_and_undo_restores_the_rig() {
    let mut s = Session::default();
    // A body (0) with an upper arm (1) and forearm (2) along +x.
    let r = call(&mut s, "skeleton_mesh", json!({"spheres": [[0, 0, 0, 0.3], [0.6, 0, 0, 0.15], [1.2, 0, 0, 0.12]], "links": [[0, 1], [1, 2]], "resolution": 64}));
    let id = r["object"]["id"].as_u64().unwrap();
    let max_x = |s: &mut Session| object(s, id)["bounds"]["max"][0].as_f64().unwrap();
    let max_y = |s: &mut Session| object(s, id)["bounds"]["max"][1].as_f64().unwrap();
    let (x0, y0) = (max_x(&mut s), max_y(&mut s));
    // Sculpt first: the rig still works and the detail rides along.
    call(&mut s, "sculpt_stroke", json!({"id": id, "brush": "inflate", "points": [[1.2, 0, 0.12]], "radius": 0.05, "strength": 0.3}));
    let r = call(&mut s, "pose_joint", json!({"id": id, "point": [0.6, 0, 0], "axis": [0, 0, 1], "angle": 90}));
    assert_eq!(r["joint"], 1);
    assert_eq!(r["joints_moved"], 1);
    // The forearm now points up: the reach along x shrinks and the top rises; the body is where it was.
    assert!(max_x(&mut s) < x0 - 0.4, "{} from {x0}", max_x(&mut s));
    assert!(max_y(&mut s) > y0 + 0.4, "{} from {y0}", max_y(&mut s));
    assert!(object(&mut s, id)["bounds"]["min"][0].as_f64().unwrap() < -0.29);
    // A second pose starts from the turned skeleton: the shoulder turns the whole bent arm down, its elbow to
    // (0, -0.6) and its hand to (0.6, -0.6).
    call(&mut s, "pose_joint", json!({"id": id, "joint": 0, "axis": [0, 0, 1], "angle": -90}));
    assert!(object(&mut s, id)["bounds"]["min"][1].as_f64().unwrap() < -0.65);
    // Undo twice, then pose again: the rig is the one that belongs to the restored mesh.
    call(&mut s, "undo", json!({}));
    call(&mut s, "undo", json!({}));
    call(&mut s, "pose_joint", json!({"id": id, "joint": 1, "axis": [0, 0, 1], "angle": 90}));
    assert!(max_y(&mut s) > y0 + 0.4 && max_y(&mut s) < y0 + 0.9);
    assert_eq!(fails(&mut s, "pose_joint", json!({"id": id, "joint": 2, "angle": 30})), "no_rig");
    let plain = add(&mut s, "sphere", json!({}));
    assert_eq!(fails(&mut s, "pose_joint", json!({"id": plain, "joint": 0, "angle": 30})), "no_rig");
}

#[test]
fn fibers_comb_cut_curl_and_lengthen() {
    let mut s = Session::default();
    add(&mut s, "sphere", json!({"detail": 3}));
    call(&mut s, "mask_shape", json!({"view": "front", "shape": "box", "points": [[0, 0], [1, 0.3]], "width": 640, "height": 640}));
    let r = call(&mut s, "grow_fibers", json!({"count": 200, "droop": 0, "length": 0.4}));
    let fibers = r["object"]["id"].as_u64().unwrap();
    let top = |s: &mut Session| object(s, fibers)["bounds"]["max"][1].as_f64().unwrap();
    let t0 = top(&mut s);
    // Comb sideways across the top: the strands lean to +x and their top comes down.
    let r = call(&mut s, "groom_fibers", json!({"id": fibers, "mode": "comb", "points": [[-0.6, 1.3, 0], [0.6, 1.3, 0]], "radius": 0.6, "strength": 1}));
    assert!(r["fibers_groomed"].as_u64().unwrap() > 50);
    assert!(top(&mut s) < t0 - 0.05, "{} from {t0} bounds {}", top(&mut s), object(&mut s, fibers)["bounds"]);
    assert!(object(&mut s, fibers)["bounds"]["max"][0].as_f64().unwrap() > 0.6);
    call(&mut s, "undo", json!({}));
    assert!((top(&mut s) - t0).abs() < 1e-4);
    // Cut level across the crown: shorter.
    call(&mut s, "groom_fibers", json!({"id": fibers, "mode": "cut", "points": [[-1, 1.15, 0], [1, 1.15, 0]], "radius": 0.6, "strength": 0}));
    assert!(top(&mut s) < t0 - 0.05, "{} from {t0}", top(&mut s));
    // Lengthen and curl run too, starting from the cut strands.
    let cut = top(&mut s);
    call(&mut s, "groom_fibers", json!({"id": fibers, "mode": "lengthen", "points": [[-1, 1.15, 0], [1, 1.15, 0]], "radius": 2, "strength": 1}));
    assert!(top(&mut s) > cut + 0.02);
    call(&mut s, "groom_fibers", json!({"id": fibers, "mode": "curl", "points": [[0, 1.2, 0]], "radius": 0.6, "strength": 1}));
    assert_eq!(fails(&mut s, "groom_fibers", json!({"id": fibers, "mode": "comb", "points": [[9, 9, 9]], "radius": 0.1})), "nothing_groomed");
    assert_eq!(fails(&mut s, "groom_fibers", json!({"id": fibers, "mode": "braid", "points": [[0, 1, 0]]})), "bad_arguments");
}
