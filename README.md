<div align="center">

<img src="assets/formsculptor_folder_icon.png" alt="formsculptor" width="120">

<h1>formsculptor</h1>

**3D sculpting & modeling for the web & the desktop, in the NodeMode style**

![version](https://img.shields.io/badge/version-0.8.0-f97316) ![rust](https://img.shields.io/badge/rust-1.80%2B-f97316) ![node](https://img.shields.io/badge/node-22-339933) ![python](https://img.shields.io/badge/python-3.12%2B-3776ab) ![tauri](https://img.shields.io/badge/tauri-2-24c8db) ![license](https://img.shields.io/badge/license-Apache--2.0-blue)

</div>

## What it is

formsculptor is the Form family's ZBrush alternative: a Rust sculpting engine driving a TypeScript viewport, a FastAPI engine service & a Tauri 2 desktop build. Every action is a keyed command in one registry, so the interface, the keyboard & outside agents (Claude Code, Codex, Cursor over MCP) all drive the same engine, w/ or w/o the window open. It is measured row by row against ZBrush 2026.2.1 & Nomad Sculpt 2.9.24 in the FormAI parity matrix.

Stack · Rust engine (`native/`, one Cargo workspace) · vanilla TypeScript & Vite (`frontend/`) · Python & FastAPI (`backend/`) · Tauri 2 (`src-tauri/`)

## Highlights

- **One command registry** · `native/crates/fsc-commands` declares every command once: dotted id, MCP name, JSON schema, annotations, label & keys. It exports the catalogue the UI, MCP server & agent skills read.
- **Engine in Rust, three targets** · the same crates are planned to compile to WASM for the window, a PyO3 module for the service (headless agent work) & native code for the desktop shell.
- **Desktop shell in formvis's shape** · splash window, frozen engine sidecar on an OS-assigned port, updater, `.fsc` file association, CI bundles for macOS, Linux & Windows.
- **Zero-flash shell** · theme applied before first paint, reserved layout, in-place text updates only.
- **Local only by default** · Vite binds 127.0.0.1 (`server.host` false), uvicorn binds 127.0.0.1.

## Quick Start

Requirements: Rust 1.80+, Node 20.19+ or 22.12+, Python 3.12+.

### macOS & Linux

```bash
./formsculptor_oc_install
./formsculptor doctor
./formsculptor
```

### Windows

```bat
formsculptor_oc_install.bat
formsculptor.bat
```

| Service | URL | Notes |
| --- | --- | --- |
| Frontend | http://localhost:5196 | Vite dev server, loopback only |
| Backend | http://127.0.0.1:8196 | FastAPI, `/api/health`, `/api/version`, `/api/capabilities` |
| Preview | http://localhost:5197 | `vite preview` |

Other commands: `./formsculptor engine build|check`, `build`, `serve`, `freeze`, `desktop dev|build`, `clean`. Run `./formsculptor help` for all of them.

## Configuration

Set in `backend/.env` (copy `backend/.env.example`; never committed).

| Variable | Purpose |
| --- | --- |
| `FSC_HOST` | Backend bind address, default 127.0.0.1 |
| `FSC_PORT` | Backend port, default 8196 (0 picks a free port, as the desktop shell does) |
| `FSC_CORS_ORIGINS` | Comma-separated extra origins |
| `FSC_DIST` | Built frontend to serve, set by `serve` & the desktop sidecar |

## Architecture

```text
formsculptor/
  native/            Rust engine workspace
    crates/fsc-core      mesh, scene, undo (planned contents)
    crates/fsc-sculpt    brushes, strokes, symmetry (planned)
    crates/fsc-topo      dyntopo, multires, remesh, booleans (planned)
    crates/fsc-paint     vertex paint, UVs, bakes (planned)
    crates/fsc-io        file formats & .fsc projects (planned)
    crates/fsc-commands  the command registry & catalogue
  frontend/          Vite app: shell, engine client, viewport
  backend/           FastAPI engine service, PyInstaller entry
  src-tauri/         Tauri 2 desktop shell
  setup/             one script per dispatcher command
  .claude/skills/    formsculptor-harness, the project skill pack
  .docs/             branching model & wiki source
```

## Development

Branching & versions follow [.docs/BRANCHING.md](.docs/BRANCHING.md). Work on the code routes through `.claude/skills/formsculptor-harness`. Edits are surgical, the DOM never flashes, every z-index is a token, & rationale lives in each side's `HASHOUT_LOG.md` rather than in comments.

## Status

0.8.0 sculpts, paints, bakes, retopologizes, manages topology, combines objects & reads and writes files: a quad remesh to a target count (quads in OBJ), detail projection between objects, & extract, extrude & inset from the mask; vertex paint (colour, roughness, metalness) w/ a Paint brush & fill that survives levels, dyntopo, remesh, booleans & export, a matcap library & a lit PBR mode; automatic UVs & map bakes (colour, roughness-metal, tangent-space normal from the highest level or another object, 32-bit EXR displacement) exported in GLB w/ tangents or as files; .fsc projects that reopen exactly (w/ autosave & recovery), import of OBJ, STL, PLY, glTF/GLB & 3MF, export to OBJ, STL, PLY, GLB (layers as morph targets) & 3MF, a print check (watertight, size, volume, wall thickness) & mesh repair; nine brushes (draw, clay, inflate, pinch, crease, flatten, smooth, grab, mask) w/ mirror & radial symmetry, dynamic topology that splits & collapses edges under the brush (brush, constant or screen detail), multiresolution levels (divide, step up & down w/ detail kept, delete higher & lower, rebuild lower), Loop subdivision, voxel remesh (watertight), quadric decimation, hole filling, watertight voxel booleans (union, difference, intersection), duplicate, merge & split, transform, bend, twist & taper held by the mask, sculpt layers w/ strength, masks, undo & redo, live strokes in the WebGL2 viewport, & an MCP server w/ 50 tools that drives the same engine w/ or w/o the window open (the window asks once before an agent's first change). Not yet: a field-aligned quad remesh w/ guides, bevel, edge loops & bridge, a low-distortion unwrap, paint layers, folders, an on-canvas transform gizmo, lattice, FBX, WebGPU, the WASM engine build (this build machine could not fetch the wasm32 target) & a built desktop bundle.

### Agents

`.mcp.json`, `.codex/config.toml` & `.cursor/mcp.json` start `./formsculptor mcp`; the `formsculptor-modeling` skill teaches the tools. `./formsculptor agents install` registers formsculptor for Claude Code, Codex & Cursor outside this folder; `./formsculptor agents sync` refreshes the skill's tool reference after a command changes.

## Contributors

| Name | Role |
| --- | --- |
| nodemode-ai | owner |

## Licensing

Apache License 2.0, see [LICENSE](LICENSE) & [NOTICE.md](NOTICE.md).
