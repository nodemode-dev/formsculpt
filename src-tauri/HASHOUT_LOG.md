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

## formsculpt 1.24.0 · the engine is held, and a failed start says why

### src/main.rs, src/engine_start.rs, src/win_job.rs · job object, 90 second wait, the failure dialog before the splash closes
- refers-to: src/main.rs :: kill_backend, limit, read_port, engine_failed, open_main, setup; src/engine_start.rs; src/win_job.rs; Cargo.toml; Cargo.lock
- kind: rationale
- logged: v1.24.0 · 2026-11-20
The engine is still started through tauri-plugin-shell's `sidecar`, as before (FormVis's and FormFabric's way, not FormTools's standard-library start), so on Windows `win_job::hold_pid(child.pid())` puts it in a job object right after the spawn; the job ends the engine, and everything it started, when the app ends however it ends, and `kill_backend` also ends the job. `win_job.rs` is FormVis's file at origin/main, unchanged (its `hold` for a standard-library child is not used here). `windows-sys` 0.61 is a Windows-only dependency (already in `Cargo.lock` through Tauri at 0.61.2; the lock gains only the name in formsculpt's own list). `engine_start.rs` holds the plain parts, FormFabric's and FormTools's file with formsculpt's names: `ADDRESS_WAIT` 90 seconds with no way to extend it (the owner's limit for every app; the old "Keep Waiting" dialog after 60 seconds is gone), the sentence for each failure, the last 12 lines the engine printed (never the window key line), and the log's path, `<data>/Logs/formsculpt-engine.log` with the data folder found as the engine finds it (`FSC_DATA_DIR`, `~` read as home, else `~/Formsculpt`). `read_port` feeds every line to the tail, and after the engine ends it reads for `EXIT_WAIT` (1 second) more, so lines that arrive after the end are still shown. `engine_failed` shows the dialog first and closes the splash, the only window at that point, after the person has closed the dialog: closing the last window first could end the app before the dialog appeared. A main window that cannot be built now gives its own sentence instead of closing the splash without a word. The shell gives the engine its process id in `engine_start::SHELL_ENV`, `FSC_PARENT_PID`, the name the engine watches. Removed code, quoted in full: `const PORT_WAIT: std::time::Duration = std::time::Duration::from_secs(60);`; `enum EngineStart { Port(u16), Exited(Option<i32>), Silent }` with its doc line "How the wait for the engine's address ended."; `quit_launch`, which called `close_splash(handle)` first and then showed "The formsculpt engine failed to start: {reason}\n\nPlease relaunch the app. If it keeps happening, install formsculpt again." titled "formsculpt", then `kill_backend` and `exit(1)`, with its doc line "Ends a launch whose engine never came up. Runs off the main thread: the dialog blocks."; `keep_waiting`, a warning "The formsculpt engine has not answered for a minute. A first start on a slow or busy computer can take longer." with the buttons "Keep Waiting" and "Quit", doc line "The engine has been silent for PORT_WAIT: true when the person chooses to go on waiting."; `read_port`'s doc line "Reads the engine's output until it names its address, it ends, or PORT_WAIT passes without an address."; the reasons "it could not be run ({err}). It may be missing from the installation or blocked by security software.", "it ended before it was ready (exit code {code}).", "it ended before it was ready."; and in `open_main` the `else { close_splash(handle); }` branch.
