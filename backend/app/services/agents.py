import asyncio
import secrets
import time

INAPP_HEADER = "x-formsculpt-inapp"
INAPP_TOKEN = secrets.token_urlsafe(24)
LIVE_SECONDS = 4.0
DECLINE_SECONDS = 600.0
ASK_SECONDS = 90.0

_last_poll = 0.0
_allowed: bool | None = None
_declined_until = 0.0
_pending: dict | None = None
_answer: asyncio.Event | None = None


def seen() -> None:
    global _last_poll
    _last_poll = time.monotonic()


def window_live() -> bool:
    return time.monotonic() - _last_poll <= LIVE_SECONDS


def pending() -> dict | None:
    return _pending


def status() -> dict:
    return {"window": window_live(), "allowed": _allowed, "asking": _pending is not None, "declined": time.monotonic() < _declined_until}


def answer(allow: bool) -> None:
    global _allowed, _declined_until, _pending
    _allowed = allow
    if not allow:
        _declined_until = time.monotonic() + DECLINE_SECONDS
    _pending = None
    if _answer is not None:
        _answer.set()


def reset() -> None:
    global _allowed, _declined_until
    _allowed = None
    _declined_until = 0.0


async def permit(tool: str, inapp: bool = False) -> str | None:
    global _pending, _answer
    if time.monotonic() < _declined_until:
        return "The person declined outside agents in the formsculpt window in the last ten minutes. Ask them to allow agents there, then retry."
    if inapp or _allowed is True:
        return None
    if not window_live():
        return "The formsculpt window is not open, so nobody can approve outside agents. Ask the person to open formsculpt and allow agents there, then retry."
    if _pending is None:
        _answer = asyncio.Event()
        _pending = {"tool": tool, "asked": time.time()}
    event = _answer
    try:
        await asyncio.wait_for(event.wait(), ASK_SECONDS)
    except asyncio.TimeoutError:
        return "Nobody answered the permission prompt in the formsculpt window. Ask the person to allow outside agents there, then retry."
    if _allowed:
        return None
    return "The person declined outside agents in the formsculpt window. Do not retry unless they ask you to."
