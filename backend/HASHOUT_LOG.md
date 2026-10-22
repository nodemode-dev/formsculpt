# backend HASHOUT_LOG

Living log for the engine service side, _v0.1.0. Read the map and the TOC first; entries are anchored by file and symbol, and removed code is quoted in full.

## Org-navi-map

```text
backend/
  formsculpt_engine.py   PyInstaller entry for the desktop sidecar, --selfcheck
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
  app/routers/system.py    /api/health, /api/version, /api/capabilities (engine status from the module), /api/integrations/blender
  app/routers/lightbox.py  /api/lightbox/projects: LightBox's Project shelf (list, save with a picture, open, picture)
  app/routers/engine.py    /api/engine/catalogue, /api/engine/dispatch, /api/engine/mesh/{id}
  app/services/engine.py   the one fsc_engine.Engine, status, dispatch, mesh_payload (FSCM binary)
```

## Contents

- [formsculpt_engine.py](#formsculpt_enginepy)
- [app/core/isolation.py](#appcoreisolationpy)
- [app/routers/system.py](#approuterssystempy)

## formsculpt_engine.py

### formsculpt_engine.py · main
- refers-to: formsculpt_engine.py :: main, selfcheck
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
The service still boots without the compiled module (a fresh clone before `./formsculpt engine build`): `/api/capabilities` reports `engine.native` false and the engine routes answer 503 with the command to run.

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
The window sends files as multipart uploads (a .fsc opens, anything else imports) and receives exports and projects as attachments; uploads stop at 1 GB. The format is validated by the route pattern before the engine runs. Autosave runs in the app lifespan every FSC_AUTOSAVE_SECONDS (60 by default) and writes only when the engine epoch moved since the last save and the scene has objects, to FSC_AUTOSAVE_DIR (default ~/.formsculpt/autosave), through a .part file and an atomic rename. The engine byte helpers were appended to the end of services/engine.py by a shell heredoc (one region, new code only), declared here.

### process · scripted replacement in 0.8.0
- refers-to: frontend/HASHOUT_LOG.md
- kind: process
- logged: v0.8.0 · 2026-10-11
Declared slip: one sentence of the frontend log entry written earlier in this release (not yet committed) was corrected by a Python one-off string swap instead of the Edit tool, after the window's Duplicate stopped offsetting. Every other change in 0.8.0 was a hand edit or a new file.

### app/routers/nodemode.py, app/services/outbound.py, app/routers/engine.py, app/routers/mcp.py · Send to NodeMode, view route, image results
- refers-to: app/routers/nodemode.py :: status, send, Send; app/services/outbound.py :: base, request, multipart, unreachable; app/services/engine.py :: view_png; app/routers/engine.py :: view_image; app/routers/mcp.py :: call_tool; app/main.py
- kind: rationale
- logged: v0.9.0 · 2026-10-11
Send to NodeMode follows formvis's /api/nodemode protocol so NodeMode needs one more receiver, not a new mechanism: the engine (not the page, which cannot post to another app's address) exports the scene as GLB, renders a 768 pixel transparent preview with get_view, uploads both through NodeMode's /api/upload, then posts one announcement to /api/formsculpt/receive with the files' addresses, the model's byte size and which image previews which model. A NodeMode without the receiver (404 or 405) still has the uploads, and the reply says so. outbound.py is formvis's helper with the multipart part taking any content type. The MCP router returns any result carrying png_base64 as an image content block plus the remaining fields as text and structuredContent. Verified against a stand-in NodeMode on the loopback address: with a receiver (announcement as above, the GLB passes the Khronos validator, the PNG is 768 square RGBA), without one (uploads, explanatory reply), nothing listening (502 naming the address and where to change it), a non-http address (422) and an empty scene (refused in the window before sending).

### setup/freeze_backend, formsculpt_engine.py, setup/lib.sh · the frozen engine carries the module
- refers-to: setup/freeze_backend; backend/formsculpt_engine.py :: selfcheck; setup/lib.sh :: fs_venv; setup/engine_build; setup/doctor
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
- refers-to: app/services/scripting.py :: Fsc (call, scene, active), ScriptError, plugin_dirs, list_plugins, run_file; app/routers/plugins.py :: plugins, run; app/main.py :: plugins router; setup/run_script.py :: main, http_dispatch, engine_running; formsculpt :: script; plugins/rock.py, plugins/pedestal.py
- kind: rationale
- logged: v0.18.0 · 2026-10-11
Scripts and plugins are plain Python with one handle, fsc, whose call(command, **args) runs any registry command by its MCP name and raises on refusal; the command is positional-only because several commands take a name argument (the first version broke on add_primitive's). ./formsculpt script FILE [JSON] runs a file against the running engine over HTTP, so the window follows along, or against an engine in its own process when none runs; paths are taken from where the command was typed (OLDPWD, as the launcher changes directory). Plugins are files in plugins/ or ~/.formsculpt/plugins (FSC_PLUGIN_DIR overrides) that define run(fsc, args); their docstring's first line is the title. The service lists them and runs one in its own process through the same dispatch the window uses, reporting a plugin's own exception as a message instead of failing the request. The service listens on the loopback address only and runs only the person's own files. Two examples ship: a rock built from a sphere with a lattice and two noise layers, and a display pedestal under the active object. No Windows launcher entry yet.

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
A plugin may declare PARAMS (fields: number, integer, boolean, choice or text, with label, default, min, max, step and options) and CLICK = True (a surface tool that receives the clicked point, normal and object id). Both are read with ast.literal_eval from the file's top level, so listing plugins never runs them; anything that is not a literal is ignored. The list route passes them to the window, which draws the fields. A new example, rivets, sets a ring of rivets around a clicked point through pick and insert_along. POST /api/engine/bridge sends an object to <name>-<id>.<format> in FSC_BRIDGE_DIR or ~/.formsculpt/bridge and returns the path to open in the other app. The suite's call helper had the same keyword collision Fsc.call had at 0.18.0 (a command's own name argument); it is positional-only now. 12 service tests.

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
GET /api/engine/view takes render=path with samples (1 to 1,024) and bounces (1 to 8). POST /api/engine/substance writes send_to_substance's package to FSC_SUBSTANCE_DIR or ~/.formsculpt/substance/<object name>; 404 for a missing object, the engine's no_uvs refusal as data. 18 service tests.

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
The dock tab runs the person's own claude through the login shell (exec, so Stop reaches it, and npm-installed claude finds node) with --mcp-config pointing at this service's /mcp, --allowedTools mcp__formsculpt and a sculpting note. The config carries x-formsculpt-inapp with a token made per engine run, so the in-app agent skips the outside-agent prompt; a decline in the last ten minutes still refuses it. CLAUDE* variables are dropped from children except the sign-in ones, or Claude Code thinks it is nested. Chat runs claude -p with stream-json once per message and --resume for continuity; images from tool results are written to <data>/Chat and served by name only from that folder. The frozen engine now chooses its own port before uvicorn starts (FSC_PORT=0 used to reach uvicorn), so the MCP URL handed to Claude Code is real. Data folder: FSC_DATA_DIR, default ~/Formsculpt (Agent, Chat, preferences.json).

### routers/engine.py · a gone object's mesh is 204, the history movie's camera
- refers-to: app/routers/engine.py :: mesh (`raise HTTPException(status_code=404, detail=f"no object {object_id}")` became a 204 with X-Formsculpt-Missing), history_movie (camera); tests/test_service.py (the two 9999 assertions)
- kind: fix
- logged: v0.73.0 · 2026-10-17
The window lists the scene and then fetches each new mesh; when an agent clears or replaces the scene in between (or a step is undone), the fetch named an object that was gone and the 404 was logged by the browser as a console error, though the window already treated it as nothing to draw. This surfaced as six console 404s in window checks that start by replacing the scene the previous check left (0.72.0's log put them down to the build swapping files; that was wrong). A gone object is now an empty 204, which the window reads as nothing to draw; the existing service test's two assertions moved from 404 to 204 with it. The history movie route passes camera (fixed or steps) through to record_history.

### core/quiet_log.py · successful requests no longer printed
- refers-to: app/core/quiet_log.py (new) :: install, _FailuresOnly; app/main.py :: lifespan (quiet_log.install), the config import
- kind: fix
- logged: v0.76.0 · 2026-10-17
The owner's terminal filled with access lines: the window asks for agent news about once a second (/api/mcp/window), for the version every few seconds, and for the scene after each change, and uvicorn printed one line per request. A filter on the uvicorn.access logger now drops records whose status is under 400, so failed requests, errors and startup lines still print. It is installed in the app's lifespan rather than at import, because uvicorn sets up its loggers after importing the app and a filter added earlier could be replaced; living in the app, it covers the dev server (backend/boot_backend), ./formsculpt serve and the frozen desktop engine alike. FSC_ACCESS_LOG=1 prints every request again. The polling itself is unchanged: it is how the window learns of an agent's changes and of the consent prompt. Checked against a booted service with the window loaded: no access lines for the polls, scene requests and dispatches; a request to a missing route still printed its 404.

### routers/engine.py · denoise on the view route
- refers-to: app/routers/engine.py :: view_image (denoise)
- kind: rationale
- logged: v0.84.0 · 2026-10-18
The path render's new denoise argument is passed through /api/engine/view as a boolean query parameter (false by default) for render=path only, so the window's Path Render can ask for it. Removed: `args.update({"pass": pass_, "aperture": aperture})` (now with denoise).

### routers/engine.py · 4K on the view route
- refers-to: app/routers/engine.py :: view_image (width, height)
- kind: rationale
- logged: v0.90.0 · 2026-10-18
The engine renders up to 4096 by 4096 now (native side log, 0.90.0), so the view route's width and height go to le=4096 for the window's 4K Render size. Removed: `width: int = Query(640, ge=64, le=2048), height: int = Query(640, ge=64, le=2048)`.

### routers/system.py · the Blender add-on download
- refers-to: app/routers/system.py :: blender_addon (new, GET /api/integrations/blender); app/core/config.py :: blender_addon_dir (new); formsculpt_engine.py :: main (FSC_INTEGRATIONS); setup/freeze_backend (the add-on folder as data)
- kind: rationale
- logged: v0.91.0 · 2026-10-18
The live link's Blender half (native side log, 0.91.0) lives in integrations/blender/formsculpt_link. The window's File panel offers it, so the service zips that folder on request (the manifest and __init__.py at the zip's root, the layout Blender's own extension build writes, skipping __pycache__) as formsculpt_link-<version>.zip, or answers 404 when the folder is not in the build. The frozen engine carries the folder as PyInstaller data under integrations/ and points FSC_INTEGRATIONS at it, as it does FSC_DIST for the frontend. The add-on itself talks to /api/engine/dispatch, which every write already passes through, so a Blender push bumps the epoch and the window refreshes as it does for an agent. Checked: the served zip installed into bpy 5.2.2 and ran the whole headless link check; the window downloaded it.

### routers/engine.py · import and export options on the file routes
- refers-to: app/routers/engine.py :: import_file, export_file (scale, up_axis, merge, triangulate); app/services/engine.py :: import_bytes, export_bytes
- kind: rationale
- logged: v0.92.0 · 2026-10-18
The engine's import and export options (native side log, 0.92.0) reach the window's file routes as query parameters: POST /api/engine/import takes scale (gt 0, le 100000), up_axis (y or z) and merge; GET /api/engine/export takes scale, up_axis and triangulate. They default to the old behaviour, so the NodeMode route's export_bytes("scene.glb", None) and every other caller is unchanged; the service wrappers pass them on to the module's new keyword arguments. Removed: `async def import_file(file: UploadFile) -> dict:`, `return await run_in_threadpool(engine.import_bytes, name, data)`, `async def export_file(format: str = Query(...), ids: str | None = None) -> Response:` and `data = await run_in_threadpool(engine.export_bytes, f"export.{format}", chosen)`, and in services/engine.py `def import_bytes(file_name: str, data: bytes) -> dict:` and `def export_bytes(file_name: str, ids: list[int] | None) -> bytes:` with their two module calls. Checked: export z at x2 gave a 4 by 4 by 12 OBJ, up_axis q answered 422, an upload at z and x0.5 stood 1 by 3 by 1.

### routers/lightbox.py · LightBox's Project shelf
- refers-to: app/routers/lightbox.py (new) :: projects, preview, save, open_project, _save, _folder, _checked, _guard; app/main.py (the router under /api)
- kind: rationale
- logged: v0.94.0 · 2026-10-18
The ZBrush layout's LightBox (frontend side log, 0.94.0) has a Project shelf, as ZBrush's does (the Starting Guide opens DefaultSphere.ZBP from LightBox's Project section with a double click). Projects live in the app's folder, data_dir()/LightBox/Project (FSC_DATA_DIR or ~/Formsculpt), each as name.fsc with name.png beside it. GET /api/lightbox/projects lists them newest first with bytes, saved time and whether a picture is kept; POST /api/lightbox/projects/{name} saves the scene (engine.save_bytes) and renders a 160 by 160 clay picture on a transparent background through get_view (no picture for an empty scene, and a stale one is removed); POST /api/lightbox/projects/{name}/open opens it through engine.open_bytes, which bumps the epoch like any other change; GET .../{name}/preview serves the picture. Names are 1 to 64 letters, digits, spaces, dots, dashes or underscores, starting with a letter or digit and not ending in a dot, so a name cannot leave the folder; anything else is a 400. The routes refuse with 503 when the engine module is not built, as the engine routes do. The browser engine has no service, so the window says the shelf needs it.
Checked: an empty list; a torus saved (23,792 bytes, picture kept, the picture looked at); listed; a new scene then open brought the torus back; "..%2Fx" never reached the route (405) and a missing name answered 404.

### routers/gof.py · GoF's hop to FormSurface
- refers-to: app/routers/gof.py (new) :: status, formsurface, _hand, _health, _call, _service, _folder, Hand; app/main.py (the router under /api)
- kind: rationale
- logged: v0.99.0 · 2026-10-19
The engine's gof_send (native side log, 0.99.0) writes GoF files; this router does the part that needs another app's service. At import it sets FORMSCULPT_GOF to data_dir()/GoF when unset, so the in-process engine and this router use one folder (with no FSC_DATA_DIR both are ~/Formsculpt/GoF, the engine's own default, so the MCP server agrees too). GET /api/gof gives the folder, gof.json's objects and FormSurface's state (version and whether its editor is linked, or null when it is not running). POST /api/gof/formsurface {file} hands a GLB from the GoF folder to FormSurface, from FormSurface 0.56.1's own code as the owner supplied it: its service (FORMSURFACE_SERVICE, default http://127.0.0.1:8766, as its MCP server uses) reports its workspace at /api/health (data, editor_linked); its load_mesh reads the workspace's meshes folder; its /api/agent runs a command in the open editor ({command, args, timeout}) and refuses when no editor is linked. So the file is copied into data/meshes and, when linked, load_mesh runs with reproject true (FormSurface's own option that keeps its painting on the surface when an edited mesh comes back). Only a bare .glb name in the GoF folder is taken (404 otherwise); 503 when FormSurface is not running, 502 when it refuses or does not answer. Requests go straight to the loopback service, not through a proxy; urllib is used, since httpx is only a test requirement here. Nothing in FormSurface was changed.
Checked against FormSurface 0.56.1's service run from the owner's zip on its own port and workspace: a torus sent for FormSurface was copied into its meshes folder and served back by FormSurface's /api/meshes (200, 99,308 bytes), with loaded false and the Agent link note, since no FormSurface editor was linked (its editor needs WebGPU, which this container lacks, so the linked load was not run; FormSurface's /api/agent answered load_mesh with its own "No FormSurface editor is linked" refusal); ../etc/passwd was refused with 404; /api/gof listed the file and FormSurface 0.56.1 unlinked.

### integrations/blender/formsculpt_link · GoF's Blender receiver
- refers-to: integrations/blender/formsculpt_link/__init__.py :: gof_folder, gof_entries, gof_import, gof_check, gof_return, _gof_tick, _set_gof_watch (new), FormsculptPreferences (gof_folder, gof_watch), FORMSCULPT_OT_gof_import, FORMSCULPT_OT_gof_return (new), FORMSCULPT_PT_link (the GoF rows), unregister (both timers); integrations/blender/README.md (GoF)
- kind: rationale
- logged: v1.0.0 · 2026-10-19
The first GoF receiver (DEV_PLAN 11.10's second box), in the add-on this router ships, since Blender is the one target app that runs here. The GoF folder is the preference, else what this service's GET /api/gof reports, else ~/Formsculpt/GoF. Import from GoF reads gof.json and brings in every GLB entry for Blender (or any) that is new or sent again since the object holding it was imported (each object keeps gof_file and gof_sent): through Blender's own glTF importer, with merge_vertices so the shared vertices formsculpt wrote stay shared; an object already brought from that file keeps its name and transform and takes the new mesh. Watch GoF is a one-second timer that does the same whenever gof.json's modified time changes. Send back to GoF writes the selected GoF objects over their files with Blender's own glTF exporter (export_format GLB, use_selection, export_yup, export_apply, no materials), which formsculpt's bridge sync reads into the same object (native side log, 0.99.0). The operators' arguments were read from bpy 5.2.2's own RNA (import_scene.gltf "Load a glTF 2.0 file" with filepath and merge_vertices; export_scene.gltf with filepath, export_format, use_selection, export_yup "+Y up", export_apply, export_materials), Blender's own description of its own API, one publisher, and every one was run below.
Checked headless with bpy 5.2.2 and the add-on installed from this service's zip, against the running service: the folder came from /api/gof and matched gof_send's; a box sent from formsculpt 1 by 3 by 0.5 (y up) came in 1 by 0.5 by 3 (Blender z up) with its 1538 vertices; a second check imported nothing; a re-send at twice the width updated the same object (one GoF object, 2 by 0.5 by 3); Blender's edit (+1 in z) sent back moved formsculpt's y bounds from -1.5..1.5 to -0.5..2.5 with 1538 vertices; with Watch GoF on, the timer's body imported the next send, and a Unity send was left alone. A headless bpy has no event loop, so the timer was called by hand, as for the live link at 0.91.0. bpy's pip build lacks Draco and MeshOptimizer and says so on export; GoF uses neither.

### routers/gof.py · FormSurface not running is an answer, not a 503
- refers-to: app/routers/gof.py :: _hand (running in every reply)
- kind: rationale
- logged: v1.0.0 · 2026-10-19
1.0.0's window suite ran with FormSurface's service stopped and the GoF check failed: the 503 for "not running" showed in the browser console as a failed resource, and the long note that carried it wrapped (CLS 0.0041). Not running is an ordinary state (the file waits in the GoF folder), so the route now answers 200 with copied null, loaded false and running false, and every reply carries running; a missing file (404) and FormSurface refusing or not answering (502) stay errors. Removed: `raise HTTPException(status_code=503, detail="FormSurface is not running; start it with ./formsurface")`.
Checked with gof.mjs both ways: FormSurface stopped (a short note, CLS 0, no errors) and its 0.56.1 service running (copied, the Agent link note, CLS 0, no errors).

### routers/gof.py · FormSurface's paint back (POST /api/gof/formsurface/back)
- refers-to: app/routers/gof.py :: Back, _stem, _back, formsurface_back (new); the engine import
- kind: rationale
- logged: v1.1.0 · 2026-10-19
DEV_PLAN 11.10's third box, its first part: FormSurface's painted maps come back into the object's paint. Read in FormSurface 0.56.1's own code (the owner's zip, one publisher): the editor exports maps to its service, which writes them to workspace/exports/<stem>/ as <stem>_<map>, the stem being the mesh file's name without its extension (main.ts export, api.ts saveMaps); its Blender template's maps are BaseColor, Metallic, Roughness, Normal and Height (export-templates.ts), with no texture-set prefix when there is one set (main.ts named); its mesh loader (engine/meshes.ts, three.js loaders for every format) stores each UV as u, 1 - v, and its painting shader puts the stored v down the texture (shaders.rs, clip y 1 - 2v), so a map's row is (1 - v) times its height in the file's own v. That is the row substance_sync samples and the names it reads (<stem>_BaseColor.png, _Roughness, _Metallic), so the route finds the object's name, makes the same stem as gof_send (letters and digits, anything else an underscore), and runs substance_sync on FormSurface's export folder, found from its /api/health. FormSurface not running and nothing exported yet are ordinary answers (running false, or painted false with the file it looked for), not error codes, so the window shows a note instead of a console error; a missing object is 404 and an engine refusal 400.
Checked against FormSurface 0.56.1's own service with the engine: a plane named Paint-test, box-projected (one chart), sent with GoF for FormSurface (Paint_test.glb copied into its meshes folder); before an export the route answered that Paint_test_BaseColor.png was missing; a four-quadrant PNG put where FormSurface's Blender template writes it came back, and every one of the 225 vertices away from the quadrant edges took the quadrant at (u, row 1 - v), read from an OBJ export's UVs and a PLY export's colours (compared in linear values, since paint is linear). FormSurface's editor itself (WebGPU) cannot run here, so a real FormSurface export was not made: the orientation rests on its code, flagged.

### GoF receivers for Unreal Engine and Unity (integrations/unreal, integrations/unity; GET /api/integrations/{app})
- refers-to: integrations/unreal/Content/Python/formsculpt_gof.py (new) :: gof_folder, entries, import_entry, check, _tick, start, stop; integrations/unreal/Content/Python/init_unreal.py (new); integrations/unreal/README.md (new); integrations/unity/Editor/FormsculptGoF.cs (new) :: FormsculptGoF (Folder, ImportNow, ToggleWatch, SetFolder, Tick, Check); integrations/unity/README.md (new); app/routers/system.py :: gof_receiver (new); app/core/config.py :: gof_receiver_dir (new); setup/freeze_backend (both folders bundled)
- kind: rationale
- logged: v1.3.0 · 2026-10-19
DEV_PLAN 11.10's second box, its last two receivers; neither editor runs in this container, so both ship flagged as not yet run inside the app. Unreal: a Python module for the project's Content/Python, started by an init_unreal.py there (Epic's "Scripting the Unreal Editor Using Python": the editor runs init_unreal.py from the paths it uses, Content/Python among them). It ticks through unreal.register_slate_post_tick_callback, reads gof.json once a second when it changes, and imports entries for unreal or any in GLB through an AssetImportTask (filename, destination_path /Game/GoF, destination_name the file's stem, replace_existing, automated, save) and AssetToolsHelpers.get_asset_tools().import_asset_tasks, every name and property from Epic's Python API reference; what it imported is remembered in the GoF folder's .seen/unreal.json by sent time. Unity: an editor script, [InitializeOnLoad] with EditorApplication.update and timeSinceStartup for a one-second watch, EditorPrefs for the folder, the watch switch and each file's sent time, JsonUtility.FromJson on gof.json, the file copied under Application.dataPath/GoF and AssetDatabase.ImportAsset("Assets/GoF/...", ImportAssetOptions.ForceUpdate), Tools > formsculpt menu items with Menu.SetChecked (set again each second since the check clears on restart) and EditorUtility.OpenFolderPanel, all from Unity's Scripting Reference; it takes FBX for unity and OBJ for any. The service zips each folder at /api/integrations/unreal and /unity as it does Blender's (the Blender route is declared first, so it still answers), and the desktop freeze bundles both.
Checked: the Unreal module against a stand-in unreal module with the documented names and formsculpt's real GoF folder (a sphere sent for Unreal, Unity and a torus for Blender and any): one import task with exactly those properties for Sphere.glb, nothing on the next tick, a re-send imported again over it, stop unregistered the tick. The Unity script compiled with the .NET 8 SDK at C# 9 against stand-ins with Unity's documented signatures (no errors, no warnings once CS0649 is silenced around the JSON classes) and its Check on the same folder copied and imported Sphere.fbx and Torus.obj with ForceUpdate, skipped the GLBs and imported nothing the second time. The routes served both zips, Blender's still, and 404 for anything else.

### GET /api/engine/triplanar_preview
- refers-to: app/routers/engine.py :: triplanar_preview (new); app/services/engine.py :: triplanar_preview (new)
- kind: rationale
- logged: v1.9.0 · 2026-10-19
Live Triplanar's route, beside boolean_preview: the solid a triplanar's masks make now as FSCM (native side log, 1.9.0), resolution 16 to 256 (64 by default), 404 for a missing object. A plane with nothing masked is an ordinary state while masking, so that answers 204 with the reason in X-Formsculpt-Refused rather than 400: a 400 would print a failed request in the window's console after every stroke until all three planes have a shape. Checked: 200 with FSCM for a triplanar, 404 for id 99, 204 with the reason after mask_clear and for a sphere.
