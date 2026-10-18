# backend HASHOUT_LOG

Living log for the engine service side, _v0.1.0. Read the map and the TOC first; entries are anchored by file and symbol, and removed code is quoted in full.

## Org-navi-map

```text
backend/
  formsculptor_engine.py   PyInstaller entry for the desktop sidecar, --selfcheck
  boot_backend             dev server with reload on 127.0.0.1:8196
  oc_install               venv and requirements (macOS reads the unpinned file)
  requirements.txt         pinned
  requirements_macos.txt   unpinned twin
  requirements_desktop*.txt  PyInstaller
  .env.example             FSC_HOST, FSC_PORT, FSC_CORS_ORIGINS
  app/__init__.py          __version__, a version site
  app/main.py              FastAPI app, middleware, routers, static mount
  app/core/config.py       names, ports, dist_dir, is_desktop, cors_origins
  app/core/isolation.py    CrossOriginIsolation ASGI middleware
  app/routers/system.py    /api/health, /api/version, /api/capabilities (engine status from the module)
  app/routers/engine.py    /api/engine/catalogue, /api/engine/dispatch, /api/engine/mesh/{id}
  app/services/engine.py   the one fsc_engine.Engine, status, dispatch, mesh_payload (FSCM binary)
```

## Contents

- [formsculptor_engine.py](#formsculptor_enginepy)
- [app/core/isolation.py](#appcoreisolationpy)
- [app/routers/system.py](#approuterssystempy)

## formsculptor_engine.py

### formsculptor_engine.py · main
- refers-to: formsculptor_engine.py :: main, selfcheck
- kind: rationale
- logged: v0.1.0 · 2026-10-11
Same contract as formvis's sidecar: `FSC_PORT=0` lets the OS pick the port, uvicorn prints `http://127.0.0.1:PORT`, and the Tauri shell reads that line to open the main window. `--selfcheck` prints JSON so CI and the freeze step can prove the frozen binary imports.

## app/core/isolation.py

### app/core/isolation.py · CrossOriginIsolation
- refers-to: app/core/isolation.py :: CrossOriginIsolation
- kind: rationale
- logged: v0.1.0 · 2026-10-11
A raw ASGI middleware rather than a `BaseHTTPMiddleware` so streamed and static responses get the COOP, COEP and CORP headers without buffering. Needed for the WASM engine's threads when the app is served from the engine (`serve` and the desktop window).

## app/routers/system.py

### app/routers/system.py · capabilities
- refers-to: app/routers/system.py :: capabilities
- kind: todo
- logged: v0.1.0 · 2026-10-11
`engine.native` is false and `engine.commands` empty until the PyO3 module lands (DEV_PLAN Phase 2); then they report the module's version and the catalogue.

## app/services/engine.py

### app/services/engine.py · mesh_payload
- refers-to: app/services/engine.py :: MAGIC, mesh_payload
- kind: explainer
- logged: v0.2.0 · 2026-10-11
Binary mesh transfer: `FSCM`, then four little-endian u32 (vertex count, index count, revision, reserved), then positions (f32 x3), normals (f32 x3) and indices (u32). The 20-byte header keeps every block 4-byte aligned so the frontend views it with typed arrays and no copy. One lock serialises calls into the engine; FastAPI runs these sync endpoints in its thread pool.

### app/services/engine.py · import fallback
- refers-to: app/services/engine.py :: fsc_engine import
- kind: rationale
- logged: v0.2.0 · 2026-10-11
The service still boots without the compiled module (a fresh clone before `./formsculptor engine build`): `/api/capabilities` reports `engine.native` false and the engine routes answer 503 with the command to run.

## app/routers/system.py

### app/routers/system.py · capabilities
- refers-to: app/routers/system.py :: capabilities
- kind: removed-code
- logged: v0.2.0 · 2026-10-11

```python
        "engine": {"native": False, "commands": []},
```

Replaced by `engine.status()` now that the PyO3 module exists; closes the 0.1.0 todo.

## app/routers/engine.py

### app/routers/engine.py · stroke WebSocket
- refers-to: app/routers/engine.py :: stroke
- kind: rationale
- logged: v0.3.0 · 2026-10-11
`/api/engine/stroke` carries `begin` (object id and brush), `sample` (ray origin, direction, pressure, seq) and `end` as JSON; each sample is answered with a JSON `{type: sample, seq, hit}` and, when vertices changed, a binary FSCU frame. Engine calls run in the thread pool under the engine lock. A dropped socket closes the open stroke so its edit still reaches the history. The mesh route's header now sets flag 1 and appends the mask (f32 per vertex).

## app/routers/mcp.py

### app/routers/mcp.py · mcp and call_tool
- refers-to: app/routers/mcp.py :: mcp, tools, call_tool, INSTRUCTIONS, STRATEGY
- kind: rationale
- logged: v0.3.0 · 2026-10-11
The MCP server in formvis's protocol shape (stateless Streamable HTTP, 2026-07-28 and 2025-11-25, Origin limited to this computer, cache hints on discover and lists) but with formvis's split removed: there is no tool catalogue in Python. `tools()` maps the engine's catalogue (name, label as title, description, schema, annotations) and `call_tool` runs `engine.dispatch`, so a command added in Rust appears over MCP after an engine rebuild with no Python change. Tools execute in the service's engine session, the one the window shows, so they work with no window open.

## app/services/agents.py

### app/services/agents.py · permit
- refers-to: app/services/agents.py :: permit, seen, window_live, answer
- kind: rationale
- logged: v0.3.0 · 2026-10-11
Consent only matters when someone is looking: a window that polled `/api/mcp/window` in the last 4 s. Then the first mutating call parks a pending request the window shows as a dialog, waits up to 90 s, and proceeds on Allow. Decline refuses for 600 s. No window: the call runs, because the person started the agent themselves. State lives for the service's lifetime; `POST /api/mcp/reset` clears it.

## app/services/engine.py

### app/services/engine.py · epoch
- refers-to: app/services/engine.py :: epoch, _bump, read_only
- kind: rationale
- logged: v0.3.0 · 2026-10-11
A counter that rises on every successful mutating dispatch (any command whose catalogue annotations are not read-only) and on every stroke that changed vertices. The window polls it to refresh after an agent's change.

### app/routers/engine.py · topology during strokes
- refers-to: app/routers/engine.py :: stroke (sample branch)
- kind: explainer
- logged: v0.4.0 · 2026-10-11
`stroke_sample` now returns a third value: true when dynamic topology changed the mesh. The socket then sends no FSCU frame for that sample and marks the JSON reply `topology: true`; the window refetches the whole mesh.

### app/routers/engine.py, app/services/autosave.py · files and autosave
- refers-to: app/routers/engine.py :: import_file, export_file, project_file, autosave_info, autosave_recover; app/services/engine.py :: save_bytes, open_bytes, import_bytes, export_bytes, object_count; app/services/autosave.py
- kind: rationale
- logged: v0.6.0 · 2026-10-11
The window sends files as multipart uploads (a .fsc opens, anything else imports) and receives exports and projects as attachments; uploads stop at 1 GB. The format is validated by the route pattern before the engine runs. Autosave runs in the app lifespan every FSC_AUTOSAVE_SECONDS (60 by default) and writes only when the engine epoch moved since the last save and the scene has objects, to FSC_AUTOSAVE_DIR (default ~/.formsculptor/autosave), through a .part file and an atomic rename. The engine byte helpers were appended to the end of services/engine.py by a shell heredoc (one region, new code only), declared here.

### process · scripted replacement in 0.8.0
- refers-to: frontend/HASHOUT_LOG.md
- kind: process
- logged: v0.8.0 · 2026-10-11
Declared slip: one sentence of the frontend log entry written earlier in this release (not yet committed) was corrected by a Python one-off string swap instead of the Edit tool, after the window's Duplicate stopped offsetting. Every other change in 0.8.0 was a hand edit or a new file.

### app/routers/nodemode.py, app/services/outbound.py, app/routers/engine.py, app/routers/mcp.py · Send to NodeMode, view route, image results
- refers-to: app/routers/nodemode.py :: status, send, Send; app/services/outbound.py :: base, request, multipart, unreachable; app/services/engine.py :: view_png; app/routers/engine.py :: view_image; app/routers/mcp.py :: call_tool; app/main.py
- kind: rationale
- logged: v0.9.0 · 2026-10-11
Send to NodeMode follows formvis's /api/nodemode protocol so NodeMode needs one more receiver, not a new mechanism: the engine (not the page, which cannot post to another app's address) exports the scene as GLB, renders a 768 pixel transparent preview with get_view, uploads both through NodeMode's /api/upload, then posts one announcement to /api/formsculptor/receive with the files' addresses, the model's byte size and which image previews which model. A NodeMode without the receiver (404 or 405) still has the uploads, and the reply says so. outbound.py is formvis's helper with the multipart part taking any content type. The MCP router returns any result carrying png_base64 as an image content block plus the remaining fields as text and structuredContent. Verified against a stand-in NodeMode on the loopback address: with a receiver (announcement as above, the GLB passes the Khronos validator, the PNG is 768 square RGBA), without one (uploads, explanatory reply), nothing listening (502 naming the address and where to change it), a non-http address (422) and an empty scene (refused in the window before sending).

### setup/freeze_backend, formsculptor_engine.py, setup/lib.sh · the frozen engine carries the module
- refers-to: setup/freeze_backend; backend/formsculptor_engine.py :: selfcheck; setup/lib.sh :: fs_venv; setup/engine_build; setup/doctor
- kind: rationale
- logged: v0.9.0 · 2026-10-11
The freeze rebuilds fsc_engine when it is missing or its version differs from the service's, and collects the whole package (the compiled extension included). The self-check now dispatches a real command through a fresh engine session and exits non-zero unless the engine answered and the frontend is bundled, so a sidecar without its engine can no longer pass. Measured here: 23 MB one-file sidecar, 53 commands, frontend inside. fs_venv, engine_build and doctor accept a Windows venv (Scripts/activate), which the desktop CI's Windows leg needs.

### app/services/engine.py, app/routers/engine.py · group and hidden payload, turntable route
- refers-to: app/services/engine.py :: FLAG_GROUPS, FLAG_HIDDEN, mesh_payload; app/routers/engine.py :: turntable
- kind: rationale
- logged: v0.11.0 · 2026-10-11
The mesh payload gains flag 4 (a u32 face group per triangle after the paint) and flag 8 (a u8 hidden flag per triangle after the groups), read from the engine's new faces() call rather than by widening mesh()'s tuple, which every caller unpacks. /api/engine/turntable runs record_turntable into a temporary file and returns it, deleting the file either way.

### app/routers/engine.py, app/services/engine.py · pen tilt on the stroke socket
- refers-to: app/routers/engine.py :: stroke socket sample message (tilt); app/services/engine.py :: stroke_sample (tilt)
- kind: rationale
- logged: v0.16.0 · 2026-10-11
Sample messages may carry tilt (radians, the pen's lean from the screen's up); it defaults to 0, so older clients and agents are unaffected, and is passed to the engine's stroke_sample, which turns the brush alpha by it.

### app/services/scripting.py, app/routers/plugins.py, setup/run_script.py, plugins/ · Python scripting and plugins
- refers-to: app/services/scripting.py :: Fsc (call, scene, active), ScriptError, plugin_dirs, list_plugins, run_file; app/routers/plugins.py :: plugins, run; app/main.py :: plugins router; setup/run_script.py :: main, http_dispatch, engine_running; formsculptor :: script; plugins/rock.py, plugins/pedestal.py
- kind: rationale
- logged: v0.18.0 · 2026-10-11
Scripts and plugins are plain Python with one handle, fsc, whose call(command, **args) runs any registry command by its MCP name and raises on refusal; the command is positional-only because several commands take a name argument (the first version broke on add_primitive's). ./formsculptor script FILE [JSON] runs a file against the running engine over HTTP, so the window follows along, or against an engine in its own process when none runs; paths are taken from where the command was typed (OLDPWD, as the launcher changes directory). Plugins are files in plugins/ or ~/.formsculptor/plugins (FSC_PLUGIN_DIR overrides) that define run(fsc, args); their docstring's first line is the title. The service lists them and runs one in its own process through the same dispatch the window uses, reporting a plugin's own exception as a message instead of failing the request. The service listens on the loopback address only and runs only the person's own files. Two examples ship: a rock built from a sphere with a lattice and two noise layers, and a display pedestal under the active object. No Windows launcher entry yet.

### app/routers/engine.py · the animation route
- refers-to: app/routers/engine.py :: animation
- kind: rationale
- logged: v0.21.0 · 2026-10-12
GET /api/engine/animation takes the keyframes as a JSON query value (at most 20,000 characters, which holds 64 keys), runs record_animation into a temporary file and returns the APNG as a download, like the turntable route; bad JSON is a 400 with the parse error. GET /api/engine/history_movie does the same for record_history (fps, size, turn).

### tests/, requirements_test.txt · the service test suite
- refers-to: tests/conftest.py :: client, call; tests/test_service.py; requirements_test.txt
- kind: rationale
- logged: v0.22.0 · 2026-10-12
pytest on the FastAPI app through its test client, with the built engine module (the suite skips, saying why, when the module is missing). Each test starts from an empty scene, since the service keeps one engine session. Covered: version and health, the catalogue against the engine, dispatch errors as data, mesh buffers against the summary and a 404 for a missing object, the view PNG, the turntable, animation (and a 400 for bad JSON keys) and history movie routes, export and upload import, MCP initialize, tools/list against the catalogue, a tool call, an image reply for get_view, an unknown tool's error and a 403 for a foreign origin, and the plugins list and run. 10 tests in 0.3 s. On the first run the plugin test was wrong, not the service: it expected the pedestal to refuse after the rock, but the rock leaves an active object to stand it on; the test now runs the pedestal first. pytest 9.1.1 and httpx 0.28.1 are pinned in requirements_test.txt; Starlette warns that httpx is deprecated for its test client, so the runner hides warnings until that moves.

### services/scripting.py, routers/plugins.py, routers/engine.py, plugins/ · plugin fields and tools, the bridge route
- refers-to: app/services/scripting.py :: _describe (PARAMS, CLICK), _param; app/routers/plugins.py :: plugins; app/routers/engine.py :: BridgeOut, bridge; plugins/rock.py, plugins/pedestal.py :: PARAMS; plugins/rivets.py :: CLICK, PARAMS, run; tests/test_service.py; tests/conftest.py :: call
- kind: rationale
- logged: v0.23.0 · 2026-10-12
A plugin may declare PARAMS (fields: number, integer, boolean, choice or text, with label, default, min, max, step and options) and CLICK = True (a surface tool that receives the clicked point, normal and object id). Both are read with ast.literal_eval from the file's top level, so listing plugins never runs them; anything that is not a literal is ignored. The list route passes them to the window, which draws the fields. A new example, rivets, sets a ring of rivets around a clicked point through pick and insert_along. POST /api/engine/bridge sends an object to <name>-<id>.<format> in FSC_BRIDGE_DIR or ~/.formsculptor/bridge and returns the path to open in the other app. The suite's call helper had the same keyword collision Fsc.call had at 0.18.0 (a command's own name argument); it is positional-only now. 12 service tests.

### routers/engine.py · FBX through the service
- refers-to: app/routers/engine.py :: export_file (fbx), BridgeOut (fbx)
- kind: rationale
- logged: v0.24.0 · 2026-10-12
The export route and the bridge take fbx; upload import needed no change since the engine picks the format from the file name. A test exports a torus as FBX, uploads it back and finds both with 2,048 vertices. 13 service tests.

### services/engine.py, routers/engine.py · the smooth preview mesh
- refers-to: app/services/engine.py :: mesh_payload (smooth); app/routers/engine.py :: mesh (smooth query); tests/test_service.py :: test_smooth_preview_subdivides_for_display_only
- kind: rationale
- logged: v0.25.0 · 2026-10-12
GET /api/engine/mesh/{id}?smooth=1..3 returns the engine's smooth_preview of the object in the same FSCM layout with only the paint flag (no mask, groups or hidden faces, which belong to the sculpted mesh) and the object's own revision; smooth above 3 is a 422 from the query validation. The test checks 4 to the power of the level times the triangles, the unchanged revision, the plain mesh byte-identical afterwards and a 404 for a missing object. 14 service tests.

### tests/test_service.py · seven-float paint and old projects
- refers-to: tests/test_service.py :: _glb_json, test_paint_has_seven_floats_and_old_projects_open_opaque
- kind: rationale
- logged: v0.26.0 · 2026-10-12
No service code changed for the paint channels: the mesh payload carries whatever the engine's paint bytes are. The test reads opacity 0.5 and emissive 0.25 as the sixth and seventh floats of the first vertex in the FSCM payload, rewrites a saved project's paint to the five floats per vertex projects held before 0.26.0, uploads it, and finds its GLB without alphaMode or emissiveFactor, so old paint opens opaque without glow. 15 service tests.

### routers/engine.py · USD export
- refers-to: app/routers/engine.py :: export_file (usda); tests/test_service.py :: test_usda_export_route
- kind: rationale
- logged: v0.27.0 · 2026-10-12
The export route takes usda (served as model/vnd.usda); upload import needed no change. The test exports a cone as USD, uploads it back and finds two objects. 16 service tests.

### services/engine.py, routers/engine.py · the live boolean preview route
- refers-to: app/services/engine.py :: boolean_preview; app/routers/engine.py :: boolean_preview; tests/test_service.py :: test_boolean_preview_draws_without_changing_the_scene
- kind: rationale
- logged: v0.28.0 · 2026-10-12
GET /api/engine/boolean_preview?a=&b=&op= returns the exact result as an FSCM payload (revision 0, a zero mask) without touching the scene or the history: 404 for a missing object, 422 for an unknown op, 400 with the engine's reason when the input is not closed. 17 service tests.

### routers/engine.py · path render and the Substance package
- refers-to: app/routers/engine.py :: view_image (render, samples, bounces), SubstanceOut, substance; tests/test_service.py :: test_path_render_and_substance_routes
- kind: rationale
- logged: v0.29.0 · 2026-10-12
GET /api/engine/view takes render=path with samples (1 to 1,024) and bounces (1 to 8). POST /api/engine/substance writes send_to_substance's package to FSC_SUBSTANCE_DIR or ~/.formsculptor/substance/<object name>; 404 for a missing object, the engine's no_uvs refusal as data. 18 service tests.

### routers/engine.py · render passes and the Substance sync
- refers-to: app/routers/engine.py :: view_image (pass, aperture), substance_sync; tests/test_service.py :: test_path_render_and_substance_routes
- kind: rationale
- logged: v0.30.0 · 2026-10-12
GET /api/engine/view takes pass (beauty, depth, normal, ao; anything else is 422) and aperture (0 to 1) with render=path; they are not sent for the preview render. POST /api/engine/substance/sync reads Painter's export from the same package folder the send route writes (substance_sync); a folder without <name>_BaseColor.png answers io_failed as data. 18 service tests.

### services/commands.py and services/events.py · the window command bus
- refers-to: app/services/commands.py, app/services/events.py, app/routers/commands.py, app/routers/events.py
- kind: rationale
- logged: v0.32.0 · 2026-10-16
Ported from FormLit (same owner, Apache-2.0), trimmed: no revision counter, no batch route. It carries window-only commands (terminal.*, claudecode.*, chat.*, agents.prefs) that must never reach MCP; every sculpting command stays in the Rust registry, which is still the only place an agent tool is declared. /api/events is SSE with a hello frame, a 15 s ping and retry 1500; the isolation middleware is pure ASGI, so the stream is not buffered.

### services/terminal.py and routers/terminal_socket.py · PTY sessions
- refers-to: app/services/terminal.py, app/services/cmd_terminal.py, app/routers/terminal_socket.py
- kind: gotcha
- logged: v0.32.0 · 2026-10-16
ptyprocess on Unix, pywinpty on Windows, 256 KB scrollback replayed to a late attacher. Browsers apply no CORS to WebSockets, so the socket refuses any origin that is not localhost, 127.0.0.1, ::1, tauri.localhost or tauri:// (close 4403); ids are 48-bit random and only listed through terminal.list. A gen counter keeps a restarted tab from receiving the old shell's last bytes. The exit frame is the literal text {"type": "exit"} (json.dumps spacing), which the window compares as a string. Checked live: stty size followed a resize frame, FSC_TERMINAL reached the shell, a foreign origin was refused.

### services/claude_code.py and services/cmd_chat.py · Claude Code in the window
- refers-to: app/services/claude_code.py, app/services/cmd_chat.py, app/services/agents.py :: INAPP_TOKEN, app/routers/mcp.py :: call_tool
- kind: rationale
- logged: v0.32.0 · 2026-10-16
The dock tab runs the person's own claude through the login shell (exec, so Stop reaches it, and npm-installed claude finds node) with --mcp-config pointing at this service's /mcp, --allowedTools mcp__formsculptor and a sculpting note. The config carries x-formsculptor-inapp with a token made per engine run, so the in-app agent skips the outside-agent prompt; a decline in the last ten minutes still refuses it. CLAUDE* variables are dropped from children except the sign-in ones, or Claude Code thinks it is nested. Chat runs claude -p with stream-json once per message and --resume for continuity; images from tool results are written to <data>/Chat and served by name only from that folder. The frozen engine now chooses its own port before uvicorn starts (FSC_PORT=0 used to reach uvicorn), so the MCP URL handed to Claude Code is real. Data folder: FSC_DATA_DIR, default ~/Formsculptor (Agent, Chat, preferences.json).

### routers/engine.py · a gone object's mesh is 204, the history movie's camera
- refers-to: app/routers/engine.py :: mesh (`raise HTTPException(status_code=404, detail=f"no object {object_id}")` became a 204 with X-Formsculptor-Missing), history_movie (camera); tests/test_service.py (the two 9999 assertions)
- kind: fix
- logged: v0.73.0 · 2026-10-17
The window lists the scene and then fetches each new mesh; when an agent clears or replaces the scene in between (or a step is undone), the fetch named an object that was gone and the 404 was logged by the browser as a console error, though the window already treated it as nothing to draw. This surfaced as six console 404s in window checks that start by replacing the scene the previous check left (0.72.0's log put them down to the build swapping files; that was wrong). A gone object is now an empty 204, which the window reads as nothing to draw; the existing service test's two assertions moved from 404 to 204 with it. The history movie route passes camera (fixed or steps) through to record_history.

### core/quiet_log.py · successful requests no longer printed
- refers-to: app/core/quiet_log.py (new) :: install, _FailuresOnly; app/main.py :: lifespan (quiet_log.install), the config import
- kind: fix
- logged: v0.76.0 · 2026-10-17
The owner's terminal filled with access lines: the window asks for agent news about once a second (/api/mcp/window), for the version every few seconds, and for the scene after each change, and uvicorn printed one line per request. A filter on the uvicorn.access logger now drops records whose status is under 400, so failed requests, errors and startup lines still print. It is installed in the app's lifespan rather than at import, because uvicorn sets up its loggers after importing the app and a filter added earlier could be replaced; living in the app, it covers the dev server (backend/boot_backend), ./formsculptor serve and the frozen desktop engine alike. FSC_ACCESS_LOG=1 prints every request again. The polling itself is unchanged: it is how the window learns of an agent's changes and of the consent prompt. Checked against a booted service with the window loaded: no access lines for the polls, scene requests and dispatches; a request to a missing route still printed its 404.

### routers/engine.py · denoise on the view route
- refers-to: app/routers/engine.py :: view_image (denoise)
- kind: rationale
- logged: v0.84.0 · 2026-10-18
The path render's new denoise argument is passed through /api/engine/view as a boolean query parameter (false by default) for render=path only, so the window's Path Render can ask for it. Removed: `args.update({"pass": pass_, "aperture": aperture})` (now with denoise).
