---
name: verification
description: "Proves a formsculpt change: throttled DOM flicker check, perf bench, engine selfcheck, doctor, release gate and the multi-site version bump. Load before calling any formsculpt work done, before a release, or when asked what to run to check a change."
---

# Verification

## Purpose

Turn "should work" into output that was read. Hands back the checks run, their results and what was not run.

## Preconditions

State at 0.22.0: the build is stable (78.0 of 97 parity at 0.21.0) and the test pass has begun. `./formsculpt test` runs the engine suites (`native/crates/fsc-commands/tests`: registry invariants, modeling behaviour, files and renders, all through `dispatch` as the MCP server and window call it) and the service suite (`backend/tests`, pytest on the FastAPI app with the built engine module); `./formsculpt test e2e` runs `frontend/e2e/smoke.py` against a booted window under 4x throttling (layout shift 0, no console errors). CI runs the engine and service suites. At 0.27.0 the engine suite also runs `frontend/e2e/wasm_smoke.mjs` over the browser engine in Node when it is built, and a window check of the browser engine opens the window at `?engine=wasm` and counts requests to /api (there should be none). A test asserts what a HASHOUT entry measured; when a test and the code disagree, read the code and the log before changing either, since the test can be the one that is wrong.

## Procedure

1. Environment: `./formsculpt doctor`.
2. Build: `./formsculpt build` (engine, frontend, backend as available).
2a. Tests: `./formsculpt test` (engine and service) and, with `./formsculpt boot` running, `./formsculpt test e2e`. A new command gets a behaviour test in the matching `tests/*.rs` file.
3. Engine selfcheck: run the Rust-side selfcheck for the touched crate; for commands, call `dispatch` with valid and invalid JSON.
4. Perf bench: `./formsculpt bench`; compare to the last recorded figure and flag regressions in sculpt and render.
5. Throttled DOM check: observe mutations on the viewport and shell at a throttled rate while idle and while sculpting. Expected: zero unexpected node churn, no flash or flicker.
6. Law scan on the diff: raw z-index numbers, icon-library imports, emoji, em dash in markdown, new comments.
7. Release gate: version bumped at every site together (frontend/package.json and lock, `frontend/src/version.ts`, `backend/app/__init__.py`, `native/Cargo.toml` workspace.package, `src-tauri/Cargo.toml`, `tauri.conf.json`, README badge, CHANGELOG), Conventional Commit authored as nodemode-ai, desktop build launches. On Linux, `./formsculpt desktop build` then the frozen engine's `--selfcheck`, and the AppImage launched under `xvfb-run` with no dev service on 8260 (the sidecar takes a free port of its own, so probe the window, not 8260).
8. Report literally what ran and what did not.

## Gate

Done when each applicable step ran with output read, or is reported as not run with the reason.

## Hands back

A short list of checks with results.

## Owned resources

None bundled.
