//! The engine as a WebAssembly module for the window, with no engine service behind it.
//!
//! A plain C ABI so the page needs no generated bindings: the page copies its input into memory from `fsc_alloc`,
//! calls a function, reads the result from `fsc_out_ptr` and `fsc_out_len` (one output buffer, replaced by each call),
//! and frees its input with `fsc_free`. Text in and out is UTF-8 JSON; meshes, strokes and files are the same bytes the
//! service sends (FSCM, FSCU, projects and exports), built by the same code in fsc_commands::live. The module imports
//! `env.fsc_now_ms` (a clock) and `env.fsc_log` (panic messages) from the page. One engine session per module.

use std::cell::RefCell;

use fsc_commands::Session;
use serde_json::{json, Value};

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::default());
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn fsc_log(ptr: *const u8, len: usize);
}

/// Sends a panic's message to the page before the module traps, so the window can say what went wrong.
#[no_mangle]
pub extern "C" fn fsc_init() {
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(|info| {
        let text = info.to_string();
        // SAFETY: the page reads `len` bytes at `ptr` from this module's memory during the call.
        unsafe { fsc_log(text.as_ptr(), text.len()) };
    }));
}

fn put(bytes: Vec<u8>) {
    OUT.with(|o| *o.borrow_mut() = bytes);
}

fn put_json(v: Value) {
    put(v.to_string().into_bytes());
}

fn reply(result: Result<Value, fsc_commands::CommandError>) -> Value {
    match result {
        Ok(result) => json!({"ok": true, "result": result}),
        Err(err) => json!({"ok": false, "error": err}),
    }
}

/// Bytes the page placed at `ptr`.
///
/// # Safety
/// `ptr` must come from `fsc_alloc` with at least `len` bytes written, still allocated.
unsafe fn input<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(ptr, len)
    }
}

/// # Safety
/// As [`input`]; the bytes are expected to be UTF-8 (anything else reads as empty).
unsafe fn text<'a>(ptr: *const u8, len: usize) -> &'a str {
    std::str::from_utf8(input(ptr, len)).unwrap_or("")
}

#[no_mangle]
pub extern "C" fn fsc_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// # Safety
/// `ptr` and `len` must be a pair `fsc_alloc` returned and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn fsc_free(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

#[no_mangle]
pub extern "C" fn fsc_out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn fsc_out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

#[no_mangle]
pub extern "C" fn fsc_version() {
    put(fsc_core::engine_version().as_bytes().to_vec());
}

#[no_mangle]
pub extern "C" fn fsc_catalogue() {
    put_json(fsc_commands::catalogue());
}

/// Runs a registered command; the output is the reply JSON (`ok` with `result`, or `error`).
///
/// # Safety
/// Both pairs must be page input from `fsc_alloc`.
#[no_mangle]
pub unsafe extern "C" fn fsc_dispatch(name: *const u8, name_len: usize, args: *const u8, args_len: usize) {
    let name = text(name, name_len);
    let raw = text(args, args_len);
    let parsed: Result<Value, _> = if raw.trim().is_empty() { Ok(json!({})) } else { serde_json::from_str(raw) };
    let out = match parsed {
        Ok(args) => SESSION.with(|s| reply(fsc_commands::dispatch(&mut s.borrow_mut(), name, &args))),
        Err(e) => json!({"ok": false, "error": {"code": "bad_arguments", "message": format!("arguments are not JSON: {e}")}}),
    };
    put_json(out);
}

/// The FSCM payload of an object (0), or -1 with the error JSON as output.
#[no_mangle]
pub extern "C" fn fsc_mesh(id: u32, smooth: u32) -> i32 {
    match SESSION.with(|s| fsc_commands::live::mesh_payload(&s.borrow(), id, smooth)) {
        Ok(bytes) => {
            put(bytes);
            0
        }
        Err(e) => {
            put_json(json!(e));
            -1
        }
    }
}

/// # Safety
/// `settings` must be page input from `fsc_alloc`.
#[no_mangle]
pub unsafe extern "C" fn fsc_stroke_begin(id: u32, settings: *const u8, settings_len: usize) -> i32 {
    let parsed: Value = match serde_json::from_str(text(settings, settings_len)) {
        Ok(v) => v,
        Err(e) => {
            put_json(json!({"code": "bad_arguments", "message": format!("settings are not JSON: {e}")}));
            return -1;
        }
    };
    match SESSION.with(|s| fsc_commands::live::stroke_begin(&mut s.borrow_mut(), id, &parsed)) {
        Ok(()) => 0,
        Err(e) => {
            put_json(json!(e));
            -1
        }
    }
}

