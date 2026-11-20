# src-tauri HASHOUT_LOG

Living log for the desktop shell and its engine start, _v0.1.0. Entries are anchored by file and symbol, and removed code is quoted in full.

## Org-navi-map

```text
src-tauri/
  src/main.rs            the shell: splash, engine start, main window, exit gate
  src/reveal.rs          splash and window reveal
  src/text_edit.rs       native text edit menu
  src/traffic_lights.rs  macOS title bar buttons
  Cargo.toml             crates; Cargo.lock beside it
  tauri.conf.json        windows, bundle, sidecar
```

## formsculpt 1.23.0 · the window key

### src/main.rs · the key line is read and handed to the main window only
- refers-to: src/main.rs :: read_port, open_main, setup
- kind: rationale
- logged: v1.23.0 · 2026-11-20
The engine is started with `FSC_WINDOW_KEY_STDOUT=1` beside `FSC_PORT` and `FSC_PARENT_PID`, so it prints `formsculpt-window-key: <key>` before its address. `read_port` takes `key: &mut Option<String>` and fills it from the first line that matches `formsculpt-window-key: ([A-Za-z0-9_-]{32,})`; the key lives in the setup task across a "Keep Waiting" round. `open_main` takes `key: Option<&str>` and, when there is one, adds `initialization_script` that sets `window.__FORMSCULPT_WINDOW_KEY` only when `window.location.origin` is the engine's own `http://127.0.0.1:<port>` (Rust's `{:?}` quoting makes both values JavaScript string literals). Nothing writes the engine's standard output to a file. Removed code, quoted in full: `async fn read_port(rx: &mut tauri::async_runtime::Receiver<CommandEvent>) -> EngineStart {`, `fn open_main(handle: &tauri::AppHandle, port: u16) {`, `let url: tauri::Url = format!("http://127.0.0.1:{port}").parse().expect("loopback url");`, `match read_port(&mut rx).await {`, `open_main(&handle, port);`. Not compiled here; the shape follows FormTools `src/main.rs` (`read_port`, `open_main`), and `initialization_script(impl Into<String>)` is the signature of tauri 2.11.6, the version in `Cargo.lock`.
