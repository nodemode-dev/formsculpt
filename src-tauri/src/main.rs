#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_updater::UpdaterExt;

mod reveal;
mod text_edit;
mod traffic_lights;

struct Backend(Mutex<Option<CommandChild>>);

/// Files the system asked formsculpt to open (a double-clicked project, a second launch). They wait here until the page
/// has registered its listener and called take_pending_opens; after that they are sent to the page as they arrive.
#[derive(Default)]
struct Opens {
    ready: bool,
    paths: Vec<String>,
}

struct OpenQueue(Mutex<Opens>);

/// Set once the page has settled unsaved work (or the app is restarting for an update): from then on a close or quit
/// request goes through instead of being handed to the page.
struct ExitGate(AtomicBool);

const FALLBACK_LOGICAL: (f64, f64) = (1323.0, 860.0);
const MIN_LOGICAL: (f64, f64) = (980.0, 640.0);
const SCREEN_FRACTION: f64 = 0.9;
const PORT_WAIT: std::time::Duration = std::time::Duration::from_secs(60);

/// How the wait for the engine's address ended.
enum EngineStart {
    Port(u16),
    Exited(Option<i32>),
    Silent,
}

fn kill_backend(handle: &tauri::AppHandle) {
    if let Some(child) = handle.state::<Backend>().0.lock().unwrap().take() {
        #[cfg(unix)]
        {
            let pid = child.pid().to_string();
            let signal = |sig: &str| {
                std::process::Command::new("kill")
                    .args([sig, pid.as_str()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .is_ok_and(|s| s.success())
            };
            if signal("-TERM") {
                for _ in 0..30 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if !signal("-0") {
                        return;
                    }
                }
            }
        }
        let _ = child.kill();
    }
}

fn close_splash(handle: &tauri::AppHandle) {
    if let Some(splash) = handle.get_webview_window("splash") {
        let _ = splash.close();
    }
}

fn open_exit_gate(handle: &tauri::AppHandle) {
    handle.state::<ExitGate>().0.store(true, Ordering::SeqCst);
}

/// Reads the engine's output until it names its address, it ends, or PORT_WAIT passes without an address.
async fn read_port(rx: &mut tauri::async_runtime::Receiver<CommandEvent>, key: &mut Option<String>) -> EngineStart {
    let pattern = regex::Regex::new(r"http://127\.0\.0\.1:(\d+)").unwrap();
    let key_pattern = regex::Regex::new(r"formsculpt-window-key: ([A-Za-z0-9_-]{32,})").unwrap();
    let (tick, mut timer) = tauri::async_runtime::channel::<()>(1);
    std::thread::spawn(move || {
        std::thread::sleep(PORT_WAIT);
        let _ = tick.blocking_send(());
    });
    loop {
        let event = std::future::poll_fn(|cx| match timer.poll_recv(cx) {
            std::task::Poll::Ready(_) => std::task::Poll::Ready(Err(())),
            std::task::Poll::Pending => rx.poll_recv(cx).map(Ok),
        })
        .await;
        let line = match event {
            Err(()) => return EngineStart::Silent,
            Ok(None) => return EngineStart::Exited(None),
            Ok(Some(CommandEvent::Terminated(end))) => return EngineStart::Exited(end.code),
            Ok(Some(CommandEvent::Stdout(bytes) | CommandEvent::Stderr(bytes))) => String::from_utf8_lossy(&bytes).into_owned(),
            Ok(Some(_)) => continue,
        };
        if key.is_none() {
            *key = key_pattern.captures(&line).map(|found| found[1].to_string());
        }
        if let Some(port) = pattern.captures(&line).and_then(|found| found[1].parse::<u16>().ok()) {
            return EngineStart::Port(port);
        }
    }
}

/// Ends a launch whose engine never came up. Runs off the main thread: the dialog blocks.
fn quit_launch(handle: &tauri::AppHandle, reason: Option<&str>) {
    close_splash(handle);
    if let Some(reason) = reason {
        handle
            .dialog()
            .message(format!("The formsculpt engine failed to start: {reason}\n\nPlease relaunch the app. If it keeps happening, install formsculpt again."))
            .title("formsculpt")
            .kind(MessageDialogKind::Error)
            .blocking_show();
    }
    kill_backend(handle);
    handle.exit(1);
}

/// The engine has been silent for PORT_WAIT: true when the person chooses to go on waiting.
fn keep_waiting(handle: &tauri::AppHandle) -> bool {
    handle
        .dialog()
        .message("The formsculpt engine has not answered for a minute. A first start on a slow or busy computer can take longer.")
        .title("formsculpt")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("Keep Waiting".into(), "Quit".into()))
        .blocking_show()
}

