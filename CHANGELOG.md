# Changelog

Versions follow `v*.*.*` with dots. Dates are UTC.

## 0.1.0 · 2026-10-11

### Added

- The scaffold: the `./formsculptor` dispatcher with one script per command in `setup/`, and Windows twins for install and boot.
- The Rust engine workspace in `native/` with six crates, and `fsc-commands`, the single command registry, with its first command (`system.info`, MCP name `get_engine_info`) and the `fsc-catalogue` exporter.
- The FastAPI engine service on 127.0.0.1:8196 with health, version and capabilities, cross-origin isolation headers, and the built frontend served from `/`.
- The frontend shell on Vite (port 5196, loopback only): tokens, the stacking scale, inline icons, theme applied before first paint, a live service status.
- The Tauri 2 desktop shell in formvis's shape, with a content security policy and the `.fsc` file association.
- CI for the frontend build, the engine check, the backend self-check and probe, and the script and text hygiene scans; the desktop release workflow; the wiki workflow.
- The `formsculptor-harness` project skill pack.
