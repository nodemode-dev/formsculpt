# Architecture

- `native/` is one Cargo workspace. `fsc-core`, `fsc-sculpt`, `fsc-topo`, `fsc-paint` and `fsc-io` hold the engine; `fsc-commands` holds the command registry and `dispatch`. The same crates are planned to compile to WASM for the window and to a PyO3 module for the service.
- Every command is declared once in `fsc-commands` with a dotted id, an MCP name, a JSON schema, annotations, a label and default keys. `fsc-catalogue` prints the catalogue as JSON; the UI, the MCP server and the agent skills read it.
- `frontend/` is a vanilla TypeScript app on Vite. The engine client never touches the DOM; the shell renders once and patches text and attributes in place.
- `backend/` is the FastAPI engine service. It serves the built frontend, which is how `serve` and the desktop window load the app.
- `src-tauri/` is the Tauri 2 shell: a splash window, the frozen engine started on port 0, the main window opened at the port the engine reports, the engine stopped on exit.