/// The engine ended while the window was open. kill_backend empties the slot before it ends the engine, so an empty
/// slot means the app itself stopped it and there is nothing to report.
fn engine_stopped(handle: &tauri::AppHandle) {
    let ours = handle.state::<Backend>().0.lock().map(|mut slot| slot.take().is_none()).unwrap_or(true);
    if ours {
        return;
    }
    open_exit_gate(handle);
    let restart = handle
        .dialog()
        .message("The formsculpt engine has stopped, so this window can no longer work. Changes made since the last save are not kept; File > Recover Autosave brings back the last autosave after a restart.")
        .title("formsculpt")
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::OkCancelCustom("Restart".into(), "Quit".into()))
        .blocking_show();
    if restart {
        handle.restart();
    }
    handle.exit(1);
}

/// Hands the page a quit request so it can ask about unsaved work; false when the page cannot answer (it has not
/// started listening, or the window is gone), which means the app may exit at once.
fn ask_page_to_quit(handle: &tauri::AppHandle) -> bool {
    if handle.state::<ExitGate>().0.load(Ordering::SeqCst) {
        return false;
    }
    let ready = handle.state::<OpenQueue>().0.lock().map(|opens| opens.ready).unwrap_or(false);
    if !ready || handle.get_webview_window("main").is_none() {
        return false;
    }
    handle.emit("formsculpt://exit-requested", ()).is_ok()
}

fn show_main_window(handle: &tauri::AppHandle) {
    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Sends files to the page, or queues them until the page is listening.
fn open_paths(handle: &tauri::AppHandle, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    let send = match handle.state::<OpenQueue>().0.lock() {
        Ok(mut opens) => {
            if opens.ready {
                Some(paths)
            } else {
                opens.paths.extend(paths);
                None
            }
        }
        Err(_) => None,
    };
    if let Some(paths) = send {
        let _ = handle.emit("formsculpt://open-paths", paths);
        show_main_window(handle);
    }
}

/// The files named in a launch's arguments (the first is the program); a relative one is read from `cwd`.
fn paths_from_args(args: &[String], cwd: Option<&str>) -> Vec<String> {
    args.iter()
        .skip(1)
        .filter(|arg| !arg.starts_with('-'))
        .filter_map(|arg| {
            let path = std::path::PathBuf::from(arg);
            let full = match cwd {
                Some(dir) if path.is_relative() => std::path::Path::new(dir).join(path),
                _ => path,
            };
            full.is_file().then(|| full.to_string_lossy().into_owned())
        })
        .collect()
}

/// The page registers its listeners and then asks for the files that arrived before it was ready.
#[tauri::command]
fn take_pending_opens(queue: tauri::State<OpenQueue>) -> Vec<String> {
    match queue.0.lock() {
        Ok(mut opens) => {
            opens.ready = true;
            std::mem::take(&mut opens.paths)
        }
        Err(_) => Vec::new(),
    }
}

/// The page has saved or discarded unsaved work: the app may exit now. The engine is stopped by the Exit event.
#[tauri::command]
fn confirm_exit(app: tauri::AppHandle, cancel: Option<bool>) {
    if cancel == Some(true) {
        return;
    }
    open_exit_gate(&app);
    app.exit(0);
}

fn check_for_updates(handle: tauri::AppHandle) {
    let configured = handle
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|updater| updater.get("pubkey"))
        .and_then(|key| key.as_str())
        .is_some_and(|key| !key.is_empty());
    if !configured {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let Ok(updater) = handle.updater() else { return };
        let Ok(Some(update)) = updater.check().await else { return };
        let install = handle
            .dialog()
            .message(format!("formsculpt {} is available. Install it and restart?", update.version))
            .title("Update available")
            .buttons(MessageDialogButtons::OkCancel)
            .blocking_show();
        if install && update.download_and_install(|_, _| {}, || {}).await.is_ok() {
            open_exit_gate(&handle);
            kill_backend(&handle);
            handle.restart();
        }
    });
}

