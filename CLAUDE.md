# formsculptor

3D sculpting and modeling app for the web and the desktop in the NodeMode design system: Rust engine workspace, vanilla TypeScript and Vite frontend, Python and FastAPI service, Tauri 2 shell.

## Repo layout notes

- `assets/` is repo-level art (the README logo and the desktop icon source), not app runtime assets: runtime files live in `frontend/public/` and `frontend/src/`.
- `native/crates/fsc-commands` is the only place a command is declared. The UI, the MCP server and the agent skills read its catalogue; never add a tool name anywhere else. The one exception is window-only service commands that must never reach MCP (`terminal.*`, `claudecode.*`, `chat.*`, `agents.prefs`), which live on the service's bus in `backend/app/services/`.
- `./build_app` (and `build_app.bat`) builds the installable desktop app start to finish; `./formsculptor app` runs it.
- A new interface string lands in every table under `frontend/src/i18n/` in the same change (`./formsculptor i18n extract`, then `check`).
- `frontend/src/engine/` never touches the DOM; UI lives in `frontend/src/ui/`.
- `backend/formsculptor_engine.py` is the PyInstaller entry for the desktop sidecar, not a dev server. Use `./formsculptor backend` or `./formsculptor boot`.
- `src-tauri/binaries/` receives the frozen engine from `setup/freeze_backend`; git ignores it.
- Planning documents (DEV_PLAN, BUILD_NOTES, TEST_PLAN, the root HASHOUT_LOG) ship in the planning sidecar zip, not in this tree. The living logs `frontend/HASHOUT_LOG.md`, `backend/HASHOUT_LOG.md` and `native/HASHOUT_LOG.md` are in the tree: read them before editing.

## Skills

- `formsculptor-harness` · the orchestrator for all formsculptor work: engine crates, brushes, topology, viewport, file formats, the command registry, the agent substrate, UI shell, engine service, desktop builds and verification. Route through its `SKILL.md` routing table to the specialist `SUB-SKILL.md`.

- `formsculptor-modeling` · operating formsculptor through its MCP tools to sculpt, not changing its code.

Always route work through `.claude/skills/formsculptor-harness/SKILL.md` first: it maps the subsystems, the dependency graph and the laws, and loads the right specialist.

## Sculpting through formsculptor

When the person wants a 3D model made or changed, use the `formsculptor` MCP server's tools with the `formsculptor-modeling` skill, not formsculptor's source code. `.mcp.json` starts it (`./formsculptor mcp`, which starts the engine if it is not running); with the window open, the first change asks the person once there. After adding or changing a command in `native/crates/fsc-commands`, rebuild the engine (`./formsculptor engine build`) and run `./formsculptor agents sync` so the skill's tool reference and its `.agents/skills` mirror follow.
