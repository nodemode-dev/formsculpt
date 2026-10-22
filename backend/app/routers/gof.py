"""GoF's hop to FormSurface. The engine's gof_send writes the files and gof.json into the GoF folder (FORMSCULPT_GOF,
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
from app.services import engine

router = APIRouter()

os.environ.setdefault("FORMSCULPT_GOF", str(config.data_dir() / "GoF"))

# FormSurface's own default service address; FORMSURFACE_SERVICE overrides it, as in FormSurface's MCP server.
_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def _service() -> str:
    return os.environ.get("FORMSURFACE_SERVICE", "http://127.0.0.1:8766").rstrip("/")


def _folder() -> Path:
    return Path(os.environ["FORMSCULPT_GOF"])


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


class Back(BaseModel):
    id: int


def _stem(name: str) -> str:
    # gof_send's and substance_sync's stem: letters and digits kept, anything else an underscore.
    return "".join(c if c.isascii() and c.isalnum() else "_" for c in name)


def _back(object_id: int) -> dict:
    if not engine.available():
        raise HTTPException(status_code=503, detail="the engine module is not built; run ./formsculpt engine build")
    scene = engine.dispatch("get_scene", {})
    found = [o for o in scene.get("result", {}).get("objects", []) if o.get("id") == object_id]
    if not found:
        raise HTTPException(status_code=404, detail=f"no object {object_id}")
    health = _health()
    if health is None:
        return {"running": False, "painted": False}
    stem = _stem(found[0]["name"])
    folder = Path(str(health.get("data", ""))) / "exports" / stem
    if not (folder / f"{stem}_BaseColor.png").is_file():
        # Nothing exported yet is an ordinary state, as FormSurface not running is.
        return {"running": True, "painted": False, "missing": f"{stem}_BaseColor.png", "folder": str(folder)}
    # FormSurface stores a mesh's V as 1 - v whatever the file, and paints with that V down the texture, so its maps
    # sit where substance_sync samples them (1 - v), and its Blender template writes the names substance_sync reads.
    reply = engine.dispatch("substance_sync", {"id": object_id, "folder": str(folder)})
    if not reply.get("ok"):
        error = reply.get("error") or {}
        raise HTTPException(status_code=400, detail=error.get("message") or "the maps could not be read")
    return {"running": True, "painted": True, "folder": str(folder), "result": reply.get("result")}


@router.post("/gof/formsurface/back")
async def formsurface_back(body: Back) -> dict:
    """Bring FormSurface's painted maps for an object back into its paint, from FormSurface's exports folder."""
    return await run_in_threadpool(_back, body.id)
