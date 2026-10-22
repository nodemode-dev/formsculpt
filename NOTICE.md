# Third-party notices

formsculpt is licensed under the Apache License 2.0 (see `LICENSE`). Copyright 2026 nodemode-ai.

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
| FastAPI, Starlette, uvicorn, python-multipart | MIT or BSD-3-Clause | engine service |
| @xterm/xterm, @xterm/addon-fit | MIT | the terminal dock |
| ptyprocess (Unix), pywinpty (Windows) | ISC; MIT | terminal sessions in the engine service |