/// One stroke sample: -1 with the error JSON as output, else bit 0 set when it hit and bit 1 when the topology
/// changed; the output is then the FSCU update (empty after a topology change).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn fsc_stroke_sample(ox: f32, oy: f32, oz: f32, dx: f32, dy: f32, dz: f32, pressure: f32, tilt: f32) -> i32 {
    match SESSION.with(|s| fsc_commands::live::stroke_sample(&mut s.borrow_mut(), [ox, oy, oz], [dx, dy, dz], pressure, tilt)) {
        Ok((hit, update, topology)) => {
            put(update);
            i32::from(hit) | (i32::from(topology) << 1)
        }
        Err(e) => {
            put_json(json!(e));
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn fsc_stroke_end() {
    put_json(SESSION.with(|s| fsc_commands::live::stroke_end(&mut s.borrow_mut())));
}

/// The scene as a .fsc project (0), or -1 with the error JSON.
#[no_mangle]
pub extern "C" fn fsc_save() -> i32 {
    match SESSION.with(|s| fsc_commands::save_data(&s.borrow())) {
        Ok(bytes) => {
            put(bytes);
            0
        }
        Err(e) => {
            put_json(json!(e));
            -1
        }
    }
}

/// Opens a .fsc project, replacing the scene; the output is the reply JSON.
///
/// # Safety
/// `data` must be page input from `fsc_alloc`.
#[no_mangle]
pub unsafe extern "C" fn fsc_open(data: *const u8, len: usize) {
    let bytes = input(data, len).to_vec();
    put_json(SESSION.with(|s| reply(fsc_commands::open_data(&mut s.borrow_mut(), bytes))));
}

/// Imports a mesh file by its name's extension; the output is the reply JSON.
///
/// # Safety
/// Both pairs must be page input from `fsc_alloc`.
#[no_mangle]
pub unsafe extern "C" fn fsc_import(name: *const u8, name_len: usize, data: *const u8, len: usize) {
    let name = text(name, name_len).to_string();
    let bytes = input(data, len);
    put_json(SESSION.with(|s| reply(fsc_commands::import_data(&mut s.borrow_mut(), &name, bytes, None))));
}

/// Exports objects (a JSON list of ids, or empty for every visible object) to the format of `name`: 0 with the file
/// as output, or -1 with the error JSON.
///
/// # Safety
/// Both pairs must be page input from `fsc_alloc`.
#[no_mangle]
pub unsafe extern "C" fn fsc_export(name: *const u8, name_len: usize, ids: *const u8, ids_len: usize) -> i32 {
    let name = text(name, name_len).to_string();
    let chosen: Option<Vec<u32>> = serde_json::from_str(text(ids, ids_len)).ok();
    let result = SESSION.with(|s| {
        let s = s.borrow();
        let ids = chosen.unwrap_or_else(|| s.scene.objects.iter().filter(|o| o.visible).map(|o| o.id).collect());
        if let Some(missing) = ids.iter().find(|&&id| s.scene.get(id).is_none()) {
            return Err(fsc_commands::CommandError::new("not_found", format!("no object {missing}")));
        }
        fsc_commands::export_data(&s, &name, &ids)
    });
    match result {
        Ok(bytes) => {
            put(bytes);
            0
        }
        Err(e) => {
            put_json(json!(e));
            -1
        }
    }
}

/// The live boolean preview of objects `a` and `b` (op 0 union, 1 difference, 2 intersection) as FSCM (0), or -1
/// with the error JSON.
#[no_mangle]
pub extern "C" fn fsc_boolean_preview(a: u32, b: u32, op: u32) -> i32 {
    let name = ["union", "difference", "intersection"].get(op as usize).copied().unwrap_or("?");
    match SESSION.with(|s| fsc_commands::live::boolean_payload(&s.borrow(), a, b, name)) {
        Ok(bytes) => {
            put(bytes);
            0
        }
        Err(e) => {
            put_json(json!(e));
            -1
        }
    }
}

/// An object's baked maps as one zip (0), or -1 with the error JSON.
#[no_mangle]
pub extern "C" fn fsc_maps(id: u32) -> i32 {
    match SESSION.with(|s| fsc_commands::live::maps_zip(&s.borrow(), id)) {
        Ok(bytes) => {
            put(bytes);
            0
        }
        Err(e) => {
            put_json(json!(e));
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out() -> Vec<u8> {
        OUT.with(|o| o.borrow().clone())
    }

    fn call(name: &str, args: Value) -> Value {
        let (n, a) = (name.as_bytes(), args.to_string().into_bytes());
        // SAFETY: plain slices stand in for page input here.
        unsafe { fsc_dispatch(n.as_ptr(), n.len(), a.as_ptr(), a.len()) };
        serde_json::from_slice(&out()).unwrap()
    }

    #[test]
    fn the_c_surface_sculpts_meshes_and_round_trips_files() {
        let added = call("add_primitive", json!({"kind": "sphere", "detail": 3}));
        let id = added["result"]["object"]["id"].as_u64().unwrap() as u32;
        assert_eq!(fsc_mesh(id, 0), 0);
        let mesh = out();
        assert_eq!(&mesh[..4], b"FSCM");
        assert_eq!(fsc_mesh(999, 0), -1);
        let settings = json!({"brush": "draw", "radius": 0.3, "strength": 1}).to_string().into_bytes();
        // SAFETY: as above.
        assert_eq!(unsafe { fsc_stroke_begin(id, settings.as_ptr(), settings.len()) }, 0);
        let flags = fsc_stroke_sample(0.0, 0.0, 5.0, 0.0, 0.0, -1.0, 1.0, 0.0);
        assert_eq!(flags & 1, 1, "the ray down the z axis hits the sphere");
        assert_eq!(&out()[..4], b"FSCU");
        fsc_stroke_end();
        let ended: Value = serde_json::from_slice(&out()).unwrap();
        assert!(ended["result"]["vertices_changed"].as_u64().unwrap() > 0);
        assert_eq!(fsc_save(), 0);
        let project = out();
        // SAFETY: as above.
        unsafe { fsc_open(project.as_ptr(), project.len()) };
        let opened: Value = serde_json::from_slice(&out()).unwrap();
        assert_eq!(opened["ok"], true);
        let name = b"x.glb";
        // SAFETY: as above.
        assert_eq!(unsafe { fsc_export(name.as_ptr(), name.len(), std::ptr::null(), 0) }, 0);
        let glb = out();
        assert_eq!(&glb[..4], b"glTF");
        // SAFETY: as above.
        unsafe { fsc_import(name.as_ptr(), name.len(), glb.as_ptr(), glb.len()) };
        assert_eq!(serde_json::from_slice::<Value>(&out()).unwrap()["ok"], true);
        assert_eq!(call("get_scene", json!({}))["result"]["objects"].as_array().unwrap().len(), 2);
    }
}
