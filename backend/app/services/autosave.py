import asyncio
import os
import time
from pathlib import Path

from starlette.concurrency import run_in_threadpool

from app.services import engine

INTERVAL = float(os.environ.get("FSC_AUTOSAVE_SECONDS", "60"))
_saved_epoch = 0
_saved_at: float | None = None


def folder() -> Path:
    configured = os.environ.get("FSC_AUTOSAVE_DIR")
    return Path(configured) if configured else Path.home() / ".formsculpt" / "autosave"


def path() -> Path:
    return folder() / "autosave.fsc"


def info() -> dict:
    p = path()
    if not p.is_file():
        return {"exists": False, "path": str(p), "saved_at": None, "bytes": 0}
    stat = p.stat()
    return {"exists": True, "path": str(p), "saved_at": stat.st_mtime, "bytes": stat.st_size}


def save_now() -> bool:
    global _saved_epoch, _saved_at
    current = engine.epoch()
    if current == _saved_epoch or engine.object_count() == 0:
        return False
    data = engine.save_bytes()
    target = path()
    target.parent.mkdir(parents=True, exist_ok=True)
    temp = target.with_suffix(".fsc.part")
    temp.write_bytes(data)
    os.replace(temp, target)
    _saved_epoch = current
    _saved_at = time.time()
    return True


def recover() -> dict:
    p = path()
    if not p.is_file():
        return {"ok": False, "error": {"code": "no_autosave", "message": "there is no autosave yet"}}
    return engine.open_bytes(p.read_bytes())


async def loop() -> None:
    while True:
        await asyncio.sleep(INTERVAL)
        if engine.available():
            try:
                await run_in_threadpool(save_now)
            except OSError:
                pass
