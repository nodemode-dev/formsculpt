//! Topology-changing and smoothing commands never leave NaN positions or non-manifold edges, undo restores a mesh to
//! the bit, and every primitive survives STL, OBJ and glTF export.

mod common;

use common::*;
use fsc_commands::Session;
use serde_json::{json, Value};

const CLOSED: [&str; 6] = ["sphere", "uv_sphere", "cube", "cylinder", "cone", "torus"];

fn mesh_of(s: &Session, id: u64) -> (Vec<[f32; 3]>, Vec<u32>) {
    let m = &s.scene.get(id as u32).expect("object").mesh;
    (m.positions.clone(), m.indices.clone())
}

fn assert_sound(s: &mut Session, id: u64, closed: bool, what: &str) {
    let (positions, indices) = mesh_of(s, id);
    assert!(!positions.is_empty() && !indices.is_empty(), "{what}: the mesh is empty");
    for (i, p) in positions.iter().enumerate() {
        assert!(p.iter().all(|c| c.is_finite()), "{what}: vertex {i} is {p:?}");
    }
    assert!(indices.len() % 3 == 0, "{what}: the index list is not whole triangles");
    assert!(indices.iter().all(|&i| (i as usize) < positions.len()), "{what}: an index is out of range");
    let normals = &s.scene.get(id as u32).expect("object").mesh.normals;
    assert!(normals.iter().all(|n| n.iter().all(|c| c.is_finite())), "{what}: a normal is not finite");
    let check = call(s, "mesh_check", json!({"id": id}));
    assert_eq!(check["non_manifold_edges"], 0, "{what}: non-manifold edges");
    if closed {
        assert_eq!(check["boundary_edges"], 0, "{what}: the mesh has open edges");
    }
}

fn stroke(s: &mut Session, id: u64, brush: &str, points: Value, extra: Value) {
    let mut args = json!({"id": id, "brush": brush, "points": points, "radius": 0.45, "strength": 1});
    if let (Some(a), Some(e)) = (args.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            a.insert(k.clone(), v.clone());
        }
    }
    call(s, "sculpt_stroke", args);
}

fn path_on_sphere() -> Value {
    json!([[0.0, 1.0, 0.0], [0.25, 0.96, 0.1], [0.5, 0.85, 0.15], [0.7, 0.7, 0.1], [0.9, 0.4, 0.1]])
}

#[test]
fn dynamic_topology_keeps_the_mesh_finite_and_manifold() {
    for mode in ["speed", "balanced", "quality"] {
        for method in ["subdivide", "collapse", "both"] {
            for brush in ["draw", "inflate", "smooth", "snake_hook", "pinch"] {
                let mut s = Session::default();
                let id = add(&mut s, "sphere", json!({"detail": 3}));
                let before = object(&mut s, id)["triangles"].as_u64().unwrap();
                stroke(&mut s, id, brush, path_on_sphere(), json!({"dyntopo": 0.2, "dyntopo_mode": mode, "dyntopo_method": method}));
                assert_sound(&mut s, id, true, &format!("dyntopo {mode}/{method}/{brush}"));
                if method != "collapse" && brush != "smooth" {
                    assert!(object(&mut s, id)["triangles"].as_u64().unwrap() > before, "dyntopo {mode}/{method}/{brush} added no detail");
                }
            }
        }
    }
}

#[test]
fn dynamic_topology_survives_a_long_dense_stroke_on_every_primitive() {
    for kind in CLOSED {
        let mut s = Session::default();
        let id = add(&mut s, kind, json!({"detail": 2}));
        let b = object(&mut s, id)["bounds"].clone();
        let top = b["max"][1].as_f64().unwrap();
        let points: Vec<Value> = (0..40).map(|i| json!([-0.4 + 0.02 * i as f64, top, 0.0])).collect();
        stroke(&mut s, id, "draw", Value::Array(points), json!({"dyntopo_size": 0.05, "radius": 0.3}));
        assert_sound(&mut s, id, true, &format!("dyntopo on {kind}"));
    }
}

