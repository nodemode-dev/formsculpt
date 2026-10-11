---
name: verification
description: "Proves a formsculptor change: throttled DOM flicker check, perf bench, engine selfcheck, doctor, release gate and the multi-site version bump. Load before calling any formsculptor work done, before a release, or when asked what to run to check a change."
---

# Verification

## Purpose

Turn "should work" into output that was read. Hands back the checks run, their results and what was not run.

## Preconditions

State: v0.1.0 scaffold. Tests are not written until the build is stable; until then, intent goes into TEST_PLAN.md in the planning sidecar, not into the tree. Checks below are the intended set; mark any that do not exist yet as planned and say so.

## Procedure

1. Environment: `./formsculptor doctor`.
2. Build: `./formsculptor build` (engine, frontend, backend as available).
3. Engine selfcheck: run the Rust-side selfcheck for the touched crate; for commands, call `dispatch` with valid and invalid JSON.
4. Perf bench: `./formsculptor bench`; compare to the last recorded figure and flag regressions in sculpt and render.
5. Throttled DOM check: observe mutations on the viewport and shell at a throttled rate while idle and while sculpting. Expected: zero unexpected node churn, no flash or flicker.
6. Law scan on the diff: raw z-index numbers, icon-library imports, emoji, em dash in markdown, new comments.
7. Release gate: version bumped at every site together (frontend/package.json and lock, `frontend/src/version.ts`, `backend/app/__init__.py`, `native/Cargo.toml` workspace.package, `src-tauri/Cargo.toml`, `tauri.conf.json`, README badge, CHANGELOG), Conventional Commit authored as nodemode-ai, desktop build launches.
8. Report literally what ran and what did not.

## Gate

Done when each applicable step ran with output read, or is reported as not run with the reason.

## Hands back

A short list of checks with results.

## Owned resources

None bundled.
