"""GoF's hop to FormSurface. The engine's gof_send writes the files and gof.json into the GoF folder (FORMSCULPTOR_GOF,
set here to GoF under the app's folder when not set, so the engine and this router agree); this router hands a GLB from
there to a running FormSurface: its service reports its workspace at /api/health, its load_mesh reads the workspace's
meshes folder, and /api/agent runs load_mesh in the open editor once Agent link is on there."""

import json
import os
import shutil
import urllib.error
import urllib.request
from pathlib import Path

from fastapi import APIRouter, HTTPException
from fastapi.concurrency import run_in_threadpool
from pydantic import BaseModel

from app.core import config

router = APIRouter()

os.environ.setdefault("FORMSCULPTOR_GOF", str(config.data_dir() / "GoF"))

# FormSurface's own default service address; FORMSURFACE_SERVICE overrides it, as in FormSurface's MCP server.
_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def _service() -> str:
    return os.environ.get("FORMSURFACE_SERVICE", "http://127.0.0.1:8766").rstrip("/")


def _folder() -> Path:
    return Path(os.environ["FORMSCULPTOR_GOF"])


def _call(method: str, path: str, body: dict | None = None, timeout: float = 10) -> dict:
    data = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(_service() + path, data=data, method=method, headers={"Content-Type": "application/json"})
    with _OPENER.open(request, timeout=timeout) as reply:
        return json.loads(reply.read().decode() or "{}")


def _health() -> dict | None:
    try:
        return _call("GET", "/api/health", timeout=3)
    except (urllib.error.URLError, OSError, ValueError):
        return None


class Hand(BaseModel):
    file: str


@router.get("/gof")
def status() -> dict:
    """The GoF folder, its manifest, and whether FormSurface is running and its editor linked."""
    folder = _folder()
    manifest = folder / "gof.json"
    objects = []
    if manifest.is_file():
        try:
            objects = json.loads(manifest.read_text(encoding="utf-8")).get("objects", [])
        except ValueError:
            objects = []
    health = _health()
    return {
        "folder": str(folder),
        "objects": objects,
        "formsurface": None if health is None else {"version": health.get("version"), "linked": bool(health.get("editor_linked"))},
    }


def _hand(file: str) -> dict:
    source = _folder() / file
    if Path(file).name != file or source.suffix.lower() != ".glb" or not source.is_file():
        raise HTTPException(status_code=404, detail=f"send it with GoF for FormSurface first; {file} is not a GLB in the GoF folder")
    health = _health()
    if health is None:
        # Not running is an ordinary state, not a failure: the file waits in the GoF folder.
        return {"copied": None, "loaded": False, "running": False}
    meshes = Path(str(health.get("data", ""))) / "meshes"
    try:
        meshes.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, meshes / file)
    except OSError as error:
        raise HTTPException(status_code=500, detail=f"could not copy into FormSurface's meshes folder: {error}") from error
    if not health.get("editor_linked"):
        return {"copied": str(meshes / file), "loaded": False, "running": True, "note": "Turn on Agent link in FormSurface to load it there, or open it from its meshes folder"}
    try:
        # reproject keeps FormSurface's painting on the surface when a sculpt comes back changed.
        _call("POST", "/api/agent", {"command": "load_mesh", "args": {"file": file, "reproject": True}, "timeout": 180}, timeout=185)
    except urllib.error.HTTPError as error:
        raise HTTPException(status_code=502, detail=f"FormSurface refused it: {error.read().decode(errors='replace')[:300]}") from error
    except (urllib.error.URLError, OSError) as error:
        raise HTTPException(status_code=502, detail=f"FormSurface did not answer: {error}") from error
    return {"copied": str(meshes / file), "loaded": True, "running": True}


@router.post("/gof/formsurface")
async def formsurface(body: Hand) -> dict:
    """Copy a GLB from the GoF folder into FormSurface's meshes folder and load it in its editor when linked."""
    return await run_in_threadpool(_hand, body.file)