#[test]
fn dynamic_topology_with_a_mask_and_protect_stays_sound() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 4}));
    stroke(&mut s, id, "mask", json!([[0.6, 0.8, 0.0]]), json!({"radius": 0.5}));
    stroke(&mut s, id, "draw", path_on_sphere(), json!({"dyntopo": 0.15, "dyntopo_protect_mask": true}));
    assert_sound(&mut s, id, true, "masked dyntopo");
}

#[test]
fn voxel_remesh_makes_a_finite_closed_manifold_of_every_primitive() {
    for kind in CLOSED {
        for resolution in [16, 40] {
            let mut s = Session::default();
            let id = add(&mut s, kind, json!({}));
            call(&mut s, "remesh_voxel", json!({"id": id, "resolution": resolution}));
            assert_sound(&mut s, id, true, &format!("voxel remesh {kind} at {resolution}"));
        }
    }
}

#[test]
fn voxel_remesh_options_and_sculpted_input_stay_sound() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 4}));
    stroke(&mut s, id, "grab", json!([[0.0, 1.0, 0.0], [0.0, 2.2, 0.0]]), json!({"radius": 0.7}));
    stroke(&mut s, id, "inflate", path_on_sphere(), json!({}));
    for args in [json!({"resolution": 48}), json!({"resolution": 48, "thicken": 100}), json!({"resolution": 48, "keep_sharp": 30})] {
        let mut t = Session::default();
        let tid = add(&mut t, "cube", json!({"detail": 3}));
        let mut a = args.clone();
        a["id"] = json!(tid);
        call(&mut t, "remesh_voxel", a);
        assert_sound(&mut t, tid, true, &format!("voxel remesh cube {args}"));
    }
    call(&mut s, "remesh_voxel", json!({"id": id, "resolution": 56}));
    assert_sound(&mut s, id, true, "voxel remesh after a grab");
}

#[test]
fn voxel_remesh_of_an_imported_file_stays_sound() {
    let mut s = Session::default();
    let id = add(&mut s, "torus", json!({}));
    let path = temp("fixture-torus.stl");
    call(&mut s, "export_mesh", json!({"path": path, "ids": [id]}));
    let mut t = Session::default();
    let r = call(&mut t, "import_mesh", json!({"path": path}));
    let tid = r["objects"][0]["id"].as_u64().unwrap();
    assert_sound(&mut t, tid, true, "imported torus");
    call(&mut t, "remesh_voxel", json!({"id": tid, "resolution": 40}));
    assert_sound(&mut t, tid, true, "voxel remesh of an imported torus");
}

#[test]
fn smoothing_never_yields_nan_or_changes_the_topology() {
    for kind in CLOSED {
        for brush in ["smooth", "relax", "polish"] {
            let mut s = Session::default();
            let id = add(&mut s, kind, json!({"detail": 3}));
            let tris = object(&mut s, id)["triangles"].clone();
            for _ in 0..6 {
                stroke(&mut s, id, brush, json!([[0.0, 1.0, 0.0], [0.5, 0.8, 0.3], [-0.5, 0.2, 0.9], [0.9, 0.0, 0.0]]), json!({"radius": 0.9}));
            }
            assert_sound(&mut s, id, true, &format!("{brush} on {kind}"));
            assert_eq!(object(&mut s, id)["triangles"], tris, "{brush} on {kind} changed the triangle count");
        }
    }
}

#[test]
fn smoothing_a_fully_masked_and_a_flat_mesh_is_safe() {
    let mut s = Session::default();
    let plane = add(&mut s, "plane", json!({"size": 2, "detail": 24}));
    for _ in 0..4 {
        stroke(&mut s, plane, "smooth", json!([[0.0, 0.0, 0.0], [0.4, 0.0, 0.2]]), json!({"radius": 1.5}));
    }
    assert_sound(&mut s, plane, false, "smooth on a plane");
    let cube = add(&mut s, "cube", json!({"detail": 3}));
    call(&mut s, "mask_invert", json!({"id": cube}));
    stroke(&mut s, cube, "smooth", json!([[0.0, 1.0, 0.0]]), json!({"radius": 2.0}));
    assert_sound(&mut s, cube, true, "smooth on a masked cube");
}

