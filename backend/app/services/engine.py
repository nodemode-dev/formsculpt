import json
import struct
import threading

try:
    import fsc_engine
except ImportError:
    fsc_engine = None

MAGIC = b"FSCM"
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


def dispatch(name: str, args: dict | None) -> dict:
    if _engine is None:
        raise EngineUnavailable("the engine module is not built; run ./formsculptor engine build")
    with _lock:
        return json.loads(_engine.dispatch(name, json.dumps(args or {})))


def mesh_payload(object_id: int) -> bytes:
    if _engine is None:
        raise EngineUnavailable("the engine module is not built; run ./formsculptor engine build")
    with _lock:
        positions, normals, indices, revision = _engine.mesh(object_id)
    header = MAGIC + struct.pack("<IIII", len(positions) // 12, len(indices) // 4, revision & 0xFFFFFFFF, 0)
    return header + positions + normals + indices
