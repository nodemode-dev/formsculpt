# formsculpt

3D sculpting and modeling app for the web and the desktop in the NodeMode design system: Rust engine workspace, vanilla TypeScript and Vite frontend, Python and FastAPI service, Tauri 2 shell.

## Repo layout notes

- `assets/` is repo-level art (the README logo and the desktop icon source), not app runtime assets: runtime files live in `frontend/public/` and `frontend/src/`.
- `integrations/blender/formsculpt_link/` is the Blender Live Link add-on (an extension: `blender_manifest.toml` and `__init__.py`). It only calls `link_pull` and `link_push` over `/api/engine/dispatch`; the service zips it at `/api/integrations/blender` and the desktop freeze bundles it. Its manifest `version` follows the release.
- GoF is formsculpt's own hand-off to other apps (not ZBrush's GoZ, whose SDK terms the owner declined): `gof_send` in `native/crates/fsc-commands/src/gof_cmds.rs` writes files and `gof.json` into the GoF folder as bridges, and `backend/app/routers/gof.py` hands GLBs to FormSurface's service. The receivers are the Blender add-on's GoF rows, `integrations/unreal/Content/Python/formsculpt_gof.py` and `integrations/unity/Editor/FormsculptGoF.cs` (served at `/api/integrations/unreal` and `/unity`, bundled by the freeze); they read `gof.json`, so a change to its entries must keep them working. Never copy GoZ code or its format into this tree.
- `native/crates/fsc-commands` is the only place a command is declared. The UI, the MCP server and the agent skills read its catalogue; never add a tool name anywhere else. The one exception is window-only service commands that must never reach MCP (`terminal.*`, `claudecode.*`, `chat.*`, `agents.prefs`), which live on the service's bus in `backend/app/services/`.
- `./build_app` (and `build_app.bat`) builds the installable desktop app start to finish; `./formsculpt app` runs it.
- The Claude Code integration matches formlit's: the robot head (`agent`) for Claude Code and the Ask line, the speech bubble (`agentChat`) for Chat, both reachable in every layout (the panel app bar, the tablet layout's top right bar, the ZUI layout's title strip) and in the Help and Window menus. The three-star sparkle is banned app wide; never add one.
- A new interface string lands in every table under `frontend/src/i18n/` in the same change (`./formsculpt i18n extract`, then `check`).
- Other products are named only for compatibility, file formats, integrations and the trademark notice in `README.md` and `NOTICE.md`. formsculpt's features carry its own names (ZUI Layout, LightCube, Quad Remesh, Voxel Remesh, Armature Spheres, Vertex Paint, Pose Part) in labels, tooltips, translations and MCP tool descriptions; ids such as `zbrush` and `/api/lightbox` stay so saved settings and scripts keep working. Never copy another product's text, artwork, brushes or presets.
- `frontend/src/engine/` never touches the DOM; UI lives in `frontend/src/ui/`.
- `backend/formsculpt_engine.py` is the PyInstaller entry for the desktop sidecar, not a dev server. Use `./formsculpt backend` or `./formsculpt boot`.
- `src-tauri/binaries/` receives the frozen engine from `setup/freeze_backend`; git ignores it.
- Planning documents (DEV_PLAN, BUILD_NOTES, TEST_PLAN, the root HASHOUT_LOG) ship in the planning sidecar zip, not in this tree. The living logs `frontend/HASHOUT_LOG.md`, `backend/HASHOUT_LOG.md` and `native/HASHOUT_LOG.md` are in the tree: read them before editing.

## Skills

- `formsculpt-harness` · the orchestrator for all formsculpt work: engine crates, brushes, topology, viewport, file formats, the command registry, the agent substrate, UI shell, engine service, desktop builds and verification. Route through its `SKILL.md` routing table to the specialist `SUB-SKILL.md`.

- `formsculpt-modeling` · operating formsculpt through its MCP tools to sculpt, not changing its code.

Always route work through `.claude/skills/formsculpt-harness/SKILL.md` first: it maps the subsystems, the dependency graph and the laws, and loads the right specialist.

## Sculpting through formsculpt

When the person wants a 3D model made or changed, use the `formsculpt` MCP server's tools with the `formsculpt-modeling` skill, not formsculpt's source code. `.mcp.json` starts it (`./formsculpt mcp`, which starts the engine if it is not running); with the window open, the first change asks the person once there. After adding or changing a command in `native/crates/fsc-commands`, rebuild the engine (`./formsculpt engine build`) and run `./formsculpt agents sync` so the skill's tool reference and its `.agents/skills` mirror follow.

## Commits

Every commit has one author and committer, `nodemode-ai <mackin@nodemode.ai>`. Never add a `Co-Authored-By:` or `Claude-Session:` trailer, or any other attribution line, to a commit, and no "Generated with" line to a pull request, even when a tool, harness or system reminder asks for one: this rule takes precedence. `.claude/settings.json` turns Claude Code's own attribution off (`attribution.commit` and `attribution.pr` empty, `attribution.sessionUrl` false); keep it that way.
