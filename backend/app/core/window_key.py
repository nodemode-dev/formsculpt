import asyncio
import hmac
import json
import os
import secrets
import sys
import time
from urllib.parse import parse_qs

from fastapi import APIRouter, HTTPException, WebSocket

HEADER = "x-formsculpt-window-key"
STDOUT_ENV = "FSC_WINDOW_KEY_STDOUT"
LINE = "formsculpt-window-key: "
REFUSED = "Only the formsculpt window can do this"
READS = ("GET", "HEAD", "OPTIONS")
OPEN = ("/mcp", "/api/window/key")
KEYED_READS = ("/api/mcp/window", "/api/events/recent")
STREAMS = ("/api/events",)
STREAM_PREFIXES = ("/api/chat/image/",)
SOCKET_WAIT = 5.0
TICKET_LIFE = 30.0
KEY = secrets.token_urlsafe(32)
STATE = {"claimed": False, "secondClaim": False}
TICKETS: dict[str, float] = {}

router = APIRouter(prefix="/api/window", tags=["window"])


def stdout_mode() -> bool:
    return os.environ.get(STDOUT_ENV) == "1"


def valid(value) -> bool:
    return isinstance(value, str) and bool(value) and hmac.compare_digest(value.encode("utf-8"), KEY.encode("utf-8"))


def claim() -> str | None:
    if STATE["claimed"]:
        STATE["secondClaim"] = True
        return None
    STATE["claimed"] = True
    return KEY


def ticket() -> str:
    now = time.monotonic()
    for old in [t for t, at in TICKETS.items() if now - at > TICKET_LIFE]:
        TICKETS.pop(old, None)
    value = secrets.token_urlsafe(24)
    TICKETS[value] = now
    return value


def spend(value) -> bool:
    if not isinstance(value, str) or not value:
        return False
    at = TICKETS.pop(value, None)
    return at is not None and time.monotonic() - at <= TICKET_LIFE


def is_stream(path: str) -> bool:
    return path in STREAMS or path.startswith(STREAM_PREFIXES)


def ticket_of(scope) -> str:
    raw = scope.get("query_string", b"").decode("latin-1")
    return (parse_qs(raw).get("ticket") or [""])[0]


def announce() -> None:
    if stdout_mode() and sys.stdout is not None:
        print(LINE + KEY, flush=True)


async def socket_admitted(ws: WebSocket) -> bool:
    try:
        first = json.loads(await asyncio.wait_for(ws.receive_text(), timeout=SOCKET_WAIT))
    except Exception:
        return False
    return isinstance(first, dict) and first.get("type") == "key" and valid(first.get("key"))


@router.post("/key")
def claim_key() -> dict:
    if stdout_mode():
        raise HTTPException(404, "Not Found")
    key = claim()
    if key is None:
        raise HTTPException(409, "The window key was already given to a window of this engine.")
    return {"key": key}


@router.get("/key")
def key_state() -> dict:
    return {"claimed": STATE["claimed"], "secondClaim": STATE["secondClaim"], "packaged": stdout_mode()}


@router.post("/ticket")
def stream_ticket() -> dict:
    return {"ticket": ticket()}


def needs_key(method: str, path: str) -> bool:
    if path in OPEN:
        return False
    return method not in READS or path in KEYED_READS or is_stream(path)


class WindowKeyGuard:
    def __init__(self, app) -> None:
        self.app = app

    async def __call__(self, scope, receive, send) -> None:
        if scope["type"] != "http" or not needs_key(str(scope.get("method", "")), str(scope.get("path", ""))):
            await self.app(scope, receive, send)
            return
        given = next((v.decode("latin-1") for k, v in scope.get("headers", []) if k.decode("latin-1").lower() == HEADER), None)
        if valid(given) or (is_stream(str(scope.get("path", ""))) and spend(ticket_of(scope))):
            await self.app(scope, receive, send)
            return
        body = json.dumps({"detail": REFUSED}).encode("utf-8")
        await send({"type": "http.response.start", "status": 403, "headers": [(b"content-type", b"application/json"), (b"content-length", str(len(body)).encode())]})
        await send({"type": "http.response.body", "body": body})
