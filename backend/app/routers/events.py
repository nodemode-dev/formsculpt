import asyncio
import json

from fastapi import APIRouter, Request
from fastapi.responses import FileResponse, JSONResponse, StreamingResponse

from app.services import cmd_chat, events

router = APIRouter(tags=["events"])
IMAGE_TYPES = {".png": "image/png", ".jpg": "image/jpeg", ".webp": "image/webp"}


@router.get("/events")
async def stream(request: Request) -> StreamingResponse:
    queue = events.subscribe()

    async def gen():
        try:
            yield f"retry: 1500\ndata: {json.dumps({'type': 'hello'})}\n\n"
            while True:
                if await request.is_disconnected():
                    break
                try:
                    message = await asyncio.wait_for(queue.get(), timeout=15.0)
                except asyncio.TimeoutError:
                    yield ": ping\n\n"
                    continue
                yield f"data: {message}\n\n"
        finally:
            events.unsubscribe(queue)

    return StreamingResponse(gen(), media_type="text/event-stream", headers={"Cache-Control": "no-store", "X-Accel-Buffering": "no"})


@router.get("/events/recent")
def recent() -> dict:
    return {"events": [json.loads(m) for m in events.recent()]}


@router.get("/chat/image/{name}")
def chat_image(name: str):
    folder = cmd_chat.image_dir().resolve()
    path = (folder / name).resolve()
    if path.parent != folder or path.suffix not in IMAGE_TYPES or not path.is_file():
        return JSONResponse({"ok": False, "error": "No such chat image"}, status_code=404)
    return FileResponse(path, media_type=IMAGE_TYPES[path.suffix])
