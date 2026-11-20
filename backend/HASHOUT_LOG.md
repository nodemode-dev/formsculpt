# backend HASHOUT_LOG

Living log for the engine service side, _v0.1.0. Read the map and the TOC first; entries are anchored by file and symbol, and removed code is quoted in full.

## Org-navi-map

```text
backend/
  formsculpt_engine.py   PyInstaller entry for the desktop sidecar, --selfcheck
  boot_backend             dev server with reload on 127.0.0.1:8260
  oc_install               venv and requirements (macOS reads its own file)
  requirements.txt         pinned
  requirements_macos.txt   pinned twin without the Windows terminal package
  requirements_desktop*.txt  PyInstaller
  .env.example             FSC_PORT, FSC_CORS_ORIGINS
  app/__init__.py          __version__, a version site
  app/main.py              FastAPI app, middleware, routers, static mount
  app/core/config.py       names, ports, dist_dir, is_desktop, cors_origins
  app/core/isolation.py    CrossOriginIsolation ASGI middleware
  app/routers/system.py    /api/health, /api/version, /api/capabilities (engine status from the module), /api/integrations/blender
  app/routers/lightbox.py  /api/lightbox/projects: LightBox's Project shelf (list, save with a picture, open, picture)
  app/routers/library.py   Home's library routes: /api/sculpts, /api/sculpt-trash, /api/sculpt-templates, /api/sculpt-settings
  app/services/library.py  the sculpt library on LightBox's Project folder: names, atomic writes, the Trash, templates, thumbnails
  app/routers/engine.py    /api/engine/catalogue, /api/engine/dispatch, /api/engine/mesh/{id}
  app/services/engine.py   the one fsc_engine.Engine, status, dispatch, mesh_payload (FSCM binary)
```

## Contents

