# Third-party notices

formsculpt is licensed under the Apache License 2.0 (see `LICENSE`). Copyright 2026 NodeAgentc, Inc.

## Included in this repository

| Work | License | Where |
| --- | --- | --- |
| formvis desktop shell and setup scripts (nodemode-ai) | Apache-2.0 | `src-tauri/src/main.rs`, `setup/`, `.github/workflows/` (adapted) |
| NodeMode design system tokens | MIT | `frontend/src/styles/tokens.css` (values) |
| formlit and formsurface terminal dock, chat pane, markdown, Claude Code and chat services, build_app (nodemode-ai) | Apache-2.0 | `frontend/src/ui/{terminal-dock,chat-pane,markdown}.ts`, `frontend/src/io/`, `frontend/src/styles/dock.css`, `backend/app/services/{terminal,cmd_terminal,claude_code,cmd_chat,commands,events}.py`, `build_app` (adapted) |
| formvis command registry, menus, palette and interface languages (nodemode-ai) | Apache-2.0 | `frontend/src/ui/{commands,menu,command-palette,i18n}.ts`, `frontend/src/ui/i18n/langs.ts` (adapted) |

## Distributed with the app

| Package | License | Notes |
| --- | --- | --- |
| Inter, JetBrains Mono (fontsource) | OFL-1.1 | bundled fonts |
| Tauri 2 crates and plugins | MIT or Apache-2.0 | desktop shell |
| regex, serde, serde_json | MIT or Apache-2.0 | shell and engine |
| zip (deflate only), base64 | MIT; MIT or Apache-2.0 | engine file formats (.fsc, 3MF, embedded glTF) |
| png | MIT or Apache-2.0 | baked texture maps |
| boolmesh (Saki Komikado) | MPL-2.0 | exact mesh booleans; used unmodified from crates.io, its source available there and at github.com/komietty/boolmesh |
| FastAPI, Starlette, uvicorn, python-multipart | MIT (FastAPI), BSD-3-Clause (Starlette, uvicorn), Apache-2.0 (python-multipart) | engine service |
| @xterm/xterm, @xterm/addon-fit | MIT | the terminal dock |
| ptyprocess (Unix), pywinpty (Windows) | ISC; MIT | terminal sessions in the engine service |
| Other Python packages frozen into the engine | MIT: annotated-doc, annotated-types, anyio, h11, httptools, pydantic, pydantic-core, pyyaml, typing-inspection, watchfiles; BSD-3-Clause: click, idna, python-dotenv, websockets; MIT OR Apache-2.0: uvloop; PSF-2.0: typing-extensions and the CPython runtime | read from the installed metadata of `backend/requirements.txt`, 2026-10-23 |
| pytest 9.1.1 (with iniconfig, packaging, pluggy, pygments) | MIT (pytest, iniconfig, pluggy), Apache-2.0 OR BSD-2-Clause (packaging), BSD-2-Clause (pygments) | runs `backend/tests` in CI (`backend/requirements_test.txt`); never installed in the app |
| Playwright for Python 1.63.0 | Apache-2.0 | drives Chromium in the window smoke, `frontend/tests/window` in CI; never installed in the app |

## MPL-2.0 crates linked

Five crates under the Mozilla Public License 2.0 are linked, all unedited and from crates.io (checked with `cargo metadata --locked` on `native/Cargo.lock` and `src-tauri/Cargo.lock`): boolmesh 0.2.1 in the sculpting engine workspace (listed above), and, through Tauri, cssparser 0.36.0, cssparser-macros 0.6.1, selectors 0.36.1, dtoa-short 0.3.5 and option-ext 0.2.0. Their source is at https://crates.io/crates/<name>/<version>.

## Desktop shell: crates under other licences

Tauri links these crates unedited (no `[patch]` and no vendored copy):

| Crate | Licence | Why it is there |
| --- | --- | --- |
| cssparser, selectors, dtoa-short, option-ext | MPL-2.0 | Tauri's HTML handling (tauri-utils); used unmodified, source at crates.io |
| ring | Apache-2.0 AND ISC | TLS for the updater (rustls) |
| webpki-root-certs | CDLA-Permissive-2.0 | root certificates for the updater's TLS |
| r-efi | MIT OR Apache-2.0 OR LGPL-2.1-or-later | used under MIT |

The Linux AppImage also carries the build machine's WebKitGTK, GTK and GLib (LGPL-2.1-or-later) as separate shared libraries.

## Trademarks

ZBrush, ZRemesher, DynaMesh, ZSpheres, Polypaint and LightBox are trademarks or registered trademarks of Maxon Computer GmbH. Nomad Sculpt is a trademark of its owner. Blender is a registered trademark of the Blender Foundation. Unreal is a trademark of Epic Games, Inc. Unity is a trademark of Unity Technologies. Other names are the property of their owners. They are used only to describe compatibility, file formats and integrations; formsculpt is not affiliated with or endorsed by them. formsculpt's features carry its own names (ZUI Layout, LightCube, Quad Remesh, Voxel Remesh, Armature Spheres, Vertex Paint, Pose Part), and no other product's artwork, brushes, presets or code are included.
