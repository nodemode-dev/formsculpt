from typing import Any

from fastapi import APIRouter, HTTPException, Response
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
