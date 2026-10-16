<div align="center">

<img src="assets/formsculptor_folder_icon.png" alt="formsculptor" width="120">

<h1>formsculptor</h1>

**3D sculpting & modeling for the web & the desktop, in the NodeMode style**

![version](https://img.shields.io/badge/version-0.51.0-f97316) ![rust](https://img.shields.io/badge/rust-1.88%2B-f97316) ![node](https://img.shields.io/badge/node-22-339933) ![python](https://img.shields.io/badge/python-3.12%2B-3776ab) ![tauri](https://img.shields.io/badge/tauri-2-24c8db) ![license](https://img.shields.io/badge/license-Apache--2.0-blue)

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
- **Claude in the window** · a terminal dock with shells, a Claude Code tab and a Chat tab, wired to the same MCP tools.
- **Menus, palette & shortcuts** · a menu bar and right-click menus built from the command registry, a `Mod+K` palette over every command & a `Mod+/` shortcut editor.
- **26 languages** · English by default; switch in View > Language, live, no reload.
- **A studio viewport** · a gradient backdrop w/ vignette (or a flat colour) & an infinite, anti-aliased floor grid that fades to the horizon, as in ZBrush & Nomad; Shift+P toggles the grid.

## Quick Start

Requirements: Rust 1.88+, Node 20.19+ or 22.12+, Python 3.12+.

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

### Build the desktop app in one step

```bash
./build_app            # macOS & Linux: tools checked, everything built, the app bundled (and installed on macOS)
./build_app --open     # the same, then open it
build_app.bat          # Windows: msi & nsis installers
```

Every step is logged to `build_app.log`. Flags: `--clean`, `--install` (Linux deb), `--no-install` (macOS); `FSC_BUNDLES=deb,rpm,appimage` for more Linux bundles.

| Service | URL | Notes |
| --- | --- | --- |
| Frontend | http://localhost:5196 | Vite dev server, loopback only |
| Backend | http://127.0.0.1:8196 | FastAPI, `/api/health`, `/api/version`, `/api/capabilities` |
| Preview | http://localhost:5197 | `vite preview` |

Other commands: `./formsculptor engine build|check|wasm` (`wasm` builds the browser engine, which the window uses on its own when no service answers, or at `?engine=wasm`), `build`, `serve`, `freeze`, `desktop dev|build`, `clean`. Run `./formsculptor help` for all of them.

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

`./formsculptor test` runs the engine suites (registry, modeling, files & renders, all through the command registry) & the service suite; `./formsculptor test e2e` drives a booted window in Chromium under 4x throttling & fails on any layout shift or console error. CI runs the engine & service suites.

## Status

0.51.0 adds selection shapes (smooth curve, path band, line, flip) that mask, hide, group or split, & balloon, extrude, splat & project mesh from a mask. 0.50.0 adds alpha stretch, rotation, tiling, mid value & invert, saved tool presets & a brush cursor ring. 0.49.0 adds stroke filters (front faces only, connected only, same face group, keep sharp edges, accumulate off, area sampling & depth masking). 0.48.0 adds stroke options (even deposit at any spacing, brush offset, seeded jitter, snapped radius, a constant 3D radius & per-brush radius). 0.47.0 adds Elastic, Polish, Layer (w/ a height cap), Nudge, Stamp & Layer reset brushes. 0.46.0 adds nested repeaters (line, radial, mirror & curve steps merged, separate or linked) & scattering copies over a surface. 0.45.0 exports UDIM objects w/ one material per tile & shows every tile at once in the UV view. 0.44.0 adds UDIM tiles (tiles by face group or by area, islands moved between tiles, per-tile bakes in Mari naming). 0.43.0 adds automatic UV charts & a UV checker in the viewport. 0.42.0 lets the UV view pin points & draw vertex paths for cutting & straightening. 0.41.0 adds UV sew, cut & straighten. 0.40.0 adds UV texel density readout & equalizing & a conformal UV relax w/ pins. 0.39.0 adds Curve Bridge & Snake Curve (`curve_bridge`) & a live re-edit that re-applies a stroke as its curve handles move. 0.38.0 adds Slice curve & Trim curve (split face groups or cut an object along a drawn line) & `group_from_outline`. 0.37.0 gives arrays a scale ramp & seeded jitter & the falloff curve smooth or corner points. 0.36.0 adds a tube profile editor w/ saved profiles & intensity that fades or pulses along a curve stroke. 0.35.0 makes curves from clicked points, from the last brush stroke (undone & brought back as handles to move & apply again) & from a mask edge or face-group border (`curve_from_border`). 0.34.0 gives curves a radius & twist per handle, start-to-end or per-point tube radii, surface offset & grid snap, & a drag that moves the handles as a fixed-length chain (`curve_ik`), & lists Selection Sets in the panel. 0.33.0 replaces the viewport's flat background & line grid w/ a studio gradient & an infinite floor grid, adds ZBrush-style Selection Sets (named snapshots of groups, mask, paint, layer strengths & UVs) & a channel clipboard that moves a mask, groups or paint between objects, & fills all 26 interface languages. 0.32.0 puts Claude in the window (a terminal dock w/ shells, a Claude Code tab & a Chat tab), builds the desktop app in one step (`./build_app`), gives every action a command w/ a menu bar, context menus, a palette & a shortcut editor, opens the interface to 26 languages, & makes paths real curves: points smooth or sharp, corners rounded by a radius, smooth, softer or straight paths, closed loops, tubes from a curve w/ circle, square, triangle, star or strap profiles, round or flat ends, hollow pipes & spirals, rounded lathe profiles & objects bent along a curve. Before it, 0.31.0 sculpts, paints, bakes, retopologizes, builds base meshes, manages topology, combines objects, reads and writes files & runs as a desktop app (a Linux AppImage, deb & rpm built from this tree & launched on its own bundled engine), w/ engine, service & window tests: skeleton base meshes posed joint by joint by their own skeleton w/ automatic weights (sculpted detail rides along), grown fibers combed, cut, curled & lengthened; a quad remesh steered by guide curves, a chamfer or round bevel on a chosen edge run & planar clips & slices (both exact booleans), ZSphere-style skeleton editing (click to grow a joint, drag to move one), single UV points dragged by hand, Substance Painter's textures read back onto the paint, & path renders w/ added lights, depth of field & depth, normal & AO passes; cloth draped over the scene, Transpose-style posing, grown fibers (hair, fur, grass), a path-traced production render, manual retopology quad by quad, edge loops, bridges & face deletion, a Substance Painter package; exact mesh booleans (cut along the true intersection, detail & paint kept) w/ a Live boolean preview; a browser engine (the whole engine as WebAssembly, so the window sculpts, paints, remeshes & saves w/ no service behind it), USD as text (.usda) both ways; opacity & emissive paint channels (see-through and glowing paint in the viewport, GLB & baked maps), UV editing (seams along face groups, a UV layout view where islands are chosen, moved, turned, scaled & packed); a smooth preview that shows any object up to three subdivision levels smoother without changing it (dynamic subdivision), a conformal UV unwrap that keeps angles (mean distortion about 1.01 against 1.2 for the box projection); binary FBX import & export (layers as blend shapes, checked against assimp), subsurface scattering w/ real thickness & refractive transmission in lit shading, & a 3D mouse over WebHID; a volumetric bevel that rounds convex & concave edges, tubes & lathes that stay editable until sculpted, a two-way file bridge to any app that opens & saves OBJ or GLB, & plugins that declare their own fields & surface tools; an insert brush that strings copies along a stroke, face groups by paint, by UV island or painted w/ a group brush, a keyframed timeline of camera moves & sculpt-layer morphs rendered to an animated PNG, an undo-history movie, & glTF written & read w/ its .bin beside it; vector displacement stamps (horn, hook & mushroom built in, more captured from a sculpted plane) that overhang, solid 3D text & extruded vector outlines w/ holes, booleans w/ a filleted seam that fuse parts, & lasso cuts & trims straight through an object; shadow-box solids from front, side & top outlines drawn w/ a Silhouette tool, mirror copies & arrays along a curve, linked copies that follow their original, & a panel whose sections fold away, which sits on either side & comes in three sizes, w/ any of its buttons pinned to the tool rail; images projected from the view or tiled triplanar into paint, reference image planes in the scene, Python scripting (`./formsculptor script`) & plugins listed & run from the window; a movable key light w/ shadows & up to three added lights, a ground shadow catcher, a noise editor (value, ridged, turbulence, cells & tiled alpha noise w/ octaves, gain & stretch) & primitives that stay editable until changed; an on-canvas transform gizmo (move, turn & scale per axis or evenly), lattice deformation, two-finger pan & pinch zoom & pen tilt turning the brush alpha; image-based lighting from four built-in environments or a loaded .hdr, turned, exposed & tone-mapped (Reinhard or ACES), shown behind the model if wanted, & post effects (ambient occlusion, bloom, depth of field, vignette, grain); paint layers w/ blend modes (normal, multiply, screen, overlay, add), opacity & visibility, reordered, merged down or flattened, saved in projects; curve strokes drawn on the surface w/ handles you drag before applying any brush along them, spray strokes, drag dot, a drawn falloff curve & a pen pressure curve that sets strength or size; brush alphas (seven built in, any PNG loaded) on every brush, upright, turned or rolled along the stroke, a drag stamp, a smudge brush for paint, image matcaps & shading per object, & custom hotkeys over the registry's keys; face groups (automatic by angle or parts, from the mask) shown in colour, masked & hidden by group or mask; arrays, insert mesh, folders, a reference image, trackball rotation, sharp-border subdivision & turntable recording; fourteen brushes (adding relax, planar, trim, snake hook & noise) w/ a lazy rope, surface noise in a layer, masks by box, lasso & ellipse, by topology, by cavity or peaks & by paint colour w/ grow & shrink, hide & isolate, align & distribute, a pivot on a double-clicked point, a field-of-view control & an undo history list you can jump through; a Linux AppImage, deb & rpm built from this tree (native save dialog, the engine inside), Send to NodeMode (standalone or embedded in its portal), & a headless render agents can look through (`get_view`); base meshes from a skeleton of spheres (watertight, w/ figure, animal, hand & snake presets), tubes swept along a drawn or given curve, & lathe solids from a profile; a quad remesh to a target count (quads in OBJ), detail projection between objects, & extract, extrude & inset from the mask; vertex paint (colour, roughness, metalness) w/ a Paint brush & fill that survives levels, dyntopo, remesh, booleans & export, a matcap library & a lit PBR mode; automatic UVs & map bakes (colour, roughness-metal, tangent-space normal from the highest level or another object, 32-bit EXR displacement) exported in GLB w/ tangents or as files; .fsc projects that reopen exactly (w/ autosave & recovery), import of OBJ, STL, PLY, glTF/GLB & 3MF, export to OBJ, STL, PLY, GLB (layers as morph targets) & 3MF, a print check (watertight, size, volume, wall thickness) & mesh repair; nine brushes (draw, clay, inflate, pinch, crease, flatten, smooth, grab, mask) w/ mirror & radial symmetry, dynamic topology that splits & collapses edges under the brush (brush, constant or screen detail), multiresolution levels (divide, step up & down w/ detail kept, delete higher & lower, rebuild lower), Loop subdivision, voxel remesh (watertight), quadric decimation, hole filling, watertight voxel booleans (union, difference, intersection), duplicate, merge & split, transform, bend, twist & taper held by the mask, sculpt layers w/ strength, masks, undo & redo, live strokes in the WebGL2 viewport, & an MCP server w/ 145 tools that drives the same engine w/ or w/o the window open (the window asks once before an agent's first change). Not yet: a fully field-aligned quad remesh (guides set one direction for the whole object), panel lines along face groups, a rig for meshes not built from a skeleton, painted weights, cloth brushes, HD geometry (sculpting levels too heavy to keep loaded), painting straight into texture maps, binary USD, WebGPU, threads in the browser engine, & macOS & Windows bundles built and run (CI builds them; none was run here).

### Agents

Inside the window: the terminal dock under the viewport (`Mod+J`, or the app-bar buttons) holds real shells, a **Claude Code** tab (`Shift+Mod+J`) running your own Claude Code sign-in already connected to formsculptor's tools (no permission prompt for it), and a **Chat** tab (`Alt+Mod+J`) where Claude answers in plain language and sculpts through the same tools; each change is one undo step. The ask line sends a request with the active object as context. Claude Code is found on your PATH (or `FSC_CLAUDE_BIN`); it starts in `~/Formsculptor/Agent` (`FSC_AGENT_DIR`).

`.mcp.json`, `.codex/config.toml` & `.cursor/mcp.json` start `./formsculptor mcp`; the `formsculptor-modeling` skill teaches the tools. `./formsculptor agents install` registers formsculptor for Claude Code, Codex & Cursor outside this folder; `./formsculptor agents sync` refreshes the skill's tool reference after a command changes.

## Contributors

| Name | Role |
| --- | --- |
| nodemode-ai | owner |

## Licensing

Apache License 2.0, see [LICENSE](LICENSE) & [NOTICE.md](NOTICE.md).
