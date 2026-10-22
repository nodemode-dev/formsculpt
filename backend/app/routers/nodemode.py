"""Send to NodeMode from a standalone formsculpt. Embedded in NodeMode, the page hands the model to its host page
(frontend/src/host.ts); standalone, the engine sends it: the scene as GLB and a rendered preview PNG are uploaded
through NodeMode's POST /api/upload, then one POST /api/formsculpt/receive tells NodeMode what arrived so it can
place the model on its canvas. A NodeMode without that receiver still gets the uploads, and the reply says so. The
protocol follows formvis's /api/nodemode routes."""

import json
import urllib.error

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field
from starlette.concurrency import run_in_threadpool

from app.services import engine, outbound

router = APIRouter(prefix="/api/nodemode", tags=["nodemode"])
EXAMPLE = "http://localhost:8000"
WHERE = "the File section's NodeMode address"


class Send(BaseModel):
    target: str = Field(max_length=300)
    title: str = Field("formsculpt", max_length=120)
    shading: str = Field("clay", max_length=20)


def _base(target: str) -> str:
    return outbound.base(target, "NodeMode", EXAMPLE)


@router.get("/status")
def status(target: str) -> dict:
    """Whether NodeMode answers at `target`, and whether it has formsculpt's receiver."""
    base = _base(target)
    try:
        with outbound.request(base + "/", timeout=3):
            pass
    except urllib.error.HTTPError:
        pass  # it answered, with an error page: it is running
    except (urllib.error.URLError, OSError) as exc:
        return {"running": False, "receiver": False, "detail": str(outbound.unreachable("NodeMode", base, WHERE, exc).detail)}
    try:
        with outbound.request(base + "/api/formsculpt/receive", timeout=3):
            receiver = True
    except urllib.error.HTTPError as exc:
        receiver = exc.code == 405  # there, for POST only
    except (urllib.error.URLError, OSError):
        receiver = False
    return {"running": True, "receiver": receiver}


def _upload(base: str, name: str, blob: bytes, kind: str) -> str:
    body, content_type = outbound.multipart("file", name, blob, kind)
    with outbound.request(base + "/api/upload", body, {"Content-Type": content_type}, "POST") as r:
        reply = json.loads(r.read() or b"{}")
    url = reply.get("url") or reply.get("path")
    if not url:
        file_id = reply.get("file_id") or reply.get("id") or reply.get("filename")
        if not file_id:
            raise HTTPException(status_code=502, detail="NodeMode took the upload but did not say where it put it.")
        url = f"/api/files/{file_id}"
    return url if url.startswith("http") else base + (url if url.startswith("/") else "/" + url)


@router.post("/send")
async def send(body: Send) -> dict:
    """Exports the scene as GLB, renders a preview and sends both to NodeMode."""
    base = _base(body.target)
    if await run_in_threadpool(engine.object_count) == 0:
        raise HTTPException(status_code=422, detail="The scene is empty; there is nothing to send.")
    glb = await run_in_threadpool(engine.export_bytes, "scene.glb", None)
    try:
        png, facts = await run_in_threadpool(engine.view_png, {"width": 768, "height": 768, "shading": body.shading, "background": "transparent"})
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    stem = "".join(c if c.isalnum() or c in "-_ " else "_" for c in body.title)[:80].strip() or "formsculpt"
    try:
        model_url = _upload(base, f"{stem}.glb", glb, "model/gltf-binary")
        preview_url = _upload(base, f"{stem}.png", png, "image/png")
    except urllib.error.HTTPError as exc:
        raise HTTPException(status_code=502, detail=f"NodeMode refused the upload ({exc.code}).")
    except (urllib.error.URLError, OSError) as exc:
        raise outbound.unreachable("NodeMode", base, WHERE, exc)
    files = [
        {"url": model_url, "filename": f"{stem}.glb", "fileType": "model", "mime": "model/gltf-binary", "bytes": len(glb)},
        {"url": preview_url, "filename": f"{stem}.png", "fileType": "image", "kind": "preview", "width": facts["width"], "height": facts["height"], "previewOf": 0},
    ]
    announce = json.dumps({"source": "formsculpt", "title": body.title, "files": files}).encode()
    try:
        with outbound.request(base + "/api/formsculpt/receive", announce, {"Content-Type": "application/json"}, "POST"):
            placed = True
    except urllib.error.HTTPError as exc:
        if exc.code not in (404, 405):
            raise HTTPException(status_code=502, detail=f"NodeMode refused what was sent ({exc.code}).")
        placed = False
    except (urllib.error.URLError, OSError) as exc:
        raise outbound.unreachable("NodeMode", base, WHERE, exc)
    return {"placed": placed, "files": files,
            **({} if placed else {"detail": "Uploaded to NodeMode, but this NodeMode cannot place a model on its canvas yet: it needs formsculpt's receiver. The GLB and preview are in NodeMode's uploads."})}