/// Saves a download through the native dialog. The page sends the file's bytes as the raw IPC body and its name in
/// the `x-file-name` header (percent-encoded); returns the chosen path, or None when the person cancels.
#[tauri::command]
async fn save_file(app: tauri::AppHandle, request: tauri::ipc::Request<'_>) -> Result<Option<String>, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected the file's bytes".into());
    };
    let raw = request.headers().get("x-file-name").and_then(|v| v.to_str().ok()).unwrap_or("formsculpt");
    let name = percent_decode(raw).replace(['/', '\\'], "_");
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    let mut dialog = app.dialog().file().set_file_name(&name);
    // Start in Documents (or home): the process's working directory is the AppImage's temporary mount.
    if let Some(dir) = app.path().document_dir().ok().filter(|d| d.is_dir()).or_else(|| app.path().home_dir().ok()) {
        dialog = dialog.set_directory(dir);
    }
    if !ext.is_empty() {
        dialog = dialog.add_filter(ext.to_ascii_uppercase(), &[ext.as_str()]);
    }
    let Some(chosen) = dialog.blocking_save_file() else {
        return Ok(None);
    };
    let path = chosen.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = std::str::from_utf8(&bytes[i + 1..i + 3]).map_err(|_| ()).and_then(|h| u8::from_str_radix(h, 16).map_err(|_| ())) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn open_main(handle: &tauri::AppHandle, port: u16, key: Option<&str>) {
    let origin = format!("http://127.0.0.1:{port}");
    let url: tauri::Url = origin.parse().expect("loopback url");
    let (width, height) = handle
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            (
                (size.width as f64 / scale) * SCREEN_FRACTION,
                (size.height as f64 / scale) * SCREEN_FRACTION,
            )
        })
        .unwrap_or(FALLBACK_LOGICAL);
    let builder = WebviewWindowBuilder::new(handle, "main", WebviewUrl::External(url))
        .title("FormSculpt")
        .visible(reveal::BUILD_VISIBLE)
        .background_color(reveal::BACKGROUND)
        .inner_size(width, height)
        .min_inner_size(MIN_LOGICAL.0, MIN_LOGICAL.1)
        .center();
    let builder = match key {
        Some(value) => builder.initialization_script(format!(
            "if (window.location.origin === {origin:?}) {{ window.__FORMSCULPT_WINDOW_KEY = {value:?}; }}"
        )),
        None => builder,
    };
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
    let builder = builder.on_page_load(|window, payload| {
        if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
            traffic_lights::place(&window.as_ref().window());
        }
    });
    if let Ok(window) = builder.build() {
        reveal::conceal(&window);
        traffic_lights::place(&window.as_ref().window());
    } else {
        close_splash(handle);
    }
}

/// Linux: the data folder this launch will use, found the way the engine finds it (FSC_DATA_DIR, else ~/Formsculpt), so
/// a second launch can tell whether a running window already owns it.
#[cfg(target_os = "linux")]
fn data_folder() -> std::path::PathBuf {
    use std::path::PathBuf;
    let var = |name: &str| std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let home = || var("HOME").map(PathBuf::from).unwrap_or_default();
    let folder = match var("FSC_DATA_DIR") {
        Some(dir) if dir == "~" => home(),
        Some(dir) => match dir.strip_prefix("~/") {
            Some(rest) => home().join(rest),
            None => PathBuf::from(dir),
        },
        None => home().join("Formsculpt"),
    };
    let absolute = if folder.is_absolute() { folder } else { std::env::current_dir().unwrap_or_default().join(folder) };
    std::fs::canonicalize(&absolute).unwrap_or(absolute)
}

