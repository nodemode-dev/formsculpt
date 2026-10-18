import base64
import io
import json
import struct
import threading
import zipfile

try:
    import fsc_engine
except ImportError:
    fsc_engine = None

MAGIC = b"FSCM"
FLAG_MASK = 1
FLAG_PAINT = 2
FLAG_GROUPS = 4  # u32 face group per triangle, after the paint
FLAG_HIDDEN = 8  # u8 hidden flag per triangle, after the groups
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


def mesh_payload(object_id: int, smooth: int = 0) -> bytes:
    if _engine is None:
        raise EngineUnavailable("the engine module is not built; run ./formsculptor engine build")
    if smooth > 0:
        # Dynamic subdivision for display: no mask, groups or hidden faces, which belong to the sculpted mesh.
        with _lock:
            positions, normals, indices, revision, paint = _engine.mesh_smooth(object_id, smooth)
        flags = FLAG_PAINT if paint else 0
        header = MAGIC + struct.pack("<IIII", len(positions) // 12, len(indices) // 4, revision & 0xFFFFFFFF, flags)
        return header + positions + normals + indices + paint
    with _lock:
        positions, normals, indices, mask, revision, paint = _engine.mesh(object_id)
        groups, hidden = _engine.faces(object_id)
    flags = FLAG_MASK | (FLAG_PAINT if paint else 0) | (FLAG_GROUPS if groups else 0) | (FLAG_HIDDEN if hidden else 0)
    header = MAGIC + struct.pack("<IIII", len(positions) // 12, len(indices) // 4, revision & 0xFFFFFFFF, flags)
    return header + positions + normals + indices + mask + paint + groups + hidden


def boolean_preview(a: int, b: int, op: str) -> bytes:
    """The exact boolean of two objects as an FSCM payload, the scene unchanged; KeyError for a missing object,
    ValueError for a refusal (an open surface, nothing left)."""
    _require()
    with _lock:
        return _engine.boolean_preview(a, b, op)


def stroke_begin(object_id: int, settings: dict) -> None:
    if _engine is None:
        raise EngineUnavailable("the engine module is not built; run ./formsculptor engine build")
    with _lock:
        _engine.stroke_begin(object_id, json.dumps(settings))


def stroke_sample(origin: list[float], direction: list[float], pressure: float, tilt: float = 0.0) -> tuple[bool, bytes, bool]:
    with _lock:
        return _engine.stroke_sample(origin, direction, pressure, tilt)


def stroke_end() -> dict:
    with _lock:
        reply = json.loads(_engine.stroke_end())
    if reply.get("ok") and reply.get("result"):
        _bump()
    return reply


def _require() -> None:
    if _engine is None:
        raise EngineUnavailable("the engine module is not built; run ./formsculptor engine build")


def save_bytes() -> bytes:
    _require()
    with _lock:
        return _engine.save_bytes()


def open_bytes(data: bytes) -> dict:
    _require()
    with _lock:
        reply = json.loads(_engine.open_bytes(data))
    if reply.get("ok"):
        _bump()
    return reply


def import_bytes(file_name: str, data: bytes, scale: float = 1.0, up_axis: str = "y", merge: bool = False) -> dict:
    _require()
    with _lock:
        reply = json.loads(_engine.import_bytes(file_name, data, None, scale, up_axis, merge))
    if reply.get("ok"):
        _bump()
    return reply


def export_bytes(file_name: str, ids: list[int] | None, scale: float = 1.0, up_axis: str = "y", triangulate: bool = False) -> bytes:
    _require()
    with _lock:
        return _engine.export_bytes(file_name, ids, scale, up_axis, triangulate)


def maps_zip(object_id: int) -> bytes | None:
    _require()
    with _lock:
        found = _engine.maps(object_id)
    if found is None:
        return None
    name, files = found
    stem = "".join(c if c.isalnum() else "_" for c in name)
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as archive:
        for suffix, data in files:
            archive.writestr(f"{stem}_{suffix}", bytes(data))
    return out.getvalue()


def view_png(args: dict) -> tuple[bytes, dict]:
    """A headless render (get_view) as PNG bytes and its facts; ValueError carries the engine's refusal."""
    reply = dispatch("get_view", args)
    if not reply.get("ok"):
        raise ValueError((reply.get("error") or {}).get("message", "render failed"))
    result = reply["result"]
    return base64.b64decode(result.pop("png_base64")), result


def object_count() -> int:
    _require()
    with _lock:
        return len(json.loads(_engine.dispatch("get_scene", "{}"))["result"]["objects"])
