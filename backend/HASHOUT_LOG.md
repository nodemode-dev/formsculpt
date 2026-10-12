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
