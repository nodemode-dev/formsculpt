import json
import struct
import threading

try:
    import fsc_engine
except ImportError:
    fsc_engine = None

MAGIC = b"FSCM"
FLAG_MASK = 1
_lock = threading.Lock()
_engine = fsc_engine.Engine() if fsc_engine else None


class EngineUnavailable(RuntimeError):
    pass


def available() -> bool:
    return _engine is not None


def catalogue() -> dict:
    if fsc_engine is None:
        raise EngineUnavailable("the engine module is not built; run ./formsculptor engine build")
    return json.loads(fsc_engine.catalogue())


def status() -> dict:
    if fsc_engine is None:
        return {"native": False, "version": None, "commands": []}
    return {"native": True, "version": fsc_engine.version(), "commands": [c["name"] for c in catalogue()["commands"]]}


_epoch = 0
_read_only: set[str] | None = None


def epoch() -> int:
    return _epoch


def _bump() -> None:
    global _epoch
    _epoch += 1


def read_only(name: str) -> bool:
    global _read_only
    if _read_only is None:
        _read_only = set()
        for c in catalogue()["commands"]:
            if c["annotations"].get("readOnlyHint"):
                _read_only.update({c["name"], c["id"]})
    return name in _read_only


def dispatch(name: str, args: dict | None) -> dict:
    if _engine is None:
        raise EngineUnavailable("the engine module is not built; run ./formsculptor engine build")
    with _lock:
        reply = json.loads(_engine.dispatch(name, json.dumps(args or {})))
    if reply.get("ok") and not read_only(name):
        _bump()
    return reply


def mesh_payload(object_id: int) -> bytes:
    if _engine is None:
        raise EngineUnavailable("the engine module is not built; run ./formsculptor engine build")
    with _lock:
        positions, normals, indices, mask, revision = _engine.mesh(object_id)
    header = MAGIC + struct.pack("<IIII", len(positions) // 12, len(indices) // 4, revision & 0xFFFFFFFF, FLAG_MASK)
    return header + positions + normals + indices + mask


def stroke_begin(object_id: int, settings: dict) -> None:
    if _engine is None:
        raise EngineUnavailable("the engine module is not built; run ./formsculptor engine build")
    with _lock:
        _engine.stroke_begin(object_id, json.dumps(settings))


def stroke_sample(origin: list[float], direction: list[float], pressure: float) -> tuple[bool, bytes, bool]:
    with _lock:
        return _engine.stroke_sample(origin, direction, pressure)


def stroke_end() -> dict:
    with _lock:
        reply = json.loads(_engine.stroke_end())
    if reply.get("ok") and reply.get("result"):
        _bump()
    return reply