#[test]
fn undo_after_a_stroke_restores_every_vertex_to_the_bit() {
    for brush in ["draw", "inflate", "grab", "smooth", "clay", "pinch", "crease", "flatten", "relax", "snake_hook", "noise"] {
        let mut s = Session::default();
        let id = add(&mut s, "sphere", json!({"detail": 4}));
        stroke(&mut s, id, "inflate", json!([[0.0, 1.0, 0.0]]), json!({"radius": 0.7}));
        let before = mesh_of(&s, id);
        stroke(&mut s, id, brush, json!([[0.0, 1.1, 0.0], [0.3, 1.0, 0.1], [0.5, 0.9, 0.2]]), json!({"radius": 0.6}));
        call(&mut s, "undo", json!({}));
        assert_eq!(mesh_of(&s, id), before, "undo after {brush} did not restore the mesh");
    }
}

#[test]
fn undo_restores_the_mesh_after_dyntopo_and_after_a_voxel_remesh() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    let original = mesh_of(&s, id);
    stroke(&mut s, id, "draw", path_on_sphere(), json!({"dyntopo": 0.15}));
    assert_ne!(mesh_of(&s, id).1.len(), original.1.len());
    call(&mut s, "undo", json!({}));
    assert_eq!(mesh_of(&s, id), original);
    call(&mut s, "remesh_voxel", json!({"id": id, "resolution": 32}));
    assert_ne!(mesh_of(&s, id).1.len(), original.1.len());
    call(&mut s, "undo", json!({}));
    assert_eq!(mesh_of(&s, id), original);
    call(&mut s, "redo", json!({}));
    assert_sound(&mut s, id, true, "redo of a voxel remesh");
}

#[test]
fn every_primitive_exports_to_stl_obj_and_gltf_and_reopens_with_its_triangles() {
    for kind in ["sphere", "uv_sphere", "cube", "cylinder", "cone", "torus", "plane"] {
        let mut s = Session::default();
        let id = add(&mut s, kind, json!({"detail": 3}));
        let tris = object(&mut s, id)["triangles"].clone();
        for ext in ["stl", "obj", "glb", "gltf"] {
            let path = temp(&format!("{kind}.{ext}"));
            call(&mut s, "export_mesh", json!({"path": path, "ids": [id]}));
            let mut t = Session::default();
            let r = call(&mut t, "import_mesh", json!({"path": path}));
            assert_eq!(r["objects"].as_array().unwrap().len(), 1, "{kind}.{ext}");
            assert_eq!(r["objects"][0]["triangles"], tris, "{kind}.{ext} changed the triangle count");
        }
    }
}

#[test]
fn binary_stl_has_its_80_byte_header_and_a_length_that_matches_its_count() {
    for kind in CLOSED {
        let mut s = Session::default();
        let id = add(&mut s, kind, json!({"detail": 3}));
        let tris = object(&mut s, id)["triangles"].as_u64().unwrap();
        let path = temp(&format!("header-{kind}.stl"));
        call(&mut s, "export_mesh", json!({"path": path, "ids": [id]}));
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len() as u64, 84 + 50 * tris, "{kind}");
        assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as u64, tris, "{kind}");
        assert!(bytes[..80].starts_with(b"formsculpt binary"), "{kind}");
        assert!(!bytes[..5].eq_ignore_ascii_case(b"solid"), "{kind}: a binary STL must not look like ASCII");
    }
}

#[test]
fn a_sculpted_and_remeshed_object_still_exports_and_reopens_in_every_format() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    stroke(&mut s, id, "draw", path_on_sphere(), json!({"dyntopo": 0.2}));
    let tris = object(&mut s, id)["triangles"].clone();
    for ext in ["stl", "obj", "glb"] {
        let path = temp(&format!("sculpted.{ext}"));
        call(&mut s, "export_mesh", json!({"path": path, "ids": [id]}));
        let mut t = Session::default();
        let r = call(&mut t, "import_mesh", json!({"path": path}));
        assert_eq!(r["objects"][0]["triangles"], tris, "{ext}");
    }
}
