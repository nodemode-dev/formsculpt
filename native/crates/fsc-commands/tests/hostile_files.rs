//! Crafted, truncated and oversized files are refused with an error, never a panic or a runaway allocation: PLY, glTF
//! and GLB, USD text, FBX, the .fsc project and PNG. Every file is built here and written to a temporary folder, and
//! after each refusal the same session still opens a correct file.

mod common;

use base64::Engine as _;
use common::*;
use fsc_commands::{dispatch, Session};
use serde_json::{json, Value};

fn write(name: &str, bytes: &[u8]) -> String {
    let path = temp(name);
    std::fs::write(&path, bytes).expect("write the crafted file");
    path
}

fn count(s: &mut Session) -> usize {
    call(s, "get_scene", json!({}))["objects"].as_array().expect("objects").len()
}

fn all_finite(s: &Session) -> bool {
    s.scene.objects.iter().all(|o| o.mesh.positions.iter().all(|p| p.iter().all(|c| c.is_finite())))
}

fn good_obj(after: &str) -> String {
    write(&format!("good-after-{after}.obj"), b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")
}

/// Imports the file into a session that holds one cube. Ok(n) is the number of objects the import added.
fn import(name: &str, bytes: &[u8]) -> (Session, Result<usize, String>) {
    let mut s = Session::default();
    add(&mut s, "cube", json!({}));
    let path = write(name, bytes);
    let r = match dispatch(&mut s, "import_mesh", &json!({"path": path})) {
        Ok(v) => Ok(v["objects"].as_array().map(|a| a.len()).unwrap_or(0)),
        Err(e) => Err(format!("{} {}", e.code, e.message)),
    };
    (s, r)
}

fn after_a_refusal_a_good_file_still_opens(s: &mut Session, what: &str) {
    let before = count(s);
    call(s, "import_mesh", json!({"path": good_obj(what)}));
    assert_eq!(count(s), before + 1, "{what}: a correct file did not open after the refusal");
}

fn refused(name: &str, bytes: &[u8]) {
    let (mut s, r) = import(name, bytes);
    let message = r.expect_err(&format!("{name} should have been refused"));
    assert!(!message.trim().is_empty(), "{name}: the refusal has no message");
    assert_eq!(count(&mut s), 1, "{name}: a refused import changed the scene");
    after_a_refusal_a_good_file_still_opens(&mut s, name);
}

fn refused_or_finite(name: &str, bytes: &[u8]) {
    let (mut s, r) = import(name, bytes);
    if let Ok(n) = r {
        assert_eq!(count(&mut s), 1 + n, "{name}");
        assert!(all_finite(&s), "{name}: an accepted import has a vertex that is not finite");
    }
    after_a_refusal_a_good_file_still_opens(&mut s, name);
}

fn glb(doc: &str, bin: &[u8], bin_len: Option<u32>) -> Vec<u8> {
    let mut j = doc.as_bytes().to_vec();
    while !j.len().is_multiple_of(4) {
        j.push(b' ');
    }
    let mut out = b"glTF".to_vec();
    out.extend(2u32.to_le_bytes());
    out.extend(((28 + j.len() + bin.len()) as u32).to_le_bytes());
    out.extend((j.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(&j);
    out.extend(bin_len.unwrap_or(bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(bin);
    out
}

fn triangle() -> Vec<u8> {
    [0f32, 0., 0., 1., 0., 0., 0., 1., 0.].iter().flat_map(|f| f.to_le_bytes()).collect()
}

const HEAD: &str = r#"{"asset":{"version":"2.0"},"scenes":[{"nodes":[0]}],"#;
const MESH: &str = r#""meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"#;
const ACCESSOR: &str = r#""accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}],"#;
const VIEW: &str = r#""bufferViews":[{"buffer":0,"byteLength":36}],"buffers":[{"byteLength":36}]}"#;

fn exported(kind: &str, ext: &str) -> Vec<u8> {
    let mut s = Session::default();
    let id = add(&mut s, kind, json!({"detail": 2}));
    let path = temp(&format!("exported-{kind}.{ext}"));
    call(&mut s, "export_mesh", json!({"path": path, "ids": [id]}));
    std::fs::read(&path).expect("exported file")
}

#[test]
fn an_empty_file_of_every_format_is_refused() {
    for ext in ["obj", "stl", "ply", "glb", "gltf", "fbx", "usda", "3mf"] {
        refused(&format!("empty.{ext}"), b"");
    }
}

#[test]
fn a_ply_header_that_declares_more_vertices_than_the_file_holds_is_refused() {
    refused("many-vertices.ply", b"ply\nformat ascii 1.0\nelement vertex 99999999999999\nend_header\n");
    refused("many-vertices-bin.ply", b"ply\nformat binary_little_endian 1.0\nelement vertex 4000000000\nproperty float x\nproperty float y\nproperty float z\nend_header\n\0\0\0\0");
}

#[test]
fn a_ply_header_that_declares_more_faces_than_the_file_holds_is_refused() {
    refused(
        "many-faces.ply",
        b"ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1000000000\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
    );
}

#[test]
fn a_ply_face_list_with_an_absurd_count_is_refused() {
    let head = b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nelement face 1\n";
    let mut huge_count = head.to_vec();
    huge_count.extend(b"property list uint uint vertex_indices\nend_header\n");
    huge_count.extend(0f32.to_le_bytes());
    huge_count.extend(0xFFFF_FFFFu32.to_le_bytes());
    refused("list-count.ply", &huge_count);
    let mut double_count = head.to_vec();
    double_count.extend(b"property list double uint vertex_indices\nend_header\n");
    double_count.extend(0f32.to_le_bytes());
    double_count.extend(1e300f64.to_le_bytes());
    refused("list-double.ply", &double_count);
}

#[test]
fn a_ply_face_that_names_a_vertex_past_the_last_is_refused() {
    refused(
        "face-out-of-range.ply",
        b"ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 7\n",
    );
}

#[test]
fn a_truncated_ply_of_a_real_mesh_is_refused() {
    let full = exported("sphere", "ply");
    refused("cut.ply", &full[..full.len() / 2]);
}

#[test]
fn a_gltf_accessor_with_more_elements_than_its_buffer_is_refused() {
    refused("count.glb", &glb(&format!(r#"{HEAD}"nodes":[{{"mesh":0}}],{MESH}"accessors":[{{"bufferView":0,"componentType":5126,"count":1000000000000000,"type":"VEC3"}}],{VIEW}"#), &triangle(), None));
}

#[test]
fn a_gltf_accessor_with_no_buffer_view_such_as_draco_or_sparse_is_refused() {
    refused("no-view.glb", &glb(&format!(r#"{HEAD}"nodes":[{{"mesh":0}}],{MESH}"accessors":[{{"componentType":5126,"count":1000000000000,"type":"VEC3"}}],"buffers":[{{"byteLength":36}}]}}"#), &triangle(), None));
    refused(
        "sparse.glb",
        &glb(&format!(r#"{HEAD}"nodes":[{{"mesh":0}}],{MESH}"accessors":[{{"componentType":5126,"count":3,"type":"VEC3","sparse":{{"count":1,"indices":{{"bufferView":0,"componentType":5125}},"values":{{"bufferView":0}}}}}}],{VIEW}"#), &triangle(), None),
    );
}

#[test]
fn a_gltf_node_that_is_its_own_ancestor_imports_its_mesh_once() {
    let (mut s, r) = import("cycle.glb", &glb(&format!(r#"{HEAD}"nodes":[{{"mesh":0,"children":[0]}}],{MESH}{ACCESSOR}{VIEW}"#), &triangle(), None));
    match r {
        Ok(n) => assert_eq!(n, 1, "a cycle made more than one object"),
        Err(e) => assert!(!e.is_empty()),
    }
    assert!(all_finite(&s));
    after_a_refusal_a_good_file_still_opens(&mut s, "cycle.glb");
}

#[test]
fn a_gltf_node_or_scene_index_out_of_range_is_not_fatal() {
    refused_or_finite("child-out-of-range.glb", &glb(&format!(r#"{HEAD}"nodes":[{{"mesh":0,"children":[9]}}],{MESH}{ACCESSOR}{VIEW}"#), &triangle(), None));
    refused_or_finite("scene-node-out-of-range.glb", &glb(&format!(r#"{{"asset":{{"version":"2.0"}},"scenes":[{{"nodes":[7]}}],"nodes":[{{"mesh":0}}],{MESH}{ACCESSOR}{VIEW}"#), &triangle(), None));
    refused_or_finite("mesh-out-of-range.glb", &glb(&format!(r#"{HEAD}"nodes":[{{"mesh":4}}],{MESH}{ACCESSOR}{VIEW}"#), &triangle(), None));
    refused_or_finite("short-scale.glb", &glb(&format!(r#"{HEAD}"nodes":[{{"mesh":0,"scale":[2]}}],{MESH}{ACCESSOR}{VIEW}"#), &triangle(), None));
}

#[test]
fn a_gltf_with_no_scenes_imports_its_root_nodes_with_their_transforms() {
    let doc = format!(r#"{{"asset":{{"version":"2.0"}},"nodes":[{{"children":[1],"translation":[5,0,0]}},{{"mesh":0}}],{MESH}{ACCESSOR}{VIEW}"#);
    let mut s = Session::default();
    let r = call(&mut s, "import_mesh", json!({"path": write("no-scene.glb", &glb(&doc, &triangle(), None))}));
    let objects = r["objects"].as_array().expect("objects");
    assert_eq!(objects.len(), 1);
    let id = objects[0]["id"].as_u64().unwrap();
    let min_x = object(&mut s, id)["bounds"]["min"][0].as_f64().unwrap();
    assert!((min_x - 5.0).abs() < 1e-3, "the parent's translation was not applied: min x {min_x}");
}

#[test]
fn a_glb_whose_bin_chunk_claims_four_gigabytes_is_not_fatal() {
    refused_or_finite("bin-length.glb", &glb(&format!(r#"{HEAD}"nodes":[{{"mesh":0}}],{MESH}{ACCESSOR}{VIEW}"#), &triangle(), Some(0xFFFF_FFF8)));
}

#[test]
fn a_truncated_glb_of_a_real_mesh_is_refused() {
    let full = exported("torus", "glb");
    refused("cut.glb", &full[..full.len() * 2 / 3]);
}

#[test]
fn a_usd_mesh_with_no_points_is_refused() {
    refused("no-points.usda", b"#usda 1.0\ndef Mesh \"a\"\n{\n point3f[] points = []\n int[] faceVertexCounts = []\n int[] faceVertexIndices = []\n}\n");
}

#[test]
fn a_usd_mesh_whose_indices_go_past_its_points_is_refused() {
    refused("index-past.usda", b"#usda 1.0\ndef Mesh \"a\"\n{\n point3f[] points = [(0,0,0),(1,0,0),(0,1,0)]\n int[] faceVertexCounts = [3]\n int[] faceVertexIndices = [0,1,9]\n}\n");
    refused("counts-past.usda", b"#usda 1.0\ndef Mesh \"a\"\n{\n point3f[] points = [(0,0,0),(1,0,0),(0,1,0)]\n int[] faceVertexCounts = [3, 1e300]\n int[] faceVertexIndices = [0,1,2,0,1,2]\n}\n");
    refused("corners-past.usda", b"#usda 1.0\ndef Mesh \"a\"\n{\n point3f[] points = [(0,0,0),(1,0,0),(0,1,0)]\n int[] faceVertexCounts = [3, 3]\n int[] faceVertexIndices = [0,1,2]\n}\n");
}

/// A zlib stream of `pairs` fixed-Huffman (length 258, distance 1) copies after one literal zero: it inflates to
/// 1 + 258 * pairs zero bytes from about 13 bits per pair.
fn zlib_zeros(pairs: usize) -> Vec<u8> {
    struct Bits {
        out: Vec<u8>,
        acc: u32,
        n: u32,
    }
    impl Bits {
        fn put(&mut self, value: u32, len: u32) {
            for i in 0..len {
                self.acc |= ((value >> i) & 1) << self.n;
                self.n += 1;
                if self.n == 8 {
                    self.out.push(self.acc as u8);
                    self.acc = 0;
                    self.n = 0;
                }
            }
        }
        fn code(&mut self, code: u32, len: u32) {
            for i in (0..len).rev() {
                self.put((code >> i) & 1, 1);
            }
        }
    }
    let mut b = Bits { out: vec![0x78, 0x01], acc: 0, n: 0 };
    b.put(1, 1);
    b.put(1, 2);
    b.code(0b0011_0000, 8);
    for _ in 0..pairs {
        b.code(0b1100_0101, 8);
        b.code(0, 5);
    }
    b.code(0, 7);
    if b.n > 0 {
        b.out.push(b.acc as u8);
    }
    let total = 1u64 + 258 * pairs as u64;
    let a = 1u32;
    let s = (total % 65521) as u32;
    b.out.extend(((s << 16) | a).to_be_bytes());
    b.out
}

fn fbx_with_array(declared: u32, data: &[u8]) -> Vec<u8> {
    let mut out = b"Kaydara FBX Binary  \0\x1a\0".to_vec();
    out.extend(7400u32.to_le_bytes());
    let name = b"Objects";
    let props_len = 1 + 12 + data.len();
    let end = out.len() + 13 + name.len() + props_len;
    out.extend((end as u32).to_le_bytes());
    out.extend(1u32.to_le_bytes());
    out.extend((props_len as u32).to_le_bytes());
    out.push(name.len() as u8);
    out.extend(name);
    out.push(b'i');
    out.extend(declared.to_le_bytes());
    out.extend(1u32.to_le_bytes());
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out.extend([0u8; 13]);
    out
}

/// The process's peak resident memory in kB, where the system reports it (Linux).
fn peak_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

#[test]
fn an_fbx_array_that_inflates_far_past_its_declared_length_is_refused_without_inflating_it() {
    let bomb = fbx_with_array(1, &zlib_zeros(2_400_000));
    let before = peak_kb();
    refused("bomb.fbx", &bomb);
    if let (Some(a), Some(b)) = (before, peak_kb()) {
        assert!(b.saturating_sub(a) < 256 * 1024, "reading a 4 byte array grew the process by {} MB", (b - a) / 1024);
    }
}

#[test]
fn a_truncated_fbx_of_a_real_mesh_is_refused() {
    let full = exported("torus", "fbx");
    refused("cut.fbx", &full[..full.len() / 2]);
}

fn open(bytes: &[u8], name: &str) -> (Session, Result<Value, String>) {
    let mut s = Session::default();
    add(&mut s, "cube", json!({}));
    let path = write(name, bytes);
    let r = dispatch(&mut s, "open_project", &json!({"path": path})).map_err(|e| format!("{} {}", e.code, e.message));
    (s, r)
}

fn project(manifest: Value, files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut all: Vec<(String, &[u8])> = files.iter().map(|(n, d)| (n.to_string(), d.as_slice())).collect();
    let text = manifest.to_string();
    all.push(("manifest.json".into(), text.as_bytes()));
    fsc_io::zip_files(&all).expect("zip")
}

fn f32s(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn u32s(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

#[test]
fn a_project_entry_that_declares_a_size_near_four_gigabytes_is_not_fatal() {
    let mut p = project(json!({"format": "formsculpt-project", "version": 1, "objects": []}), &[]);
    let cd = p.windows(4).rposition(|w| w == b"PK\x01\x02").expect("central directory");
    p[cd + 24..cd + 28].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
    let (mut s, r) = open(&p, "huge-entry.fsc");
    if r.is_ok() {
        assert_eq!(count(&mut s), 0);
    }
    after_a_refusal_a_good_file_still_opens(&mut s, "huge-entry.fsc");
}

#[test]
fn a_project_whose_level_data_does_not_fit_its_mesh_is_refused_at_open() {
    let manifest = json!({"format": "formsculpt-project", "version": 1, "objects": [{"id": 1, "name": "a", "positions": "p", "indices": "i", "mask": "m",
        "multires": {"current": 0, "levels": [{"vertices": 3, "triangles": 1}, {"vertices": 3, "triangles": 1, "positions": "p", "indices": "i", "stencil": {"offsets": "so", "sources": "ss", "weights": "sw"}}]}}]});
    let files = [
        ("p", f32s(&[0., 0., 0., 1., 0., 0., 0., 1., 0.])),
        ("i", u32s(&[0, 1, 2])),
        ("m", f32s(&[0., 0., 0.])),
        ("so", u32s(&[0, 1, 2, 3])),
        ("ss", u32s(&[0, 1, 99])),
        ("sw", f32s(&[1., 1., 1.])),
    ];
    let (mut s, r) = open(&project(manifest, &files), "bad-levels.fsc");
    assert!(r.is_err(), "a level stack that names vertex 99 of 3 was opened");
    assert_eq!(count(&mut s), 1, "a refused project replaced the scene");
}

#[test]
fn a_project_whose_buffers_are_shorter_than_its_counts_is_refused() {
    let manifest = json!({"format": "formsculpt-project", "version": 1, "objects": [{"id": 1, "name": "a", "positions": "p", "indices": "i"}]});
    let files = [("p", f32s(&[0., 0., 0., 1., 0., 0.])), ("i", u32s(&[0, 1, 2]))];
    let (mut s, r) = open(&project(manifest, &files), "short-buffers.fsc");
    assert!(r.is_err(), "a project whose triangle names a third vertex it does not hold was opened");
    assert_eq!(count(&mut s), 1);
}

#[test]
fn a_truncated_project_is_refused_and_leaves_the_scene() {
    let mut s = Session::default();
    let id = add(&mut s, "sphere", json!({"detail": 3}));
    call(&mut s, "multires_divide", json!({"id": id}));
    let path = temp("whole.fsc");
    call(&mut s, "save_project", json!({"path": path}));
    let full = std::fs::read(&path).unwrap();
    let (mut t, r) = open(&full[..full.len() / 2], "cut.fsc");
    assert!(r.is_err(), "half a project was opened");
    assert_eq!(count(&mut t), 1);
    let (mut u, r) = open(&full, "whole-again.fsc");
    r.expect("the whole project opens");
    assert_eq!(count(&mut u), 1);
}

/// A 69 byte PNG whose header says 50000 by 50000 pixels.
fn huge_png() -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc(&body).to_be_bytes());
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = 50_000u32.to_be_bytes().to_vec();
    ihdr.extend(50_000u32.to_be_bytes());
    ihdr.extend([8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &[0x78, 0x9c, 0x63, 0x60, 0xa0, 0x0c, 0x00, 0x00, 0x00, 0x40, 0x00, 0x01]);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[test]
fn a_png_that_declares_50000_by_50000_pixels_is_refused_by_every_command_that_reads_pictures() {
    let png = huge_png();
    assert_eq!(png.len(), 69);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    let path = write("huge.png", &png);
    let mut s = Session::default();
    add(&mut s, "sphere", json!({"detail": 2}));
    for (name, args) in [
        ("spotlight_add", json!({"png_base64": b64})),
        ("alpha_load", json!({"name": "huge", "png_base64": b64})),
        ("alpha_load", json!({"name": "huge", "path": path})),
        ("paint_project", json!({"png_base64": b64})),
    ] {
        let code = fails(&mut s, name, args.clone());
        assert!(!code.is_empty(), "{name} {args}");
    }
    assert!(all_finite(&s));
    call(&mut s, "sculpt_stroke", json!({"brush": "draw", "points": [[0, 0, 1]], "radius": 0.3, "strength": 1}));
}
