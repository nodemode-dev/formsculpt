import json
import os
import tempfile
from typing import Any

from fastapi import APIRouter, HTTPException, Query, Response, UploadFile, WebSocket, WebSocketDisconnect
from starlette.concurrency import run_in_threadpool
from pydantic import BaseModel, Field

from app.services import autosave, engine

router = APIRouter()


class Call(BaseModel):
    name: str = Field(max_length=120)
    args: dict[str, Any] = Field(default_factory=dict)


def _guard() -> None:
    if not engine.available():
        raise HTTPException(status_code=503, detail="the engine module is not built; run ./formsculptor engine build")


@router.get("/catalogue")
def catalogue() -> dict:
    _guard()
    return engine.catalogue()


@router.post("/dispatch")
def dispatch(call: Call) -> dict:
    _guard()
    return engine.dispatch(call.name, call.args)


@router.get("/mesh/{object_id}")
def mesh(object_id: int, smooth: int = Query(0, ge=0, le=3)) -> Response:
    _guard()
    try:
        payload = engine.mesh_payload(object_id, smooth)
    except KeyError:
        # The object went between the window listing the scene and asking for its mesh (an agent cleared or
        # replaced the scene, a step was undone): an ordinary race, not a failed request, so the browser logs no
        # error. The window draws nothing for it and the next listing settles the scene.
        return Response(status_code=204, headers={"Cache-Control": "no-store", "X-Formsculptor-Missing": str(object_id)})
    return Response(content=payload, media_type="application/octet-stream", headers={"Cache-Control": "no-store"})


@router.get("/boolean_preview")
def boolean_preview(a: int, b: int, op: str = Query("union", pattern="^(union|difference|intersection)$")) -> Response:
    """Live Boolean: the exact result of a op b for the window to draw in their place, without changing the scene."""
    _guard()
    try:
        payload = engine.boolean_preview(a, b, op)
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=str(exc).strip("'\""))
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    return Response(content=payload, media_type="application/octet-stream", headers={"Cache-Control": "no-store"})


MAX_UPLOAD = 1 << 30


async def _read(upload: UploadFile) -> bytes:
    data = await upload.read(MAX_UPLOAD + 1)
    if len(data) > MAX_UPLOAD:
        raise HTTPException(status_code=413, detail="the file is over 1 GB")
    return data


@router.post("/import")
async def import_file(file: UploadFile) -> dict:
    _guard()
    data = await _read(file)
    name = file.filename or "upload"
    if name.lower().endswith(".fsc"):
        return await run_in_threadpool(engine.open_bytes, data)
    return await run_in_threadpool(engine.import_bytes, name, data)


@router.get("/export")
async def export_file(format: str = Query("glb", pattern="^(obj|stl|ply|glb|3mf|fbx|usda)$"), ids: str | None = None) -> Response:
    _guard()
    chosen = [int(x) for x in ids.split(",") if x.strip()] if ids else None
    try:
        data = await run_in_threadpool(engine.export_bytes, f"export.{format}", chosen)
    except (KeyError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc).strip("'\""))
    media = {"glb": "model/gltf-binary", "stl": "model/stl", "obj": "model/obj", "ply": "application/octet-stream", "3mf": "model/3mf", "fbx": "application/octet-stream", "usda": "model/vnd.usda"}[format]
    return Response(content=data, media_type=media, headers={"Content-Disposition": f'attachment; filename="formsculptor.{format}"', "Cache-Control": "no-store"})


@router.get("/view")
async def view_image(view: str = "three_quarter", width: int = Query(640, ge=64, le=2048), height: int = Query(640, ge=64, le=2048), shading: str = "clay", background: str = "dark", render: str = Query("preview", pattern="^(preview|path)$"), samples: int = Query(32, ge=1, le=1024), bounces: int = Query(4, ge=1, le=8), pass_: str = Query("beauty", alias="pass", pattern="^(beauty|depth|normal|ao)$"), aperture: float = Query(0.0, ge=0.0, le=1.0)) -> Response:
    """The scene rendered headless (the get_view command) as a PNG, for previews and for anything that wants a picture;
    render=path is the path-traced production render, with pass (depth, normal, ao) and aperture (depth of field)."""
    _guard()
    args = {"view": view, "width": width, "height": height, "shading": shading, "background": background, "render": render, "samples": samples, "bounces": bounces}
    if render == "path":
        args.update({"pass": pass_, "aperture": aperture})
    try:
        data, _ = await run_in_threadpool(engine.view_png, args)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    return Response(content=data, media_type="image/png", headers={"Cache-Control": "no-store"})