/// One running window per data folder. The plugin keys its hand-off on the application identifier, so on Linux a second
/// data folder on the same login could not open a window; the key carries a hash of the data folder instead.
fn single_instance() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let plugin = tauri_plugin_single_instance::Builder::<tauri::Wry>::new().callback(|app, args, cwd| {
        open_paths(app, paths_from_args(&args, Some(&cwd)));
        show_main_window(app);
    });
    #[cfg(target_os = "linux")]
    let plugin = {
        let hash = data_folder().to_string_lossy().bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3));
        plugin.dbus_id(format!("ai.nodemode.formsculpt.d{hash:016x}"))
    };
    plugin.build()
}

fn main() {
    reveal::mark_launch();
    let app = tauri::Builder::default()
        .plugin(single_instance())
        .manage(OpenQueue(Mutex::new(Opens::default())))
        .manage(ExitGate(AtomicBool::new(false)))
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(event, WindowEvent::Resized(_) | WindowEvent::Focused(_) | WindowEvent::ThemeChanged(_) | WindowEvent::ScaleFactorChanged { .. })
            {
                traffic_lights::place(window);
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && ask_page_to_quit(window.app_handle()) {
                    api.prevent_close();
                }
            }
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Backend(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            save_file,
            take_pending_opens,
            confirm_exit,
            traffic_lights::set_window_title,
            reveal::app_ready,
            text_edit::text_edit
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            reveal::round_splash(&handle);
            let launch_args: Vec<String> = std::env::args().collect();
            open_paths(&handle, paths_from_args(&launch_args, None));
            let spawned = app
                .shell()
                .sidecar("formsculpt-engine")
                .and_then(|engine| engine.env("FSC_PORT", "0").env("FSC_PARENT_PID", std::process::id().to_string()).env("FSC_WINDOW_KEY_STDOUT", "1").spawn());
            let spawned = match spawned {
                Ok((rx, child)) => {
                    app.state::<Backend>().0.lock().unwrap().replace(child);
                    Ok(rx)
                }
                Err(err) => Err(err.to_string()),
            };

            tauri::async_runtime::spawn(async move {
                let mut rx = match spawned {
                    Ok(rx) => rx,
                    Err(err) => {
                        let reason = format!("it could not be run ({err}). It may be missing from the installation or blocked by security software.");
                        return quit_launch(&handle, Some(&reason));
                    }
                };
                let mut key: Option<String> = None;
                let port = loop {
                    match read_port(&mut rx, &mut key).await {
                        EngineStart::Port(found) => break found,
                        EngineStart::Silent if keep_waiting(&handle) => continue,
                        EngineStart::Silent => return quit_launch(&handle, None),
                        EngineStart::Exited(code) => {
                            let reason = match code {
                                Some(code) => format!("it ended before it was ready (exit code {code})."),
                                None => "it ended before it was ready.".to_string(),
                            };
                            return quit_launch(&handle, Some(&reason));
                        }
                    }
                };
                open_main(&handle, port, key.as_deref());
                check_for_updates(handle.clone());
                while let Some(event) = rx.recv().await {
                    if matches!(event, CommandEvent::Terminated(_)) {
                        break;
                    }
                }
                engine_stopped(&handle);
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("tauri app should build");

    app.run(|handle, event| match event {
        RunEvent::Exit => kill_backend(handle),
        RunEvent::ExitRequested { api, .. } => {
            if ask_page_to_quit(handle) {
                api.prevent_exit();
            }
        }
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => {
            let paths = urls
                .iter()
                .filter_map(|url| url.to_file_path().ok())
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            open_paths(handle, paths);
            show_main_window(handle);
        }
        _ => {}
    });
}
