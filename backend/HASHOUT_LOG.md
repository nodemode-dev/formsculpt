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