@router.get("/turntable")
async def turntable(frames: int = Query(48, ge=8, le=180), size: int = Query(384, ge=64, le=1024), shading: str = "clay", background: str = "dark") -> Response:
    """A looping animated PNG of the scene turning once (record_turntable, written to a temporary file and returned)."""
    _guard()
    fd, path = tempfile.mkstemp(suffix=".png")
    os.close(fd)
    try:
        reply = await run_in_threadpool(engine.dispatch, "record_turntable", {"path": path, "frames": frames, "size": size, "shading": shading, "background": background})
        if not reply.get("ok"):
            raise HTTPException(status_code=400, detail=(reply.get("error") or {}).get("message", "render failed"))
        with open(path, "rb") as f:
            data = f.read()
    finally:
        os.unlink(path)
    return Response(content=data, media_type="image/png", headers={"Content-Disposition": 'attachment; filename="turntable.png"', "Cache-Control": "no-store"})


class BridgeOut(BaseModel):
    id: int = Field(ge=1)
    format: str = Field("obj", pattern="^(obj|glb|fbx|ply|stl)$")


@router.post("/bridge")
def bridge(body: BridgeOut) -> dict:
    """Sends an object to a file in the bridge folder (FSC_BRIDGE_DIR, or ~/.formsculptor/bridge) for another app to
    open; bridge_sync brings that app's saves back. Returns the file's path to open there."""
    _guard()
    folder = os.environ.get("FSC_BRIDGE_DIR") or os.path.join(os.path.expanduser("~"), ".formsculptor", "bridge")
    os.makedirs(folder, exist_ok=True)
    scene = engine.dispatch("get_scene", {})
    found = next((o for o in scene.get("result", {}).get("objects", []) if o["id"] == body.id), None)
    if found is None:
        raise HTTPException(status_code=404, detail=f"no object {body.id}")
    stem = "".join(ch if ch.isalnum() or ch in "-_" else "_" for ch in found["name"]).strip("_") or "object"
    path = os.path.join(folder, f"{stem}-{body.id}.{body.format}")
    return engine.dispatch("bridge_send", {"id": body.id, "path": path})


class SubstanceOut(BaseModel):
    id: int = Field(ge=1)


@router.post("/substance")
def substance(body: SubstanceOut) -> dict:
    """Writes a Substance 3D Painter package for an object to FSC_SUBSTANCE_DIR or ~/.formsculptor/substance/<name>
    (send_to_substance) and returns its folder and files."""
    _guard()
    root = os.environ.get("FSC_SUBSTANCE_DIR") or os.path.join(os.path.expanduser("~"), ".formsculptor", "substance")
    scene = engine.dispatch("get_scene", {})
    found = next((o for o in scene.get("result", {}).get("objects", []) if o["id"] == body.id), None)
    if found is None:
        raise HTTPException(status_code=404, detail=f"no object {body.id}")
    stem = "".join(ch if ch.isalnum() or ch in "-_" else "_" for ch in found["name"]).strip("_") or "object"
    return engine.dispatch("send_to_substance", {"id": body.id, "folder": os.path.join(root, stem)})


@router.post("/substance/sync")
def substance_sync(body: SubstanceOut) -> dict:
    """Reads the textures Painter exported back into the object's package folder (substance_sync): BaseColor, and
    Roughness and Metallic when there, onto the vertex paint."""
    _guard()
    root = os.environ.get("FSC_SUBSTANCE_DIR") or os.path.join(os.path.expanduser("~"), ".formsculptor", "substance")
    scene = engine.dispatch("get_scene", {})
    found = next((o for o in scene.get("result", {}).get("objects", []) if o["id"] == body.id), None)
    if found is None:
        raise HTTPException(status_code=404, detail=f"no object {body.id}")
    stem = "".join(ch if ch.isalnum() or ch in "-_" else "_" for ch in found["name"]).strip("_") or "object"
    return engine.dispatch("substance_sync", {"id": body.id, "folder": os.path.join(root, stem)})