- [formsculpt_engine.py](#formsculpt_enginepy)
- [app/core/isolation.py](#appcoreisolationpy)
- [app/routers/system.py](#approuterssystempy)
- [app/main.py](#appmainpy)
- [The pre-release audit of the service, the shell and the scripts](#the-pre-release-audit-of-the-service-the-shell-and-the-scripts)
- [The desktop shell: opened files, one instance, the quit gate, text-field edits](#the-desktop-shell-opened-files-one-instance-the-quit-gate-text-field-edits)
- [The Home screen's library](#the-home-screens-library)
- [formsculpt 1.21.0 · the desktop shell and the scripts](#formsculpt-1210--the-desktop-shell-and-the-scripts)
- [formsculpt 1.22.0 · the agent dock 2.2.0](#formsculpt-1220--the-agent-dock-220)

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

### core/local_guard.py · LocalGuard: only this computer's own window and agents reach the engine
- refers-to: app/core/local_guard.py (new) :: LocalGuard; app/core/config.py :: window_origins (new); app/main.py :: app.add_middleware(LocalGuard)
- kind: rationale
- logged: v1.18.6 · 2026-10-25
The family stabilization plan's Phase 3a (`FAMILY_DEV_PLAN.md` v0.4.2, formai_dev sidecar; ledger FB-032). The engine answered a request whose Host was a foreign name: `curl -H 'Host: evil.example:8196' http://127.0.0.1:8196/api/health` returned 200. A DNS-rebinding page (a hostile name that resolves to 127.0.0.1) could therefore call every route as if it were the window: the terminal socket and the window-only `terminal.*` commands (which start shells), the Claude Code and chat commands, the agent consent routes, `/api/engine/dispatch` and the file routes (imports, exports, autosave), `/api/gof` and `/mcp`. The stroke WebSocket `/api/engine/stroke` checked no Origin at all, so any page could open it. CORS decides whether a page may read a response, not whether a request runs, so it did not help.
LocalGuard is a pure ASGI middleware in front of every route and websocket, added right after CrossOriginIsolation (so it is the outer layer of the two). It refuses a request whose Host is not 127.0.0.1, localhost or ::1. When an Origin header is present it must be one of `config.window_origins()` (`cors_origins()`, the preview port 5197, `tauri://localhost` and `http://tauri.localhost`) or exactly the engine's own origin, compared on parsed scheme, host and port, never by prefix, so `http://localhost.evil.com` and `null` fail. The desktop window loads from the engine's own address, so its origin is the request's own Host and needs no list entry. Requests with no Origin (the CLI, `setup/mcp_stdio.py`, Claude Code's MCP client, the Blender add-on) pass once the Host is local. HTTP refusals answer 403 with a JSON detail; websocket refusals close with 4403 before accept. The existing `_local` Origin check in routers/terminal_socket.py stays as it was.
Checked against the running engine: a foreign Host, `Origin: http://evil.example`, `http://localhost.evil.com` and `null` on `/api/health` answer 403 (4 of 4); no Origin, `http://127.0.0.1:8196`, `http://localhost:5196` and `tauri://localhost` answer 200 (4 of 4). `/mcp` initialize with no Origin answers (serverInfo formsculpt) and `setup/mcp_stdio.py` initialize answers 1.18.5 through the guard. `/api/engine/stroke`: a foreign Origin and a rebound Host are refused with 403; the own origin, `localhost:5196` and no Origin open. `/api/terminal/{sid}`: the same two are refused with 403 and the other three reach the route (4404, no such session). The built window served by the engine (`FSC_DIST`) in Chromium at 1440 by 900 with 4x CPU and network throttling: layout shift 0 on two shift runs and the throttle run, no page or console error, and no 403 from the window (the 8 refusals in the log are the probes).
Left for the family standard: a per-launch token on the routes that write files, run processes or answer consent, and a review of GET routes with side effects, which a cross-site page can still trigger without an Origin.

### core/local_guard.py · LocalGuard refuses cross-site requests that carry no Origin
- refers-to: app/core/local_guard.py :: LocalGuard._permitted
- kind: rationale
- logged: v1.18.7 · 2026-10-25
The family agent-stack standard, rule G4 (`AGENT_STACK_STANDARD.md` v0.2.0, owner decision D4, ledger FB-035, formai_dev sidecar; the design of formpdf 0.56.4). The entry above let a request with no Origin through once its Host was local, so a web page could still make the engine answer a GET by pointing an `<img>` or a link at `http://127.0.0.1:<port>/...`: browsers send no Origin on those, and any GET route with a side effect would run.
Browsers label every request with `Sec-Fetch-Site`, and only a page on another site gets `cross-site`. So a request with no Origin is now refused when that header says `cross-site` (trimmed, case-insensitive). A request that carries an Origin is still decided by the Origin rule alone, and `same-site` is kept on purpose: a dev window on `localhost:5196` loading straight from `localhost:8196` is same-site, and no outside page can be same-site with localhost. The CLI, `setup/mcp_stdio.py`, Claude Code's MCP client and the Blender add-on send neither header and pass as before.
Checked against the running engine: no Origin with `cross-site` 403; `same-site`, `same-origin`, `none`, no header at all, and `cross-site` with the allowed dev Origin 200; foreign Host and an `evil.example` Origin still 403. In Chromium, a page served from `http://127.0.0.2:9901` with an `<img>` and a clicked link pointing at `/api/health` got 403 for both, shown in the engine log. The built window served by the engine (`FSC_DIST`) in Chromium at 4x CPU, 150 ms latency and about 1.6 Mbps, two shift runs and one throttle run: layout shift 0, no errors, no new 403 (the 6 in the log are the probes; this engine's log lists refusals and sockets only, and the stroke socket was accepted). `/mcp` initialize with no Origin 200; `setup/mcp_stdio.py` (via `FSC_MCP_URL`) initializes and lists 184 tools.

### app/services/agents.py · permit refuses an outside agent when no window is open
- refers-to: app/services/agents.py :: permit
- kind: rationale
- logged: v1.18.8 · 2026-10-25
The family agent-stack standard, section 7.1, owner decision D2 (`AGENT_STACK_STANDARD.md` v0.2.0, formai_dev sidecar). `permit` returned None, so a changing tool ran, whenever `window_live()` was false: with no window open nobody could be asked, so everybody was allowed, and any local process that found the port could edit the scene. Removed code, quoted in full: `if inapp or _allowed is True or not window_live():` followed by `return None`.
Now the in-app token and an earlier yes (`_allowed is True`) still return None, and with no window the call is refused with "The formsculpt window is not open, so nobody can approve outside agents. Ask the person to open formsculpt and allow agents there, then retry." The 600 s decline check above it is unchanged, read-only tools never reach `permit`, and a live window still gets the dialog. The yes lives in the module's `_allowed` only (`answer`, cleared by `reset`): formsculpt keeps no persisted agents choice, so "allowed" with no window means the person allowed agents earlier in this engine's life. The agent text (the modeling skill in `.claude` and `.agents`, the agent-substrate sub-skill, eval 3, `AGENTS.md`, `CLAUDE.md`) says so.
Checked against an engine booted with no window: `tools/call` of `add_primitive` from an outside client got the new message; `get_scene` still answered; `POST /api/mcp/consent {"allow":true}` made the same `add_primitive` run headless, `POST /api/mcp/reset` made it refuse again; the in-app path (`call_tool(..., inapp=True)`) ran with no window. With the built window served by the engine (`FSC_DIST`) in Chromium at 4x CPU, 150 ms latency and about 1.6 Mbps: two shift runs and one throttle run gave layout shift 0, no errors and no 403; an outside `add_primitive` made the consent dialog appear and the call waited for the click. `./formsculpt agents sync --check` reports only the version line of the generated tool reference (the native catalogue binary is still the 1.18.6 build).

### app/core/config.py · own dev ports 5260, 5261 and 8260
- refers-to: app/core/config.py :: FRONTEND_PORT, BACKEND_PORT; frontend/vite.config.ts :: FRONTEND_PORT, BACKEND_PORT
- kind: rationale
- logged: v1.18.8 · 2026-10-25
The family agent-stack standard, section 9, owner decision D3, ledger FB-018 (`AGENT_STACK_STANDARD.md` v0.2.0, formai_dev sidecar). formsculpt's 5196, 5197 and 8196 were also formref's, so the two could not run side by side and a request meant for one reached the other. The dev window is now 5260, the preview 5261 and the engine 8260. Changed at every place the number was formsculpt's own default: `config.py`, `vite.config.ts` (server, preview and proxy target), `boot_backend` and its `.bat`, `.env.example`, `formsculpt` and `formsculpt.bat`, `setup/` (`boot_all`, `serve`, `doctor`, `run_tests`, `mcp_stdio.py`, `run_script.py`), `frontend/e2e/smoke.py`, the one origin in `backend/tests/test_service.py`, the CI health probe, the Blender add-on's default address (manifest unchanged, only the preference default), the README, the wiki's Getting Started, and the harness skill's sub-skills. `window_origins()` and `cors_origins()` derive from the two constants, so the guard now admits `localhost:5260`, `127.0.0.1:5260` and the preview pair 5261, and no longer admits 5196. The packaged shell still starts the frozen engine on a free port and is untouched. Old numbers left on purpose: CHANGELOG and HASHOUT entries that record what was true at the time.
Checked by booting `./formsculpt boot` with no FSC_ variables: Vite on 5260 and uvicorn on 8260 (nothing on 5196, 8196). `Origin: http://localhost:5260` 200, `http://localhost:5261` 200, the old `http://localhost:5196` 403, a foreign Host, a foreign Origin and a cross-site request with no Origin 403, plain 200, and the Vite proxy on 5260 answered `/api/health`. `./formsculpt mcp` found the engine on 8260 with no variable and listed 184 tools. The dev window on 5260 in Chromium at 4x CPU, 150 ms latency and about 1.6 Mbps (two shift runs, one throttle run): layout shift 0, no errors; the built window on 8260 the same, with no 403 in the engine log.

## backend/tests

### backend/tests · the family suite and the loopback Host
- refers-to: backend/tests/conftest.py :: client; backend/tests/family.py; backend/tests/test_f1_guard.py through test_undo.py
- kind: rationale
- logged: v1.19.0 · 2026-10-26
`FAMILY_TEST_PLAN.md` v0.3.0 (formai_dev sidecar): F1 to F6 and F9 plus the P1 gap fills. `family.py` holds the per-app block (ports, env names, window origins, version sites, adapters to `app.services.agents`) and sets a sandbox data folder and HOME before `app.main` is imported; `conftest.py` imports it first. The existing `client` fixture used TestClient's default Host `testserver`, which `LocalGuard` refuses with 403, so 17 of the 18 tests in `test_service.py` failed on the 1.18.9 tree (not an app bug: the guard is right). The fixture now uses `http://127.0.0.1:8260`. The websocket guard tests send an explicit loopback Host so the Origin check is what refuses them, and a bare TestClient case proves the default Host is refused. Differences from the template: no `engine.json` (the stdio relay is pointed at the booted engine with `FSC_MCP_URL`), consent is `agents.answer` and `agents.reset` with a ten-minute decline cooldown rather than an ask, blocked and allowed policy, and the refusal texts are matched by substring. Open question for the owner, not changed: `permit` checks the decline cooldown before the in-app token, so declining outside agents also refuses the in-app agent for ten minutes.
Checked: 148 passed on the engine module built with maturin; the 130 new tests pass on the 1.18.9 tree and the old fixture fails 17.

## setup and the install scripts

### backend/oc_install, backend/boot_backend, setup/lib.sh, setup/freeze_backend · the Windows build in Git Bash
- refers-to: backend/oc_install; backend/boot_backend; setup/lib.sh :: fs_python, fs_native_path (new); setup/freeze_backend
- kind: rationale
- logged: v1.19.2 · 2026-10-30
formcompiler's Windows leg runs `./formsculpt install`, `install-desktop` and `desktop build` in Git Bash on windows-latest, where setup-python provides `python` (and `py`) but not always `python3`, and a venv keeps `Scripts/activate` instead of `bin/activate` (`AUDIT_family_builds_2026-10-30.md`, formcompiler sidecar v0.5.0, family problem 3). `backend/oc_install` tried only python3.14 to python3 and then ran `source venv/bin/activate`, so the first step of every Windows build failed. It now sources `setup/lib.sh` and uses `fs_python` and `fs_venv`, so the candidate list and the venv layout live in one place. Removed code, quoted in full: the loop `for c in python3.14 python3.13 python3.12 python3; do` / `if command -v "$c" >/dev/null 2>&1 && "$c" -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 12) else 1)' 2>/dev/null; then PY="$c"; break; fi` / `done` with `PY=""` before it and `[ -n "$PY" ] || { echo "[formsculpt] Python 3.12 or newer is required" >&2; exit 1; }` after it, and `source venv/bin/activate`. `boot_backend` had the same `venv/bin/activate` test and source (`[ -f venv/bin/activate ] || { echo "[formsculpt] backend venv missing, run ./formsculpt install" >&2; exit 1; }`), now `fs_venv`. `fs_python` gains `python` as its last candidate; the version check still rejects a Windows Store `python3` stub or a Python older than 3.12, so the loop moves on to the next name.
`freeze_backend` passed PyInstaller `--add-data "$FSC_ROOT/...;dest"`. On Windows `$FSC_ROOT` is an MSYS path (`/d/a/...`), and Git Bash's argument conversion does not reliably rewrite a path inside a `src;dest` pair, so PyInstaller (a native Windows program) could get a source path it cannot open. `fs_native_path` runs `cygpath -w` on Windows and passes the path through unchanged elsewhere; the four `--add-data` sources and the entry script go through it. The rest of the chain was checked for the same assumptions: `fs_venv`, `engine_build` and `doctor` already accept `Scripts/activate`; `install_all`, `install_desktop`, `build_frontend`, `engine_wasm` and `desktop_build` call no `python3` and no `venv/bin`; `build_app` refuses Windows and points at `build_app.bat`. `formsculpt mcp`, `agents` and `i18n` still call `python3`; they are not part of a build and are left for a later change.

### build_app.bat · the Windows freeze carries the integrations
- refers-to: build_app.bat (the pyinstaller line)
- kind: rationale
- logged: v1.19.2 · 2026-10-30
`build_app.bat` is the supported start-to-finish build on Windows (CLAUDE.md, and `build_app` sends Windows users to it), but its PyInstaller line bundled only `frontend\dist`, so its sidecar had no Blender add-on, Unreal or Unity receiver and `/api/integrations/blender`, `/unreal` and `/unity` had nothing to serve. It now passes the same three `--add-data` pairs as `setup/freeze_backend`. formcompiler does not run this file; it builds Windows through `./formsculpt` in Git Bash.

## app/main.py

### app/main.py · .js, .mjs and .css keep their types on Windows
- refers-to: app/main.py :: the three mimetypes.add_type calls after the last import
- kind: rationale
- logged: v1.19.3 · 2026-10-31
Python's `mimetypes` takes file types from the Windows registry (`HKEY_CLASSES_ROOT\.js`, value `Content Type`) over its own table, and some installers and editors set `.js` to `text/plain`. On those PCs the engine served the built window's scripts as `text/plain`, WebView2 refused every module script under strict MIME checking, and the desktop window stayed blank; macOS, Linux, dev mode (Vite serves the page) and Windows PCs with the default value never showed it. The three calls pin `.js` and `.mjs` to `text/javascript` and `.css` to `text/css`, which covers `StaticFiles`, `FileResponse` and any direct `guess_type`. They sit after the last import, not above the imports: a module that re-reads the registry while importing cannot undo them, and no import follows a statement (E402). They look redundant off Windows; keep them. Repro on a Windows PC: `reg add "HKCU\Software\Classes\.js" /v "Content Type" /d "text/plain" /f`, then launch the rebuilt app; restore with `/d "text/javascript"`. Checked here: with the table poisoned to `text/plain` for js, mjs and css through `mimetypes.init(files=...)`, the three calls return `text/javascript`, `text/javascript` and `text/css`; `py_compile` passes. Not yet run on a Windows PC.

## The pre-release audit of the service, the shell and the scripts

### routers/nodemode.py · a send no longer stops the whole service
- refers-to: app/routers/nodemode.py :: send, _announce (new)
- kind: rationale
- logged: v1.20.0 · 2026-11-15
`send` is an async route and called `_upload` twice and `outbound.request` once directly, so each blocking `urlopen` (20 s timeout) ran on the event loop. Seen on a booted service: with a NodeMode address that accepts the connection and never answers, `GET /api/health` got no reply for 6 s while the send was in flight (the window's polls, strokes and every other route wait the same way). The three calls now run through `run_in_threadpool`; the announce moved into `_announce` so it can. Removed code, quoted in full: `model_url = _upload(base, f"{stem}.glb", glb, "model/gltf-binary")`, `preview_url = _upload(base, f"{stem}.png", png, "image/png")` and `with outbound.request(base + "/api/formsculpt/receive", announce, {"Content-Type": "application/json"}, "POST"):` followed by `placed = True`.
Checked against a booted service: the same hanging address, `/api/health` 200 in 2 ms during the send and the send still ends 502; against a stand-in receiver, `status` gives running and receiver true and `send` gives placed true with both files.

### routers/engine.py and routers/mcp.py · three requests that answered 500
- refers-to: app/routers/engine.py :: export_file; app/routers/mcp.py :: mcp
- kind: rationale
- logged: v1.20.0 · 2026-11-15
`GET /api/engine/export?ids=abc` raised ValueError outside the route's own try, `POST /mcp` with `params` as a list raised AttributeError on `params.get`, and `tools/call` with a `name` that is a list raised `TypeError: unhashable type` on `name in names`; each answered 500 with a traceback in the engine log. The id list is now parsed inside the try (400 with the reason), non-object `params` answer 400 with JSON-RPC -32602, and `name` must be a string before it is looked up (-32602, "No tool named"). Checked on a booted service: 400, 400 and 200 with the error object; a valid export and `tools/call get_scene` unchanged.

### app/main.py · the Chat tab's Claude Code process ends with the service
- refers-to: app/main.py :: lifespan; app/services/cmd_chat.py :: cancel
- kind: rationale
- logged: v1.20.0 · 2026-11-15
The lifespan closed the terminal sessions at shutdown but not the `claude -p` process a Chat answer runs in. Seen with a stand-in program set as `claudeBin`: after `chat.send` and SIGTERM to the service, the service was gone and the child was still running. `lifespan` now calls `cmd_chat.cancel()` before `terminal.close_all()`. Checked the same way: the child is gone with the service, the terminal's shell too.

### formsculpt_engine.py · the desktop engine prefers 8260, and removes what it unpacked when the shell is gone
- refers-to: formsculpt_engine.py :: free_port, watch_parent; src-tauri/src/main.rs :: kill_backend
- kind: rationale
- logged: v1.20.0 · 2026-11-15
Port. The shell starts the engine with `FSC_PORT=0` and `free_port` asked the OS for any port, so the installed app listened somewhere new on every launch, while everything that talks to a running formsculpt assumes 8260: the Blender add-on's default address (its README says the desktop app works), `setup/mcp_stdio.py` (which then started a second engine with its own scene beside the open window), `setup/run_script.py` and the harness skill. `free_port` now tries `config.BACKEND_PORT` first and falls back to any free port when it is taken (a dev service or a second window), which is formdeck's `FORMDECK_PORT_PREFER`. The probe sets SO_REUSEADDR off Windows so a relaunch right after a quit gets the port back, as uvicorn's own socket does. Removed code, quoted in full: `with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:` / `s.bind(("127.0.0.1", 0))` / `return s.getsockname()[1]`.
Stopping. The one-file sidecar is PyInstaller's launcher with the server as its child; the launcher removes the `_MEI` folder it unpacked when the child exits. `kill_backend` used `child.kill()`, a forced kill that reaches only the launcher (PyInstaller 6.22.3's launcher has no parent-death signal and no job object: read in its `bootloader/src`), so on every quit the server lived on until `watch_parent` saw the shell gone and called `os._exit(0)`, and nothing removed the folder: one copy of the window, Python and the engine left in the temporary folder per launch. This is formvis's fault of 1.86.2 and the same two changes: `kill_backend` sends SIGTERM first on unix (the launcher passes it to the server, uvicorn shuts down, the lifespan closes terminals and Chat, the launcher removes the folder) and waits up to 1.5 s before the forced kill; and `watch_parent` removes `sys._MEIPASS` (only when its name starts with `_MEI`) before `os._exit(0)`, for the cases where the launcher is already dead. On Windows the forced kill stays, so the server still exits through `watch_parent`, which now removes everything Windows lets a running program delete; the loaded libraries stay behind. formvis's `sweep_stale_unpacks` (1.86.3) would clear those at the next launch and is not copied here.
Checked: `formsculpt_engine.py` run unfrozen with `FSC_PORT=0` printed `http://127.0.0.1:8260`, a second one beside it took another port, and the first exited within 3 s of the process named in `FSC_PARENT_PID` ending; `watch_parent` with a stand-in `_MEI` folder as `sys._MEIPASS` removed it before exiting; the `kill_backend` block compiled alone with rustc against a stand-in child and returned in 55 ms after SIGTERM with no forced kill. Not run: the frozen sidecar, and `src-tauri` itself (no webkit2gtk here).

### app/services/scripting.py, setup/freeze_backend, build_app.bat · the desktop app carries the plugins
- refers-to: app/services/scripting.py :: ROOT; setup/freeze_backend and build_app.bat (the pyinstaller line)
- kind: rationale
- logged: v1.20.0 · 2026-11-15
`ROOT` was three folders above `scripting.py`. In the frozen engine that file is `<unpack folder>/app/services/scripting.py`, so `ROOT / "plugins"` was `plugins` in the temporary folder itself (`/tmp/plugins` on Linux, which any user of the computer can create), and the freeze did not pack `plugins/` at all: the installed app listed none of rock, pedestal and rivets. `ROOT` is now `sys._MEIPASS` when frozen, and both freezes pass `--add-data` for `plugins`, as they do for the integrations. Removed code, quoted in full: `ROOT = Path(__file__).resolve().parents[3]`. Checked: with a stand-in unpack folder as `sys._MEIPASS`, `list_plugins` finds the three there; from the checkout it finds them as before, and the three ran on a booted service. Not run: PyInstaller.

### setup/serve, backend/boot_backend, .env.example · loopback only
- refers-to: setup/serve; backend/boot_backend; backend/.env.example
- kind: rationale
- logged: v1.20.0 · 2026-11-15
Both scripts passed `--host "${FSC_HOST:-127.0.0.1}"`. `LocalGuard` trusts the Host header, which the sender writes: with `FSC_HOST=0.0.0.0`, `curl -H 'Host: 127.0.0.1' http://<this computer's network address>:8263/api/commands/terminal.list -X POST` answered 200, so anyone on the network could reach the window-only terminal commands. The harness's engine-service rule is "Bind 127.0.0.1:8260 only", and the `.bat` twin, CI and the desktop engine already do. The two scripts now pass `--host 127.0.0.1` and the example file no longer lists the variable. Removed code, quoted in full: `--host "${FSC_HOST:-127.0.0.1}"` (twice) and the line `FSC_HOST=127.0.0.1`.

### formsculpt.bat · install and engine build the engine module, serve sets FSC_DIST without a trailing space
- refers-to: formsculpt.bat :: install, engine, serve
- kind: rationale
- logged: v1.20.0 · 2026-11-15
The README sends Windows users to `formsculpt_oc_install.bat` and `formsculpt.bat`. `install` ran only the two `oc_install.bat` files, where `setup/install_all` also builds the engine, and `engine` ran `cargo build --workspace --release`, which never installs the Python module (`setup/engine_build` excludes fsc-py and runs `maturin develop`). So a Windows checkout installed this way booted a service that answers 503 "the engine module is not built" on every engine route, with no `.bat` command that builds it. `engine` now does what `build_app.bat` already does (the workspace without fsc-py, then `maturin develop --release` in the venv with `CARGO_TARGET_DIR` on `native\target`), and `install` runs it when cargo is there, saying so when it is not. `serve` had `set FSC_DIST=%~dp0frontend\dist && uvicorn ...`: cmd keeps the space before `&&` in the value, so the service looked for `dist \index.html`; the assignment is now quoted. Removed code, quoted in full: `cd native && cargo build --workspace --release` and `set FSC_DIST=%~dp0frontend\dist`. Not run: there is no Windows here; the file keeps CRLF line ends.

### blender_manifest.toml and ci.yml · the add-on's website, build_app in the shell syntax check
- refers-to: integrations/blender/formsculpt_link/blender_manifest.toml :: website; .github/workflows/ci.yml :: scripts, Shell syntax
- kind: rationale
- logged: v1.20.0 · 2026-11-15
The add-on's `website` named `github.com/nodemode-ai/formsculpt`; the repository is `nodemode-dev/formsculpt` (the remote, and the README's clone line), so Blender's extension page linked to the old account. The CI loop that runs `bash -n` listed every shell script except `build_app`; it is in the list now. Checked: `bash -n build_app` passes.

## The desktop shell: opened files, one instance, the quit gate, text-field edits

The page's half is logged in `frontend/HASHOUT_LOG.md` under the same heading.

### src-tauri/src/main.rs · a double-clicked project opens in the running window
- refers-to: src-tauri/src/main.rs :: Opens, OpenQueue, open_paths, paths_from_args, show_main_window, take_pending_opens, single_instance, setup (launch arguments), run (RunEvent::Opened); src-tauri/Cargo.toml :: tauri-plugin-single-instance; src-tauri/build.rs; src-tauri/capabilities/engine-page.json
- kind: rationale
- logged: v1.20.0 · 2026-11-15
`tauri.conf.json` declares the `.fsc` association and nothing handled it: the app opened with an empty scene, and a second double-click started a second app with a second engine. The three paths formvis has are here now, with its code: the launch arguments (Windows and Linux), `RunEvent::Opened` (macOS: Finder and the Dock) and a second launch, which `tauri-plugin-single-instance` 2.4.5 (the version formvis locks; MIT or Apache-2.0) hands to the running app before it exits. Paths wait in `OpenQueue` until the page has its listener and calls `take_pending_opens`; later ones go out as the event `formsculpt://open-paths`, and the window is unminimized, shown and focused. Left out of formvis's version: the `--command`, `--action` and `--script` launch arguments, and the Linux hand-off key per data folder (formvis keys the D-Bus name on its data folder so two folders can run side by side; formsculpt's shell takes no data folder argument, so the plugin's own key, the application identifier, is used: one running formsculpt per login).
`Cargo.lock` gained the plugin and the 32 crates it brings on Linux and Windows (zbus and the async-io family; all MIT or Apache-2.0), written by cargo.
Checked: `cargo check` of src-tauri on Linux, exit 0, no warning. Not compiled there: the `RunEvent::Opened` arm (macOS only). Not run: any of it in a built app.

### src-tauri/src/main.rs · quit and close ask the page first
- refers-to: src-tauri/src/main.rs :: ExitGate, open_exit_gate, ask_page_to_quit, confirm_exit, on_window_event (CloseRequested), run (RunEvent::ExitRequested), check_for_updates
- kind: rationale
- logged: v1.20.0 · 2026-11-15
The run loop handled `RunEvent::Exit` only, so Cmd+Q, the native Quit item (new with the macOS menu bar) and the close button ended the app with unsaved work unasked. formvis's gate: `CloseRequested` on the main window and `ExitRequested` send `formsculpt://exit-requested` and are held back while the page is listening; the page answers with `confirm_exit`, which opens the gate and exits, or with `cancel`, which leaves the app running. The app exits at once when the page cannot answer (not loaded yet, or no main window), so a splash with a dead engine never traps the person. The update path opens the gate before it restarts. `on_window_event` also puts the traffic lights back after a resize, focus, theme or scale change, as formvis does.
The engine-failure path ended with `std::process::exit(1)`; it is `handle.exit(1)` now, formvis's form, so the Exit event runs.
Removed code, quoted in full:

```rust
    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            kill_backend(handle);
        }
    });
```

Checked: `cargo check`, exit 0. Not run: a real quit; whether macOS raises `ExitRequested` for Cmd+Q is taken from formvis, which relies on it.

### src-tauri/src/text_edit.rs · Edit items act on a focused text field
- refers-to: src-tauri/src/text_edit.rs :: text_edit, send; src-tauri/src/main.rs :: generate_handler
- kind: rationale
- logged: v1.20.0 · 2026-11-15
formvis's file, unchanged but for its header comment. On macOS the menu bar's Edit items own Cmd+Z, X, C, V and A, so a focused text field never sees those keys; while one has focus the page calls `text_edit` and the standard action (`undo:`, `redo:`, `cut:`, `copy:`, `paste:`, `selectAll:`) goes to the first responder, the web view, which edits the field as its context menu does. Elsewhere the command does nothing and the page uses the browser's own editing.
Checked: `cargo check`, exit 0, which compiles the command and the empty `send`. Not compiled on Linux: the macOS `send` (objc2 `msg_send!` to `NSApplication`), copied from formvis line for line.

## The Home screen's library

The service side of the Home screen (frontend/src/ui/home.ts), shaped after formdeck's deck routes. No native change, no new dependency.

### app/services/library.py · where the files live
- refers-to: app/services/library.py :: folder, _home, _trash, _files
- kind: rationale
- logged: v1.20.0 · 2026-11-15
The library is LightBox's Project folder as it stands (`<data>/LightBox/Project/<name>.fsc` with LightBox's square `<name>.png`), so nothing is migrated and both screens list the same sculpts. Home's own files sit in two hidden folders there, which LightBox's `*.fsc` glob never sees: `.home/` holds `<name>.thumb` (the 16:9 picture), `<name>.json` (objects, triangles, the template it came from) and `home.json` (pinned, recentHidden, when each was last opened); `.trash/<name>-<unix seconds>/` holds a deleted sculpt as `sculpt.fsc`, `picture.png`, `thumb`, `info.json`. The entry's name is all a restore needs, as in formdeck. Template pictures are drawn once and kept in `<data>/LightBox/.templates/<id>-<version>.png`.

### app/services/library.py · names are ids, and every one is checked
- refers-to: app/services/library.py :: checked, _within, _entry, NAME, ENTRY, RESERVED
- kind: warning-context
- logged: v1.20.0 · 2026-11-15
A sculpt's id is its file name, under LightBox's own rule (1 to 64 letters, digits, spaces, dots, dashes, underscores, starting with a letter or digit), plus no trailing dot and no Windows device name. Every id and trash entry from a request passes `checked` or `_entry` and then `_within`, which refuses a symlink or a path that resolves outside the library, before any file is touched. Because an id is a free name, the Trash, the templates and the settings are not under `/api/sculpts/...` (a sculpt called `trash` would shadow them): they are `/api/sculpt-trash`, `/api/sculpt-templates`, `/api/sculpt-settings`. That is the one place the routes leave formdeck's shape.

### app/services/library.py · writes
- refers-to: app/services/library.py :: _write, _store, remove, restore
- kind: rationale
- logged: v1.20.0 · 2026-11-15
Every file is written to `<name>.part` beside it and moved into place with `os.replace`, so a crash leaves the old file or the new one. Moving to the Trash and restoring are `os.replace` moves inside the same folder tree. A restore whose name is taken again becomes `<name> restored`; a duplicate is `<name> copy`; a new sculpt whose name is taken gets ` 2`, ` 3`. One lock serialises the mutating calls.

### app/services/library.py · templates are built in a second engine
- refers-to: app/services/library.py :: TEMPLATES, _build, create, template_picture, _scratch
- kind: rationale
- logged: v1.20.0 · 2026-11-15
The ten starting meshes are the engine's own primitives (`add_primitive`), listed in `TEMPLATES` with the commands that build them; no asset is shipped. Creating one, or drawing its picture, runs in a second `fsc_engine.Engine` kept by this module, so the scene on screen is never replaced by a template being built; the new sculpt is written to the library and the window then opens it like any other.

### app/services/library.py · thumbnails
- refers-to: app/services/library.py :: _store, put_thumb, thumb, THUMB_SIZE, PICTURE_SIZE
- kind: rationale
- logged: v1.20.0 · 2026-11-15
A save writes LightBox's 160 px square picture as before and a 480 by 270 `get_view` render as the Home thumbnail. The window then replaces that thumbnail with a 16:9 capture of the viewport (`PUT /api/sculpts/{id}/thumb`, PNG, JPEG or WebP by its first bytes, 2 MB at most); when it says it will (`thumb=client`) the service skips its own 16:9 render. A sculpt saved by LightBox before 1.20.0 has only the square picture, which the thumb route serves and Home shows whole inside its 16:9 box.

### app/routers/library.py · the routes
- refers-to: app/routers/library.py :: router
- kind: explainer
- logged: v1.20.0 · 2026-11-15
`GET /api/sculpts`, `POST /api/sculpts` (`{title?, template?}` or `{title?, fromScene: true}`), `POST /api/sculpts/{id}/open`, `POST /api/sculpts/{id}/save`, `POST /api/sculpts/{id}/duplicate`, `DELETE /api/sculpts/{id}`, `PUT` and `GET /api/sculpts/{id}/thumb`, `GET /api/sculpt-trash`, `POST /api/sculpt-trash/{entry}/restore`, `DELETE /api/sculpt-trash/{entry}`, `DELETE /api/sculpt-trash`, `GET /api/sculpt-templates`, `GET /api/sculpt-templates/{id}/thumb`, `GET` and `POST /api/sculpt-settings`. There is no rename route: renaming stays with saving under a new name. All of them sit behind `LocalGuard` like every other route.
Checked: the existing suite (148 passed), and the routes in process against a temporary data folder, including `../x`, `con`, an encoded slash and a `.home-1` trash entry, each refused. No test file was added (not assigned).

## formsculpt 1.21.0 · the desktop shell and the scripts

### src-tauri/src/main.rs · the engine's start and its end
- refers-to: src-tauri/src/main.rs :: PORT_WAIT, EngineStart, read_port, quit_launch, keep_waiting, engine_stopped, setup (sidecar spawn)
- kind: rationale
- logged: v1.21.0 · 2026-11-16
Three `expect` calls ended the app with no word when the sidecar was missing or could not be run (security software, a broken install); a launch whose engine printed nothing kept the splash up for ever; an engine that died after the window opened left a window whose every call failed. The wait for the address is formtools' `read_port` (a 60 s timer thread raced against the engine's output) and now ends one of three ways: an address opens the window; an exit (or a spawn error) closes the splash and says why in an error dialog, with the exit code; a silent minute asks Keep Waiting or Quit, since a first start on a slow computer can be long. After the window opens the shell keeps reading the engine's events: a `Terminated` while the `Backend` slot still holds the child means the engine died on its own (the app's own stops go through `kill_backend`, which empties the slot first), so the shell opens the exit gate and offers Restart or Quit, naming Recover Autosave. The dialogs are English only, as the shell's other dialogs.
Removed code, quoted in full:

```rust
            let (mut rx, child) = app
                .shell()
                .sidecar("formsculpt-engine")
                .expect("sidecar binary is bundled")
                .env("FSC_PORT", "0")
                .env("FSC_PARENT_PID", std::process::id().to_string())
                .spawn()
                .expect("the formsculpt engine failed to spawn");
```

and the old wait, a `while let Some(event) = rx.recv().await` loop that broke on `Terminated` and showed "The formsculpt engine failed to start. Please relaunch the app." when no port came.
Checked: tauri-check and `cargo check --locked` of src-tauri on Linux, exit 0; `read_port`, the slot test and the data folder key copied into a stand-alone tokio program against stand-in engines (silent `sleep`, `exit 3`, one that prints a uvicorn address and then exits, a missing program), every case as expected. Not run: the dialogs in a built app, Windows, macOS.

### src-tauri/src/main.rs · one window per data folder on Linux
- refers-to: src-tauri/src/main.rs :: data_folder, single_instance
- kind: rationale
- logged: v1.21.0 · 2026-11-16
The single-instance plugin keys its D-Bus name on the application identifier, so a second formsculpt with another `FSC_DATA_DIR` handed its files to the first and exited. formvis's form: the name is `ai.nodemode.formsculpt.d` plus an FNV-1a hash of the canonical data folder, found as `app/core/config.py::data_dir` finds it (`FSC_DATA_DIR`, `~` expanded, else `~/Formsculpt`). The shell has no data folder argument, so only the variable is read. macOS and Windows keep the plugin's own key.

### formsculpt_engine.py · sweep_stale_unpacks
- refers-to: formsculpt_engine.py :: sweep_stale_unpacks, main
- kind: rationale
- logged: v1.21.0 · 2026-11-16
formvis's sweep, extended to Windows. The one-file launcher unpacks to `_MEI` plus its process id in eight hex digits plus a suffix (six random characters on macOS and Linux, `_wtempnam`'s counter on Windows, per PyInstaller 6.22.3's bootloader); on Windows the engine cannot delete the files it still has loaded, so `watch_parent`'s own clean-up leaves part of the folder behind on every quit. At start, in a background thread, a folder of that form whose process is gone is removed; this engine's own and any whose id is alive are kept. On Windows liveness is `parent_alive` (OpenProcess), since `os.kill` there ends the process. Checked on Linux with fake folders (a dead id removed, a live id, a non-matching name and the own folder kept). Not run on Windows.

### setup/i18n.py · extract
- refers-to: setup/i18n.py :: extract
- kind: rationale
- logged: v1.21.0 · 2026-11-16
`sorted(found, key=str.lower)` is stable, but `found` is a set whose order changes with each run's hash seed, so strings equal but for case swapped places on every extract. The key is `(s.lower(), s)` now, a total order. Checked on a copy of the tree with three hash seeds: identical output.

### setup/mcp_stdio.py · start_engine
- refers-to: setup/mcp_stdio.py :: start_engine
- kind: rationale
- logged: v1.21.0 · 2026-11-16
It started the bash launcher on every system, which Windows cannot run from cmd. formtxt's form: `formsculpt.bat` on Windows, started detached in its own process group (`0x208`), `start_new_session` elsewhere. Not run on Windows.

### formsculpt.bat · parity with the shell launcher
- refers-to: formsculpt.bat :: the dispatch list, install-desktop, freeze, desktop, app, doctor, test, bench, mcp, script, i18n, agents, clean, serve, engine
- kind: rationale
- logged: v1.21.0 · 2026-11-16
The verbs the shell launcher has and the `.bat` lacked, each following its `setup/` script: `install-desktop`, `freeze` (the same PyInstaller line with `;` separators and `winpty`), `desktop dev|build`, `app`, `doctor`, `test [all|engine|service|e2e]`, `bench`, `mcp`, `script`, `i18n`, `agents`, `clean [--all]`, and `engine check|wasm`. Python is `python`, as `build_app.bat` and `backend\oc_install.bat` find it. `serve` builds only when `frontend\dist\index.html` is missing and honours `FSC_PORT`, as `setup/serve` does; `engine` installs maturin when the venv lacks it and fails with exit 1 when a build fails. `engine wasm` has no `-Zbuild-std` fallback: it names the `rustup target add` line instead. Removed code, quoted in full: `cd frontend && call npm run build && cd ..\backend && call venv\Scripts\activate.bat && set "FSC_DIST=%~dp0frontend\dist" && uvicorn app.main:app --host 127.0.0.1 --port 8260`. Not run: no Windows here; read against `formsculpt` and `setup/*` line by line.

### requirements_macos.txt, requirements_desktop_macos.txt · pinned
- refers-to: requirements_macos.txt, requirements_desktop_macos.txt
- kind: rationale
- logged: v1.21.0 · 2026-11-16
Both were unpinned, so a macOS release build took whatever was newest that day. They carry the versions `requirements.txt` and `requirements_desktop.txt` pin (fastapi 0.141.1, uvicorn 0.53.0, python-multipart 0.0.32, maturin 1.11.5, ptyprocess 0.7.0, pyinstaller 6.22.3).

### .github/workflows · the desktop shell is compiled, actions pinned
- refers-to: .github/workflows/ci.yml :: desktop-shell, every uses; .github/workflows/wiki.yml :: publish
- kind: rationale
- logged: v1.21.0 · 2026-11-16
A `desktop-shell` job runs `cargo check --locked` in `src-tauri` on Ubuntu after the window job's apt mirror fallback and the Tauri package list from `setup/install_desktop`, with a built window and an empty sidecar placeholder for the build script (formfabric's job). Every action is pinned to formcompiler's commit and version: checkout v7.0.1, setup-node v7.1.0, setup-python v7.1.0, rust-cache v2.9.3.

### integrations/blender/formsculpt_link · the toggles after a restart
- refers-to: integrations/blender/formsculpt_link/__init__.py :: _resume, register, unregister
- kind: rationale
- logged: v1.21.0 · 2026-11-16
Blender saves the add-on's Live and Watch GoF preferences but runs no `update` callback when it loads them, so after a restart both showed on and no timer ran until toggled. `register` now queues a one-shot persistent timer that calls the two update functions with the loaded preferences; `unregister` removes it if it has not run. Not run: no Blender here.

### app/services/library.py, app/routers/library.py · rename and Save As
- refers-to: app/services/library.py :: _free, rename, create_from_scene (exact); app/routers/library.py :: NewSculpt.exact, NewName, rename
- kind: rationale
- logged: v1.21.0 · 2026-11-16
The 1.20.0 entry above says there is no rename route; there is one now, because the window has Save As and Rename (owner decision list, final verification 1). `POST /api/sculpts/{id}/rename` with `{title}`: `checked` refuses a name outside the library's rule (400), `_free` refuses a name another sculpt has, compared without case (409), the sculpt itself excepted so a change of case is allowed; under the lock the four files (`.fsc`, LightBox's `.png`, `.home/<name>.thumb` and `.json`) move with `os.replace`, a stray file already at the new name is removed, and the pinned, hidden and opened lists follow the new name. `POST /api/sculpts` with `fromScene` and `exact: true` (Save As) keeps the typed name and refuses a taken or empty one instead of numbering it. Checked with curl against a temporary data folder: 409 on a clash (also by case), 400 on `../x`, `con` and an empty name, 404 on an unknown sculpt, a case-only rename, the files and `.home` entries moved; backend suite 220 passed.

### app/routers/engine.py · export_file, ids out of range
- refers-to: app/routers/engine.py :: export_file (after the ids are parsed)
- kind: rationale
- logged: v1.21.0 · 2026-11-16
An id that parses as an integer but is not an engine id (0, -1, past 32 or 64 bits) reached `engine.export_bytes` and the conversion into the engine raised `OverflowError`, so the route answered 500. Ids outside 1 to 4294967295 now raise the same `ValueError` the route already turns into 400, with the reason. Checked: `tests/test_local_service.py::test_an_export_with_ids_out_of_range_is_refused_and_never_a_500` (was a strict expected failure), backend suite 228 passed.

### app/routers/mcp.py · tools/call, arguments that are not an object
- refers-to: app/routers/mcp.py :: mcp, the tools/call branch
- kind: rationale
- logged: v1.21.0 · 2026-11-16
`tools/call` with `arguments` that is not an object (a list, a string) was answered -32602 "No tool named get_scene" for a tool that exists. It is now -32602 "arguments must be an object"; an unknown or non-string name keeps "No tool named". Checked: `tests/test_local_service.py::test_a_tools_call_whose_arguments_are_not_an_object_says_so` (was a strict expected failure).

## formsculpt 1.22.0 · the agent dock 2.2.0

### app/services · the dock's engine contract, extended in place
- refers-to: app/services/terminal.py, cmd_terminal.py, claude_code.py, agents.py, agent_programs.py (new), agent_lookup.py (new); app/routers/terminal_socket.py, mcp.py
- kind: rationale
- logged: v1.22.0 · 2026-11-20
The owner chose to replace the old dock with the family's shared agent dock 2.2.0 (`frontend/src/dock/`, copied from FormVis 9480a07). The engine side was extended, not replaced by ports of FormTxt's `routers/terminal.py` and `services/terminal.py`: formsculpt's terminals already live on the service's window-only command bus (`terminal.*`, `claudecode.*`, behind `LocalGuard`) and a loopback-origin socket, which is the caller rule of clauses 1 and 2, so a second set of REST routes would only duplicate it. New window-only commands: `terminal.agents` (clause 10 status for claude, codex, cursor and antigravity in one answer, plus `contract: "2.2.0"` for clause 16) and `terminal.agent` (opens a codex, cursor or antigravity tab). No route or command name changed.

### app/services/agent_lookup.py · the family's finder, unchanged
- refers-to: app/services/agent_lookup.py
- kind: rationale
- logged: v1.22.0 · 2026-11-20
Copied whole from FormVis (identical in FormTxt and FormDeck), VERSION 1.0.1, SHA-256 `62667f9b68b7d101e1896ce617240424e33b7e498c560bc79d45f0ab26c77d51`; `./formsculpt agents check` holds it to that hash. It finds a program however the app was started (the setting, the override variable, the engine's PATH, the known install folders, then the person's shell PATH, read once), checks it with `--version` started as a tab starts it, and builds the child environment.

### app/services/claude_code.py · located, usable, said, terminal_env, a token per tab
- refers-to: app/services/claude_code.py :: OVERRIDES, SETTINGS, located, usable, said, child_env, terminal_env, tab_mcp_config, cli, _signed_in, info, argv, open_
- kind: rationale
- logged: v1.22.0 · 2026-11-20
`located(kind)` is the one lookup for all four programs: the `claudeBin` setting for Claude Code (the only path setting formsculpt has), `FSC_CLAUDE_BIN`, `FSC_CODEX_BIN`, `FSC_CURSOR_BIN`, `FSC_ANTIGRAVITY_BIN`, then the finder. `info` keeps its 60 s cache and answers the old fields plus the finder's (`finding`, `how`, `looked`, `said`), `install` (a command line, empty when it runs) and `signedIn` from `claude auth status`. Clause 11: a Claude Code tab got `--mcp-config` with the window's own `INAPP_TOKEN` written into the JSON on the command line, the same token for every tab and for Chat, alive for the whole service. Now each tab gets `agents.issue()`, passed only in `FSC_INAPP_TOKEN`; the config on the command line names the server `formsculpt_tab` with the header `${FSC_INAPP_TOKEN}`, and the token is revoked from `Session.on_close`. A missing or broken program is refused with 424 and the finder's sentence (clause 13 `notInstalled`), was 400 with the fixed `INSTALL` sentence; Chat keeps that sentence and 400. Removed code, quoted in full: `KEEP_ENV = {"CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_USE_VERTEX", "CLAUDE_CONFIG_DIR"}`, `CANDIDATES = (".claude/local/claude", ".local/bin/claude", ".npm-global/bin/claude", ".bun/bin/claude", ".volta/bin/claude")`, `SYSTEM_PATHS = ("/opt/homebrew/bin/claude", "/usr/local/bin/claude", "/usr/bin/claude")`; `child_env()` was `return {k: v for k, v in terminal.base_env().items() if not k.startswith("CLAUDE") or k in KEEP_ENV}`; `_from_login_shell()` ran `terminal.login_shell() + ["-c", "command -v claude"]` and took the last absolute line; `cli()` tried the setting, `FSC_CLAUDE_BIN` or `shutil.which("claude")`, the candidates, then the login shell; `info()` ran `[path, "--version"]` and answered `{"installed": bool(path), "path": path, "version": version}`; `argv()` was `[path, "--mcp-config", mcp_config(), "--allowedTools", "mcp__formsculpt", "--append-system-prompt", NOTE]` plus `--resume` and the prompt, wrapped as `terminal.login_shell() + ["-c", "exec " + shlex.join(cmd)]` off Windows; `open_` raised `CommandError(INSTALL, 400)` and passed `child_env()`.

### app/services/agent_programs.py · Codex, Cursor and Antigravity tabs
- refers-to: app/services/agent_programs.py :: PROGRAMS, TABS, report, codex_args, cursor_config, write_tab_entry, remove_tab_entry, open_terminal
- kind: rationale
- logged: v1.22.0 · 2026-11-20
A port of FormTxt's `agent_programs.py` onto the command bus. Every tab gets its own token in `FSC_INAPP_TOKEN` and the MCP address in `FSC_MCP_URL`, never on the command line. Codex reads the server from `-c mcp_servers.formsculpt_tab.url=...` and `env_http_headers` naming the variable; Cursor from `.cursor/mcp.json` in the tab's folder (`${env:...}` references, written once at 0600, every other entry kept); Antigravity has no variable form, so the entry `formsculpt_tab` with `serverUrl` and the header carrying the token goes into `.agents/mcp_config.json` of its own folder (`<data>/agent-tabs/antigravity`), replaced whole at 0600 with every other entry kept, and is taken out on close when it still carries that tab's token. Codex and Cursor start in the Claude Code folder (`workdir`). `report` caches the three for 60 s and asks Claude Code's own cache; the dock asks only while open.

### app/services/terminal.py, agents.py, routers · close hooks, kinds, tokens, the exit frame
- refers-to: app/services/terminal.py :: Session.on_close, describe, close, base_env; app/services/agents.py :: INAPP_VAR, TOKENS, issue, revoke, is_inapp; app/routers/mcp.py :: mcp; app/routers/terminal_socket.py :: attach; app/services/cmd_terminal.py :: close
- kind: rationale
- logged: v1.22.0 · 2026-11-20
`Session.on_close` runs when a tab closes (token revoke, Antigravity entry removal), and close now tells attached channels the program ended. `describe` adds `tip` (the folder). `base_env` drops `FSC_INAPP_TOKEN`, so a shell tab never inherits one (clause 11). The MCP route accepts the window's `INAPP_TOKEN` (Chat) or any live tab token through `agents.is_inapp`. `terminal.close` succeeds for an id that is already gone (clause 6); it answered 404. The socket now always sends the scrollback first, an empty string included (clause 7), and sends the exit event as a binary frame `{"type": "exit"}` so a program that prints that text is not taken for an exit. Removed code, quoted in full: `inapp = secrets.compare_digest(request.headers.get(agents.INAPP_HEADER, ""), agents.INAPP_TOKEN)`; in `attach`, `if backlog: await ws.send_text(backlog)`, `await ws.send_text(json.dumps({"type": "exit"}))` and `await ws.send_text(json.dumps({"type": "exit"}) if text is None else text)`; in `terminal.close`, `_session(id)` before `terminal.close(id)`.

### The contract, clause by clause
- refers-to: frontend/src/dock/CONTRACT.md
- kind: rationale
- logged: v1.22.0 · 2026-11-20
Met: 1 and 2 (the window-only command bus and the loopback-origin socket; no ticket needed), 3 to 9 (restart keeps the id and clears the screen; a slow reader's output is dropped past 4000 queued chunks), 10 (four programs, `signedIn` null for the three that have no status command), 11, 13 (403 refused from `LocalGuard`, 424 not installed, 404 "Unknown command" read as an older engine, 409 busy), 14 (no tabs on the browser engine), 16. Clause 12 does not apply: formsculpt keeps its own Chat (`chat.*` commands, `cmd_chat.py`) through `chat.pane`. Left out: 15, consent in the dock; formsculpt asks for outside agents in its own `AgentLink` prompt and the module's dialog is not used. The dock does not dock right here (`canDockRight` false): the three layouts (panel, tablet, ZUI) each place a `dock` grid row, not a column. No engine route became unused, so none was removed. Checked live on Linux with a booted service: a shell tab answered `dock-ok`; status listed the four programs; a stand-in `agy` on PATH was found, its tab wrote `.agents/mcp_config.json` at 0600 with the tab's token, a changing MCP tool ran with that token while the tab was open and was refused after it closed, and the entry was gone. Service suite: 228 passed, no test changed.
