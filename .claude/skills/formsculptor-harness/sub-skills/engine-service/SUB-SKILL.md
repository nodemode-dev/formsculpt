---
name: engine-service
description: "Covers the formsculptor Python backend in backend/app: FastAPI on 127.0.0.1:8196, routers system, io, projects, jobs, assets, mcp and nodemode, the fsc-py PyO3 module built with maturin, and the PyInstaller sidecar entry backend/formsculptor_engine.py. Load for backend, API, job or freeze changes."
---

# Engine Service

## Purpose

Own `backend/app/`. Serves the built frontend, exposes the API and runs the engine headless. Hands back a backend change that boots under the dispatcher and freezes cleanly.

## Preconditions

State at 0.9.0: routers `system`, `engine` (dispatch, mesh, stroke socket, import, export, project, maps, autosave, and `/view`, a PNG from the headless `get_view`), `nodemode` (status and send: the scene as GLB plus a rendered preview uploaded through NodeMode's `/api/upload`, then announced to `/api/formsculptor/receive`; formvis's protocol) and `mcp`. Outbound requests go through `services/outbound.py` (http or https only, as given). `__version__` lives in `backend/app/__init__.py` and is one of the bump sites. At 0.18.0 `services/scripting.py` (`Fsc`, `run_file`, `list_plugins`) serves both `./formsculptor script` (`setup/run_script.py`, HTTP to a running engine or an in-process one) and the `plugins` router (`/api/plugins`, `/api/plugins/run`); scripts and plugins reach the engine only through registry commands, so they never need their own engine API. Keep `Fsc.call`'s command positional-only. At 0.23.0 plugins declare `PARAMS` (window fields) and `CLICK` (a surface tool) as top-level literals, read with `ast.literal_eval` so listing never runs a plugin; keep it that way. `POST /api/engine/bridge` names bridge files in `FSC_BRIDGE_DIR` or `~/.formsculptor/bridge`. Tests live in `backend/tests`; their `call` helper is positional-only for the same reason as `Fsc.call`.

## Procedure

1. Bind 127.0.0.1:8196 only. Routers: system, io, projects, jobs, assets, mcp, nodemode.
2. Heavy engine work (remesh, bake, import of large files) runs as a job with progress, not inside a request.
3. Headless engine calls go through the `fsc-py` module, built with maturin. Rebuild it after any Rust change you depend on.
4. `backend/formsculptor_engine.py` is the PyInstaller sidecar entry. The desktop shell starts it with port 0 and reads the uvicorn port line from its output; do not change that line format without updating `sub-skills/desktop-shell/SUB-SKILL.md`.
5. The mcp router implements the contract in `sub-skills/agent-substrate/SUB-SKILL.md`.
6. Rationale goes to `backend/HASHOUT_LOG.md`.
7. Tasks run through the root dispatcher: `./formsculptor backend`, `serve`, `engine build`, `freeze`, `doctor`.

## Gate

Done when `./formsculptor doctor` is clean, the backend starts, and the changed route answers on 127.0.0.1:8196.

## Hands back

The change and the route or job evidence.

## Owned resources

None bundled.
