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
def mesh(object_id: int) -> Response:
    _guard()
    try:
        payload = engine.mesh_payload(object_id)
    except KeyError:
        raise HTTPException(status_code=404, detail=f"no object {object_id}")
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
async def export_file(format: str = Query("glb", pattern="^(obj|stl|ply|glb|3mf)$"), ids: str | None = None) -> Response:
    _guard()
    chosen = [int(x) for x in ids.split(",") if x.strip()] if ids else None
    try:
        data = await run_in_threadpool(engine.export_bytes, f"export.{format}", chosen)
    except (KeyError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc).strip("'\""))
    media = {"glb": "model/gltf-binary", "stl": "model/stl", "obj": "model/obj", "ply": "application/octet-stream", "3mf": "model/3mf"}[format]
    return Response(content=data, media_type=media, headers={"Content-Disposition": f'attachment; filename="formsculptor.{format}"', "Cache-Control": "no-store"})


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
                    hit, update, topology = await run_in_threadpool(engine.stroke_sample, _vec(message["origin"]), _vec(message["dir"]), float(message.get("pressure", 1.0)))
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
