"""LightBox's Project shelf: projects kept in the app's folder (FSC_DATA_DIR or ~/Formsculptor, under LightBox/Project),
each with a small picture rendered when it was saved, listed, saved and opened by name. As ZBrush's LightBox does,
a project opens with a double click in the window; this router only serves the files."""

import base64
import json
import re
import time
from pathlib import Path

from fastapi import APIRouter, HTTPException
from fastapi.concurrency import run_in_threadpool
from fastapi.responses import Response

from app.core import config
from app.services import engine

router = APIRouter()

NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9 _.-]{0,63}$")


def _folder() -> Path:
    path = config.data_dir() / "LightBox" / "Project"
    path.mkdir(parents=True, exist_ok=True)
    return path


def _checked(name: str) -> str:
    name = name.strip()
    if not NAME.match(name) or name.endswith("."):
        raise HTTPException(status_code=400, detail="a project name is 1 to 64 letters, digits, spaces, dots, dashes or underscores")
    return name


def _guard() -> None:
    if not engine.available():
        raise HTTPException(status_code=503, detail="the engine module is not built; run ./formsculptor engine build")


@router.get("/lightbox/projects")
def projects() -> dict:
    """The saved projects, newest first: name, bytes, when saved (seconds since the epoch) and whether a picture is kept."""
    items = []
    for f in _folder().glob("*.fsc"):
        stat = f.stat()
        items.append({"name": f.stem, "bytes": stat.st_size, "saved": stat.st_mtime, "preview": f.with_suffix(".png").is_file()})
    items.sort(key=lambda x: x["saved"], reverse=True)
    return {"folder": str(_folder()), "projects": items}


@router.get("/lightbox/projects/{name}/preview")
def preview(name: str) -> Response:
    path = _folder() / f"{_checked(name)}.png"
    if not path.is_file():
        raise HTTPException(status_code=404, detail="no picture for this project")
    return Response(path.read_bytes(), media_type="image/png", headers={"Cache-Control": "no-cache"})


def _save(name: str) -> dict:
    data = engine.save_bytes()
    folder = _folder()
    (folder / f"{name}.fsc").write_bytes(data)
    # The picture is the scene as the headless render sees it; a scene with nothing in it keeps no picture.
    reply = engine.dispatch("get_view", {"width": 160, "height": 160, "background": "transparent", "shading": "clay"})
    picture = folder / f"{name}.png"
    if reply.get("ok") and reply["result"].get("png_base64"):
        picture.write_bytes(base64.b64decode(reply["result"]["png_base64"]))
    elif picture.exists():
        picture.unlink()
    return {"name": name, "bytes": len(data), "saved": time.time(), "preview": picture.is_file()}


@router.post("/lightbox/projects/{name}")
async def save(name: str) -> dict:
    """Saves the scene as a LightBox project (replacing one of the same name) with its picture."""
    _guard()
    return await run_in_threadpool(_save, _checked(name))


@router.post("/lightbox/projects/{name}/open")
async def open_project(name: str) -> dict:
    """Opens a LightBox project, replacing the scene (the same as opening its .fsc file)."""
    _guard()
    path = _folder() / f"{_checked(name)}.fsc"
    if not path.is_file():
        raise HTTPException(status_code=404, detail=f"no project named {name}")
    reply = await run_in_threadpool(engine.open_bytes, path.read_bytes())
    if not reply.get("ok"):
        raise HTTPException(status_code=400, detail=json.dumps(reply.get("error")))
    return reply
