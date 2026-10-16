import json
import secrets
import urllib.parse

from fastapi import APIRouter, Request, Response
from pydantic import BaseModel
from starlette.concurrency import run_in_threadpool

from app import __version__
from app.services import agents, engine, events

router = APIRouter()
VERSIONS = ["2026-07-28", "2025-11-25"]
CACHE_HINTS = {"ttlMs": 300_000, "cacheScope": "public"}
INSTRUCTIONS = (
    "formsculptor is a 3D sculpting app running on this computer; these tools change the scene its engine holds, which the "
    "formsculptor window shows live when it is open and which works without it. Start with get_scene for object ids, sizes and "
    "bounds. Coordinates are object space; a default sphere has radius 1, so brush radii around 0.05 to 0.3 suit it. Every change "
    "is one undoable step (undo). For the order of work, get the sculpt_strategy prompt."
)
STRATEGY = """How to sculpt in formsculptor with these tools.

1. Look first. get_scene lists every object with its id, vertex and triangle counts and bounds; read the bounds before choosing a brush radius (about a tenth of the object's size is a good start).
2. Start from a primitive. add_primitive with kind sphere (detail 6 or 7 for sculpting, 81,920 or 327,680 triangles), cube, cylinder, cone, torus or plane, and size for its radius or edge.
3. Block out big forms with grab (two points: where to grab and where to move it), inflate and draw with a large radius and moderate strength.
4. Refine with clay and flatten for planes, crease for folds and lines, pinch to sharpen edges, smooth to relax noise. Build a stroke from points spaced about radius * 0.15 apart along the surface; points can carry a fourth number, pressure 0 to 1.
5. Protect areas with the mask: sculpt_stroke with brush mask paints it, mask_invert flips it, mask_blur softens its edge, mask_clear removes it. Masked vertices do not move under other brushes.
6. Use symmetry for anything bilateral: symmetry {x: true} mirrors across the x = 0 plane; radial repeats around the y axis.
7. Check your work with get_scene (bounds) after each step, and undo a step that went wrong rather than trying to repair it with more strokes.
"""
PROMPTS = [{"name": "sculpt_strategy", "title": "How to sculpt in formsculptor", "description": "The order of work for formsculptor's tools: looking, primitives, blocking out, refining, masking, symmetry and checking.", "arguments": []}]


def _local_origin(origin: str | None) -> bool:
    if not origin:
        return True
    host = urllib.parse.urlparse(origin).hostname or ""
    return host in ("localhost", "127.0.0.1", "::1", "tauri.localhost") or origin.startswith("tauri://")


def _ok(req_id, result: dict, modern: bool, cacheable: bool = False) -> dict:
    extra = {"resultType": "complete", **(CACHE_HINTS if cacheable else {})} if modern else {}
    return {"jsonrpc": "2.0", "id": req_id, "result": {**result, **extra}}


def _err(req_id, code: int, message: str) -> dict:
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": message}}


def tools() -> list[dict]:
    out = []
    for c in engine.catalogue()["commands"]:
        out.append({"name": c["name"], "title": c["label"], "description": c["description"], "inputSchema": c["inputSchema"], "annotations": c["annotations"]})
    return out


def _text(payload: dict, error: bool = False) -> dict:
    out = {"content": [{"type": "text", "text": json.dumps(payload)}], "structuredContent": payload}
    if error:
        out["isError"] = True
    return out


async def call_tool(name: str, args: dict, inapp: bool = False) -> dict:
    if not engine.read_only(name):
        refusal = await agents.permit(name, inapp)
        if refusal:
            return {"content": [{"type": "text", "text": refusal}], "isError": True}
    reply = await run_in_threadpool(engine.dispatch, name, args)
    if reply.get("ok"):
        result = reply["result"]
        if isinstance(result, dict) and "png_base64" in result:
            # A render comes back as an image the agent can look at, with the facts beside it as text.
            facts = {k: v for k, v in result.items() if k != "png_base64"}
            return {"content": [{"type": "image", "data": result["png_base64"], "mimeType": result.get("mime", "image/png")}, {"type": "text", "text": json.dumps(facts)}], "structuredContent": facts}
        return _text(result)
    error = reply.get("error") or {}
    return _text({"error": error.get("code"), "message": error.get("message")}, error=True)


