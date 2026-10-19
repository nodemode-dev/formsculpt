---
name: formsculptor-harness
description: "Guides work on formsculptor, the Form-family 3D sculpting app by nodemode-ai: Rust engine, WASM and PyO3 bindings, MCP tools, Tauri desktop. Use whenever a task touches sculpting, brushes, dyntopo or fsc-topo holes, voxel remesh, multires, masks, layers, the viewport dirty range upload, the command registry, command palette, MCP catalogue, .fsc or glTF import, the Tauri sidecar, doctor and bench runs, version bumps (1.x from 1.0.0), GoF and the integrations, or any file under native/, frontend/src, backend/app, src-tauri or setup/. Enforces project laws (SEDO, zero DOM flicker, z-index tokens). Not for formvis."
license: Apache-2.0
metadata:
  nodemode.author: nodemode-ai
  nodemode.pack-version: "0.2.0"
---

# Formsculptor Harness

formsculptor is a Form-family 3D sculpting and modeling app by nodemode-ai (Apache-2.0), the ZBrush alternative, measured against ZBrush 2026.2.1 and Nomad Sculpt 2.9.24. It is modeled on the sibling repo formvis. Current state: v0.1.0 scaffold. The engine crates are minimal shells, so anything described below as a design is marked planned in the sub-skills until code exists. Read the code before trusting a claim here.

This pack exists so that an agent entering the repo cold edits the right layer, in the right order, under the project laws, and proves the result.

## Layout

```
formsculptor-harness/
  SKILL.md                      this file, the router
  references/evals.md           three evaluations and the trigger list
  sub-skills/[skill-name]/SUB-SKILL.md
    engine-core  sculpt-brushes  topology  render-viewport  io-formats
    command-registry  agent-substrate  ui-shell  engine-service
    desktop-shell  verification
```

Sub-skill paths resolve as `sub-skills/[skill-name]/SUB-SKILL.md` from this folder.

## Subsystem map

| Area | Location | Role |
| --- | --- | --- |
| Rust engine | `native/` (one Cargo workspace) | all geometry, sculpt, topology, paint, IO, commands |
| fsc-core | `native/crates/fsc-core` | mesh SoA buffers, adjacency, normals, BVH, scene, masks, face groups, vertex colour, sculpt layers, region delta undo |
| fsc-sculpt | `native/crates/fsc-sculpt` | stroke sampling, falloff, pressure, lazy rope, brush kernels, mirror and radial symmetry, rayon |
| fsc-topo | `native/crates/fsc-topo` | dynamic topology, multires, voxel remesh, QEM decimation, quad remesh, booleans, hole fill |
| fsc-paint | `native/crates/fsc-paint` | vertex PBR paint, auto UV, bakes |
| fsc-io | `native/crates/fsc-io` | OBJ, STL, PLY, glTF/GLB, 3MF, FBX import, the `.fsc` project zip |
| fsc-commands | `native/crates/fsc-commands` | the single command registry and `dispatch(id, json)` |
| fsc-wasm | `native/crates/fsc-wasm` (built at 0.27.0; `./formsculptor engine wasm`) | plain C ABI over fsc-commands and fsc_commands::live, no threads, loaded on the page's main thread by `frontend/src/engine/wasm.ts` when no engine service answers |
| fsc-py | `native/crates/fsc-py` | PyO3 module `fsc_engine` built with maturin; the backend's engine, which the window and MCP both drive |
| Frontend | `frontend/` (vanilla TypeScript, Vite, port 5196) | `src/engine/` worker client, `src/viewport/` renderer, `src/ui/` shell, `src/styles/tokens.css` |
| Backend | `backend/app/` (FastAPI, 127.0.0.1:8196) | routers system, io, projects, jobs, assets, mcp, nodemode; serves the built frontend |
| Sidecar entry | `backend/formsculptor_engine.py` | PyInstaller entry for the frozen engine |
| Desktop | `src-tauri/` (Tauri 2) | splash, sidecar spawn, main window, file association |
| Dispatcher | `./formsculptor`, `./build_app` and `setup/` | one script per task; `build_app` (and `build_app.bat`) builds the installable desktop app start to finish |

## Dependency graph

```
fsc-core  <-  fsc-sculpt, fsc-topo, fsc-paint, fsc-io
fsc-commands  ->  fsc-core, fsc-sculpt, fsc-topo, fsc-paint, fsc-io
fsc-wasm, fsc-py  ->  fsc-commands
frontend/src/engine (service.ts, wasm.ts)  ->  fsc-wasm (no service) or the engine service (fsc-py)
viewport (frontend/src/viewport)  <-  WASM buffers by dirty range
backend/app (mcp router, engine router, stroke socket)  ->  fsc-py (one shared engine session)
window  ->  /api/engine (dispatch, mesh, stroke socket) and /api/mcp/window (change epoch, consent)
src-tauri  ->  frozen backend sidecar  ->  built frontend
catalogue.json (from fsc-commands)  ->  MCP server, palette, menus, shortcuts, agents sync
```