@router.get("/animation")
async def animation(keys: str = Query(..., max_length=20000), fps: int = Query(24, ge=1, le=60), size: int = Query(384, ge=64, le=1024), shading: str = "clay", background: str = "dark") -> Response:
    """A looping animated PNG of keyframed camera moves and layer strengths (record_animation; keys is its JSON list)."""
    _guard()
    try:
        parsed = json.loads(keys)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=f"keys is not JSON: {exc}") from exc
    fd, path = tempfile.mkstemp(suffix=".png")
    os.close(fd)
    try:
        reply = await run_in_threadpool(engine.dispatch, "record_animation", {"path": path, "keys": parsed, "fps": fps, "size": size, "shading": shading, "background": background})
        if not reply.get("ok"):
            raise HTTPException(status_code=400, detail=(reply.get("error") or {}).get("message", "render failed"))
        with open(path, "rb") as f:
            data = f.read()
    finally:
        os.unlink(path)
    return Response(content=data, media_type="image/png", headers={"Content-Disposition": 'attachment; filename="animation.png"', "Cache-Control": "no-store"})


@router.get("/history_movie")
async def history_movie(fps: int = Query(4, ge=1, le=30), size: int = Query(384, ge=64, le=1024), turn: float = Query(0, ge=-720, le=720), shading: str = "clay", background: str = "dark", camera: str = Query("fixed", pattern="^(fixed|steps)$")) -> Response:
    """A looping animated PNG of the model at every undo step from the start (record_history); camera=steps looks at
    each step from the direction it was made from."""
    _guard()
    fd, path = tempfile.mkstemp(suffix=".png")
    os.close(fd)
    try:
        reply = await run_in_threadpool(engine.dispatch, "record_history", {"path": path, "fps": fps, "size": size, "turn": turn, "shading": shading, "background": background, "camera": camera})
        if not reply.get("ok"):
            raise HTTPException(status_code=400, detail=(reply.get("error") or {}).get("message", "render failed"))
        with open(path, "rb") as f:
            data = f.read()
    finally:
        os.unlink(path)
    return Response(content=data, media_type="image/png", headers={"Content-Disposition": 'attachment; filename="history.png"', "Cache-Control": "no-store"})


@router.get("/maps/{object_id}")
async def maps_file(object_id: int) -> Response:
    _guard()
    try:
        data = await run_in_threadpool(engine.maps_zip, object_id)
    except KeyError:
        raise HTTPException(status_code=404, detail=f"no object {object_id}")
    if data is None:
        raise HTTPException(status_code=404, detail="the object has no baked maps; run bake_maps first")
    return Response(content=data, media_type="application/zip", headers={"Content-Disposition": 'attachment; filename="maps.zip"', "Cache-Control": "no-store"})


@router.get("/project")
async def project_file() -> Response:
    _guard()
    data = await run_in_threadpool(engine.save_bytes)
    return Response(content=data, media_type="application/x-formsculptor", headers={"Content-Disposition": 'attachment; filename="scene.fsc"', "Cache-Control": "no-store"})


@router.get("/autosave")
def autosave_info() -> dict:
    return autosave.info()


@router.post("/autosave/recover")
async def autosave_recover() -> dict:
    _guard()
    return await run_in_threadpool(autosave.recover)


def _vec(value: Any) -> list[float]:
    if not isinstance(value, list) or len(value) != 3:
        raise ValueError("expected three numbers")
    return [float(v) for v in value]


@router.websocket("/stroke")
async def stroke(socket: WebSocket) -> None:
    await socket.accept()
    if not engine.available():
        await socket.send_json({"type": "error", "message": "the engine module is not built; run ./formsculptor engine build"})
        await socket.close()
        return
    active = False
    try:
        while True:
            message = await socket.receive_json()
            kind = message.get("type")
            try:
                if kind == "begin":
                    await run_in_threadpool(engine.stroke_begin, int(message["id"]), dict(message.get("brush") or {}))
                    active = True
                elif kind == "sample" and active:
                    hit, update, topology = await run_in_threadpool(engine.stroke_sample, _vec(message["origin"]), _vec(message["dir"]), float(message.get("pressure", 1.0)), float(message.get("tilt", 0.0)))
                    await socket.send_json({"type": "sample", "seq": message.get("seq"), "hit": hit, "topology": topology})
                    if len(update) > 16:
                        await socket.send_bytes(update)
                elif kind == "end" and active:
                    active = False
                    result = await run_in_threadpool(engine.stroke_end)
                    await socket.send_json({"type": "end", "result": result})
            except (KeyError, ValueError, TypeError) as exc:
                await socket.send_json({"type": "error", "message": str(exc)})
    except WebSocketDisconnect:
        if active:
            await run_in_threadpool(engine.stroke_end)