@router.post("/mcp")
async def mcp(request: Request) -> Response:
    if not _local_origin(request.headers.get("origin")):
        return Response(status_code=403, content=json.dumps(_err(None, -32600, "Origin not allowed")), media_type="application/json")
    try:
        msg = json.loads(await request.body())
    except ValueError:
        return Response(content=json.dumps(_err(None, -32700, "Parse error")), media_type="application/json", status_code=400)
    if isinstance(msg, list):
        return Response(content=json.dumps(_err(None, -32600, "Batches are not supported")), media_type="application/json", status_code=400)
    if not isinstance(msg, dict):
        return Response(content=json.dumps(_err(None, -32600, "Invalid request")), media_type="application/json", status_code=400)
    req_id, method, params = msg.get("id"), msg.get("method"), msg.get("params") or {}
    if req_id is None:
        return Response(status_code=202)
    meta = params.get("_meta") or {}
    version = meta.get("io.modelcontextprotocol/protocolVersion") or request.headers.get("mcp-protocol-version") or "2025-11-25"
    modern = version == "2026-07-28"
    server = {"name": "formsculptor", "version": __version__}
    if not engine.available() and method in ("tools/list", "tools/call"):
        out = _err(req_id, -32000, "The formsculptor engine module is not built; run ./formsculptor engine build in the formsculptor folder.")
    elif version not in VERSIONS and method != "initialize":
        out = _err(req_id, -32602, f"Unsupported protocol version {version}; supported: {', '.join(VERSIONS)}")
    elif method == "server/discover":
        out = _ok(req_id, {"supportedVersions": VERSIONS, "capabilities": {"tools": {}, "prompts": {}}, "_meta": {"io.modelcontextprotocol/serverInfo": server}, "instructions": INSTRUCTIONS}, True, cacheable=True)
    elif method == "initialize":
        asked = params.get("protocolVersion")
        out = _ok(req_id, {"protocolVersion": asked if asked in VERSIONS else "2025-11-25", "capabilities": {"tools": {}, "prompts": {}}, "serverInfo": server, "instructions": INSTRUCTIONS}, False)
    elif method == "ping":
        out = _ok(req_id, {}, modern)
    elif method == "prompts/list":
        out = _ok(req_id, {"prompts": PROMPTS}, modern, cacheable=True)
    elif method == "prompts/get":
        out = _ok(req_id, {"description": PROMPTS[0]["description"], "messages": [{"role": "user", "content": {"type": "text", "text": STRATEGY}}]}, modern) if params.get("name") == "sculpt_strategy" else _err(req_id, -32602, f"No prompt named {params.get('name')}")
    elif method == "tools/list":
        out = _ok(req_id, {"tools": tools()}, modern, cacheable=True)
    elif method == "tools/call":
        name, args = params.get("name"), params.get("arguments") or {}
        names = {t["name"] for t in tools()}
        inapp = secrets.compare_digest(request.headers.get(agents.INAPP_HEADER, ""), agents.INAPP_TOKEN)
        if inapp:
            events.publish("agent.activity", tool=name)
        out = _ok(req_id, await call_tool(name, args, inapp), modern) if name in names and isinstance(args, dict) else _err(req_id, -32602, f"No tool named {name}")
    else:
        out = _err(req_id, -32601, f"Method not found: {method}")
    return Response(content=json.dumps(out), media_type="application/json")


class Consent(BaseModel):
    allow: bool


@router.get("/api/mcp/status")
def status() -> dict:
    return {"endpoint": "/mcp", "engine": engine.available(), "tools": len(tools()) if engine.available() else 0, **agents.status()}


@router.get("/api/mcp/window")
def window(since: int = -1) -> dict:
    agents.seen()
    return {"epoch": engine.epoch(), "changed": engine.epoch() != since, "consent": agents.pending()}


@router.post("/api/mcp/consent")
def consent(body: Consent) -> dict:
    agents.answer(body.allow)
    return agents.status()


@router.post("/api/mcp/reset")
def reset() -> dict:
    agents.reset()
    return agents.status()
