from typing import Any

from fastapi import APIRouter, HTTPException, Response, WebSocket, WebSocketDisconnect
from starlette.concurrency import run_in_threadpool
from pydantic import BaseModel, Field

from app.services import engine

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
