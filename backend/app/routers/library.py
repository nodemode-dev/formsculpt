"""The Home screen's library routes: sculpts (list, create, open, save, duplicate, move to the Trash, thumbnail), the
Trash (list, restore, delete permanently, empty), the starting meshes and Home's pinned and hidden lists. The files
are LightBox's Project folder, see app/services/library.py. A sculpt's id is its file name, so the Trash, templates
and settings have their own paths instead of living under /sculpts, where a sculpt of that name would shadow them."""

from fastapi import APIRouter, HTTPException, Request
from fastapi.concurrency import run_in_threadpool
from fastapi.responses import Response
from pydantic import BaseModel

from app.services import library

router = APIRouter()


class NewSculpt(BaseModel):
    title: str | None = None
    template: str | None = None
    fromScene: bool = False
    thumb: str = "render"


class HomeSettings(BaseModel):
    pinned: list[str] | None = None
    recentHidden: list[str] | None = None


async def _run(fn, *args):
    try:
        return await run_in_threadpool(fn, *args)
    except library.LibraryError as err:
        raise HTTPException(status_code=err.status, detail=err.detail) from None
    except OSError as err:
        raise HTTPException(status_code=500, detail=f"the library folder could not be written: {err.strerror or err}") from None


@router.get("/sculpts")
async def sculpts() -> dict:
    """Every sculpt in the library, the most recently saved or opened first."""
    return {"folder": str(library.folder()), "sculpts": await _run(library.sculpts)}


@router.post("/sculpts")
async def create(body: NewSculpt) -> dict:
    """A new sculpt in the library: from a starting mesh (built apart from the scene on screen), or with fromScene
    the scene as it is now (thumb=client leaves the 16:9 thumbnail for the window to send). The name is made unique;
    the summary carries the id it got."""
    if body.fromScene:
        return await _run(library.create_from_scene, body.title, body.thumb != "client")
    return await _run(library.create, body.title, body.template)


@router.post("/sculpts/{sculpt_id}/open")
async def open_sculpt(sculpt_id: str) -> dict:
    """Opens a sculpt, replacing the scene, and notes it as recently opened."""
    return await _run(library.open_sculpt, sculpt_id)


@router.post("/sculpts/{sculpt_id}/save")
async def save(sculpt_id: str, thumb: str = "render") -> dict:
    """Writes the scene to the sculpt of this name with LightBox's picture; thumb=client leaves the 16:9 thumbnail
    for the window to send."""
    return await _run(library.save, sculpt_id, thumb != "client")


@router.post("/sculpts/{sculpt_id}/duplicate")
async def duplicate(sculpt_id: str) -> dict:
    return await _run(library.duplicate, sculpt_id)


@router.delete("/sculpts/{sculpt_id}")
async def remove(sculpt_id: str) -> dict:
    """Moves a sculpt and its pictures into the Trash."""
    return await _run(library.remove, sculpt_id)


@router.put("/sculpts/{sculpt_id}/thumb")
async def put_thumb(sculpt_id: str, request: Request) -> dict:
    declared = request.headers.get("content-length", "")
    if declared.isdigit() and int(declared) > library.THUMB_MAX:
        raise HTTPException(status_code=413, detail="a thumbnail is at most 2 MB")
    data = bytearray()
    async for chunk in request.stream():
        data.extend(chunk)
        if len(data) > library.THUMB_MAX:
            raise HTTPException(status_code=413, detail="a thumbnail is at most 2 MB")
    return await _run(library.put_thumb, sculpt_id, bytes(data))


@router.get("/sculpts/{sculpt_id}/thumb")
async def thumb(sculpt_id: str) -> Response:
    data, kind = await _run(library.thumb, sculpt_id)
    return Response(data, media_type=kind, headers={"Cache-Control": "no-cache"})


@router.get("/sculpt-trash")
async def trash() -> dict:
    return {"sculpts": await _run(library.trash)}


@router.post("/sculpt-trash/{entry}/restore")
async def restore(entry: str) -> dict:
    return await _run(library.restore, entry)


@router.delete("/sculpt-trash/{entry}")
async def purge(entry: str) -> dict:
    return await _run(library.purge, entry)


@router.delete("/sculpt-trash")
async def empty_trash() -> dict:
    return await _run(library.empty_trash)


@router.get("/sculpt-templates")
async def templates() -> dict:
    return await _run(library.templates)


@router.get("/sculpt-templates/{template_id}/thumb")
async def template_thumb(template_id: str) -> Response:
    data = await _run(library.template_picture, template_id)
    return Response(data, media_type="image/png", headers={"Cache-Control": "max-age=3600"})


@router.get("/sculpt-settings")
async def settings() -> dict:
    return await _run(library.settings)


@router.post("/sculpt-settings")
async def update_settings(body: HomeSettings) -> dict:
    return await _run(library.update_settings, body.model_dump(exclude_none=True))
