# Third-party notices

formsculptor is licensed under the Apache License 2.0 (see `LICENSE`). Copyright 2026 nodemode-ai.

## Included in this repository

| Work | License | Where |
| --- | --- | --- |
| formvis desktop shell and setup scripts (nodemode-ai) | Apache-2.0 | `src-tauri/src/main.rs`, `setup/`, `.github/workflows/` (adapted) |
| NodeMode design system tokens | MIT | `frontend/src/styles/tokens.css` (values) |

## Distributed with the app

| Package | License | Notes |
| --- | --- | --- |
| Inter, JetBrains Mono (fontsource) | OFL-1.1 | bundled fonts |
| Tauri 2 crates and plugins | MIT or Apache-2.0 | desktop shell |
| regex, serde, serde_json | MIT or Apache-2.0 | shell and engine |
| zip (deflate only), base64 | MIT; MIT or Apache-2.0 | engine file formats (.fsc, 3MF, embedded glTF) |
| png | MIT or Apache-2.0 | baked texture maps |
| FastAPI, Starlette, uvicorn, python-multipart | MIT or BSD-3-Clause | engine service |