The dependency edges between the leaf crates are the intended design; confirm against the Cargo.toml files before relying on one.

## Laws

These come from the dev-rules operating mode and apply to every sub-skill.

1. SEDO: surgical hand edits only. No scripts, patches, sed, git apply or codemods to change source.
2. Zero DOM flash, flicker or re-render. Verify with a throttled check (see verification).
3. Every z-index is a `var(--z-*)` token from `frontend/src/styles/tokens.css`.
4. No icon libraries. Icons are inline currentColor SVG in `frontend/src/ui/icons.ts`.
5. No emoji. No em dash in markdown (use the interpunct).
6. Near-zero comments. Rationale goes to `frontend/HASHOUT_LOG.md`, `backend/HASHOUT_LOG.md` or `native/HASHOUT_LOG.md`.
7. Versions v*.*.* are bumped together at every site: frontend/package.json and lock, `frontend/src/version.ts`, `backend/app/__init__.py`, `native/Cargo.toml` workspace.package, `src-tauri/Cargo.toml` and `tauri.conf.json`, README badge, CHANGELOG.
8. Conventional Commits, authored as nodemode-ai.
9. Tests only after the build is stable. Until then testing intent goes to TEST_PLAN.md in the planning sidecar.
10. Planning docs (DEV_PLAN, BUILD_NOTES, TEST_PLAN, root HASHOUT_LOG) live in the planning sidecar zip, not in the tree.
11. Licences: no GPL code (Blender is study only), no code from ZBrush or Nomad.
12. The Rust registry in fsc-commands is the one source of truth for commands. Never hand-write a tool list elsewhere.

## Routing table

| If the task is about | Load |
| --- | --- |
| Mesh buffers, adjacency, BVH, scene, masks, face groups, vertex colour, layers, undo | `sub-skills/engine-core/SUB-SKILL.md` |
| Brushes, strokes, falloff, pressure, symmetry, rope | `sub-skills/sculpt-brushes/SUB-SKILL.md` |
| Dyntopo, multires, voxel remesh, decimation, quad remesh, booleans, hole fill | `sub-skills/topology/SUB-SKILL.md` |
| WebGPU or WebGL2 viewport, shading, dirty-range uploads | `sub-skills/render-viewport/SUB-SKILL.md` |
| OBJ, STL, PLY, glTF, 3MF, FBX, the `.fsc` zip | `sub-skills/io-formats/SUB-SKILL.md` |
| Adding or changing a command end to end | `sub-skills/command-registry/SUB-SKILL.md` |
| MCP server, window bridge, consent, headless path, stdio shim, agent configs | `sub-skills/agent-substrate/SUB-SKILL.md` |
| Terminal dock, Claude Code tab, Chat tab, the window command bus, PTY sessions | `sub-skills/agent-substrate/SUB-SKILL.md` § The dock |
| UI shell, palette, menus, shortcuts, tokens, icons, worker client | `sub-skills/ui-shell/SUB-SKILL.md` |
| Interface languages, strings.json, a new interface string | `sub-skills/ui-shell/SUB-SKILL.md` § Languages |
| FastAPI routers, PyO3 use, jobs, assets, sidecar entry | `sub-skills/engine-service/SUB-SKILL.md` |
| Tauri, sidecar spawn, splash, updater, file association, packaging | `sub-skills/desktop-shell/SUB-SKILL.md` |
| Proving a change: DOM check, bench, selfcheck, release gate, version bump | `sub-skills/verification/SUB-SKILL.md` |

A cross-cutting task loads every row it touches. Adding a brush, for example, loads sculpt-brushes then command-registry then verification.

## Start-of-session reading order

1. This file.
2. Orientation: list `native/`, `frontend/src/`, `backend/app/`, `src-tauri/`, `setup/`.
3. `native/Cargo.toml` for the crate list and workspace version.
4. The directory of the crate you will touch (names above), then the matching sub-skill.
5. `native/HASHOUT_LOG.md`, `frontend/HASHOUT_LOG.md` or `backend/HASHOUT_LOG.md` for the layer, to learn past rationale.
6. The planning sidecar (DEV_PLAN, BUILD_NOTES) when it is available to you.

## Verification bar

A change is done only when the matching check in `sub-skills/verification/SUB-SKILL.md` ran and its output was read. Say what ran, not what should work. Anything not yet built stays marked planned.
