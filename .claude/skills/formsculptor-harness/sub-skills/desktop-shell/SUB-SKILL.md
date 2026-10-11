---
name: desktop-shell
description: "Covers the formsculptor Tauri 2 desktop shell in src-tauri: splash window, frozen engine sidecar spawn with port 0, main window URL, child cleanup, updater, .fsc file association and identifier ai.nodemode.formsculptor. Load for desktop dev or build, sidecar, installer or window lifecycle work."
---

# Desktop Shell

## Purpose

Own `src-tauri/`, in formvis's shape. Hands back a desktop change that launches, finds the engine and exits clean.

## Preconditions

State: v0.1.0 scaffold; the full lifecycle is planned until code exists. The sidecar comes from `./formsculptor freeze` and lives as `binaries/formsculptor-engine-[triple]`.

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
