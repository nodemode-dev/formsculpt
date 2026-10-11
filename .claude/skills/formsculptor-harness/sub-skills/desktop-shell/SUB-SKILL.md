---
name: desktop-shell
description: "Covers the formsculptor Tauri 2 desktop shell in src-tauri: splash window, frozen engine sidecar spawn with port 0, main window URL, child cleanup, updater, .fsc file association and identifier ai.nodemode.formsculptor. Load for desktop dev or build, sidecar, installer or window lifecycle work."
---

# Desktop Shell

## Purpose

Own `src-tauri/`, in formvis's shape. Hands back a desktop change that launches, finds the engine and exits clean.

## Preconditions

State at 0.9.0: built and bundled on Linux (AppImage and deb). `setup/freeze_backend` rebuilds `fsc_engine` when it is missing or its version differs from the service, freezes with `--collect-all fsc_engine`, and runs `--selfcheck`, which dispatches a real command and fails unless the engine and the frontend are inside. The sidecar lives as `binaries/formsculptor-engine-[triple]`. The main window shows the page the sidecar serves on the loopback port; capability `engine-page` (remote `http://127.0.0.1:*`) lets that page call the one app command, `save_file`, which takes the file as the raw IPC body and its percent-encoded name in `x-file-name`, shows the native save dialog and writes the file (`frontend/src/ui/desktop.ts` calls it; the browser keeps blob downloads). Tauri crates are pinned to formvis's tested set (tauri 2.11.6 with @tauri-apps/api 2.11.1): the bundler refuses a crate and npm minor mismatch, so move both together. A remote page gets no app command by default: every new command goes into `build.rs`'s `AppManifest::commands` (which generates `allow-<name>`) and into `capabilities/engine-page.json`, or the page sees "not allowed by ACL". Both windows set the background to #111111 so the first frame is not white. The shell passes `FSC_PARENT_PID`; the engine's watchdog exits when that process is gone (the shell's `kill_backend` only runs on a clean exit, and SIGKILL to the PyInstaller bootloader does not reach its child). Verify a launch under Xvfb: frames for the first two seconds (no white), a real close via `WM_DELETE_WINDOW` (xdotool's windowclose destroys the window and is not a close), then `kill -9` of the shell, and count `formsculptor-engine` processes after each.

## Procedure

1. Startup: show the splash window, spawn the frozen sidecar with port 0, read the uvicorn port line, open the main window at that URL.
2. On exit, kill the child process. Verify no orphan remains.
3. The updater is skipped while the pubkey is empty. Do not ship a placeholder key.
4. File association: `.fsc`. Identifier: `ai.nodemode.formsculptor`.
5. Version sites `src-tauri/Cargo.toml` and `tauri.conf.json` bump with all others (`sub-skills/verification/SUB-SKILL.md`).
6. Commands: `./formsculptor desktop dev` and `desktop build`; `install-desktop` for prerequisites.
7. The port line contract is owned by `sub-skills/engine-service/SUB-SKILL.md`.

## Gate

Done when desktop dev opens the main window against the sidecar and closing it leaves no engine process.

## Hands back

The change and the launch and exit evidence.

## Owned resources

None bundled.
