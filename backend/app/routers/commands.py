import logging
from typing import Any

from fastapi import APIRouter, Body
from fastapi.responses import JSONResponse

from app.services.commands import REGISTRY, CommandError, catalogue, dispatch

router = APIRouter(tags=["commands"])
log = logging.getLogger("formsculptor.commands")


@router.get("/commands")
def list_commands(group: str | None = None) -> dict:
    items = catalogue()
    if group:
        items = [c for c in items if c["group"] == group]
    return {"commands": items, "count": len(items)}


@router.get("/commands/{cid}")
def describe(cid: str) -> Any:
    spec = REGISTRY.get(cid)
    if spec is None:
        return JSONResponse({"ok": False, "error": f"Unknown command: {cid}"}, status_code=404)
    return {"id": spec.id, "label": spec.label, "group": spec.group, "params": spec.params, "description": spec.description}


@router.post("/commands/{cid}")
def run(cid: str, params: dict | None = Body(default=None)) -> Any:
    try:
        result = dispatch(cid, params or {})
    except CommandError as exc:
        return JSONResponse({"ok": False, "command": cid, "error": str(exc)}, status_code=exc.status)
    except Exception as exc:
        log.exception("command %s failed", cid)
        return JSONResponse({"ok": False, "command": cid, "error": f"{type(exc).__name__}: {exc}"}, status_code=500)
    return {"ok": True, "command": cid, "result": result}
