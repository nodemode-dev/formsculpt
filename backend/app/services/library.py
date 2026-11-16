"""The sculpt library behind the Home screen. It is LightBox's Project folder (FSC_DATA_DIR or ~/Formsculpt, under
LightBox/Project): each sculpt is <name>.fsc with LightBox's square <name>.png beside it, so both screens show the
same files and nothing is migrated. Home's own files stay out of LightBox's sight in two hidden folders there:
.home (a 16:9 thumbnail and a few facts per sculpt, the pinned and recent lists) and .trash (one folder per deleted
sculpt, named <name>-<unix seconds>, which is all a restore needs)."""

import base64
import json
import os
import re
import shutil
import threading
import time
from datetime import datetime, timezone
from pathlib import Path

from app.core import config
from app.services import engine

try:
    import fsc_engine
except ImportError:
    fsc_engine = None

NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9 _.-]{0,63}$")
ENTRY = re.compile(r"^([A-Za-z0-9][A-Za-z0-9 _.-]{0,63})-(\d{1,12})$")
RESERVED = {"con", "prn", "aux", "nul", *(f"com{n}" for n in range(1, 10)), *(f"lpt{n}" for n in range(1, 10))}
THUMB_MAX = 2_000_000
THUMB_SIZE = (480, 270)
PICTURE_SIZE = (160, 160)
LIST_MAX = 500
UNTITLED = "Untitled sculpt"

TEMPLATES: list[dict] = [
    {"id": "sphere", "name": "Sphere", "category": "Sculpting bases", "tags": ["round", "organic", "ball"], "description": "A sphere of even triangles, the usual start for organic forms.", "steps": [["add_primitive", {"kind": "sphere"}]]},
    {"id": "quad_sphere", "name": "Quad Sphere", "category": "Sculpting bases", "tags": ["round", "quads", "subdivision"], "description": "A sphere made of quads, good for multires levels.", "steps": [["add_primitive", {"kind": "quad_sphere", "detail": 32}]]},
    {"id": "head", "name": "Head", "category": "Sculpting bases", "tags": ["face", "bust", "character"], "description": "A plain head shape to start a portrait or a character from.", "steps": [["add_primitive", {"kind": "head"}]]},
    {"id": "cube", "name": "Cube", "category": "Primitives", "tags": ["box", "hard surface"], "description": "A cube with sixteen divisions a side.", "steps": [["add_primitive", {"kind": "cube"}]]},
    {"id": "cylinder", "name": "Cylinder", "category": "Primitives", "tags": ["tube", "hard surface"], "description": "A capped cylinder two units tall.", "steps": [["add_primitive", {"kind": "cylinder"}]]},
    {"id": "cone", "name": "Cone", "category": "Primitives", "tags": ["spike", "hard surface"], "description": "A cone two units tall.", "steps": [["add_primitive", {"kind": "cone"}]]},
    {"id": "torus", "name": "Torus", "category": "Primitives", "tags": ["ring", "donut"], "description": "A ring with a round section.", "steps": [["add_primitive", {"kind": "torus"}]]},
    {"id": "plane", "name": "Plane", "category": "Primitives", "tags": ["flat", "relief", "ground"], "description": "A flat sheet for reliefs and ground.", "steps": [["add_primitive", {"kind": "plane", "detail": 64}]]},
    {"id": "uv_sphere", "name": "UV Sphere", "category": "Primitives", "tags": ["round", "poles"], "description": "A sphere of rings and segments with a pole at each end.", "steps": [["add_primitive", {"kind": "uv_sphere"}]]},
    {"id": "icosahedron", "name": "Icosahedron", "category": "Primitives", "tags": ["facets", "low poly"], "description": "The twenty flat faces of an icosahedron.", "steps": [["add_primitive", {"kind": "icosahedron"}]]},
]
DEFAULT_TEMPLATE = "sphere"

_lock = threading.RLock()
_scratch = None
_scratch_lock = threading.Lock()
_template_pictures: dict[str, bytes] = {}


class LibraryError(Exception):
    def __init__(self, status: int, detail: str) -> None:
        super().__init__(detail)
        self.status = status
        self.detail = detail


def folder() -> Path:
    path = config.data_dir() / "LightBox" / "Project"
    path.mkdir(parents=True, exist_ok=True)
    return path


def _home() -> Path:
    path = folder() / ".home"
    path.mkdir(exist_ok=True)
    return path


def _trash() -> Path:
    path = folder() / ".trash"
    path.mkdir(exist_ok=True)
    return path


def checked(name: str) -> str:
    name = name.strip() if isinstance(name, str) else ""
    if not NAME.match(name) or name.endswith((".", " ")) or name.split(".")[0].strip().lower() in RESERVED:
        raise LibraryError(400, "a sculpt name is 1 to 64 letters, digits, spaces, dots, dashes or underscores")
    return name


def _within(path: Path, root: Path) -> Path:
    if path.is_symlink() or not path.resolve().is_relative_to(root.resolve()):
        raise LibraryError(400, "that path is outside the library")
    return path


def _files(sid: str) -> dict[str, Path]:
    root = folder()
    home = _home()
    return {
        "sculpt.fsc": _within(root / f"{sid}.fsc", root),
        "picture.png": _within(root / f"{sid}.png", root),
        "thumb": _within(home / f"{sid}.thumb", root),
        "info.json": _within(home / f"{sid}.json", root),
    }


def _write(path: Path, data: bytes) -> None:
    temp = path.with_name(path.name + ".part")
    try:
        temp.write_bytes(data)
        os.replace(temp, path)
    finally:
        if temp.exists():
            temp.unlink()


def _read_json(path: Path) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    return data if isinstance(data, dict) else {}


def _state() -> dict:
    data = _read_json(_home() / "home.json")
    names = lambda v: [x for x in v if isinstance(x, str) and NAME.match(x)][:LIST_MAX] if isinstance(v, list) else []
    opened = data.get("opened")
    return {
        "pinned": names(data.get("pinned")),
        "recentHidden": names(data.get("recentHidden")),
        "opened": {k: float(v) for k, v in opened.items() if isinstance(k, str) and isinstance(v, (int, float))} if isinstance(opened, dict) else {},
    }


def _save_state(state: dict) -> None:
    _write(_home() / "home.json", json.dumps(state, indent=1).encode("utf-8"))


def _forget(sid: str) -> None:
    state = _state()
    state["pinned"] = [x for x in state["pinned"] if x != sid]
    state["recentHidden"] = [x for x in state["recentHidden"] if x != sid]
    state["opened"].pop(sid, None)
    _save_state(state)


def settings() -> dict:
    with _lock:
        state = _state()
    return {"pinned": state["pinned"], "recentHidden": state["recentHidden"]}


def update_settings(patch: dict) -> dict:
    with _lock:
        state = _state()
        for key in ("pinned", "recentHidden"):
            if key not in patch:
                continue
            value = patch[key]
            if not isinstance(value, list) or len(value) > LIST_MAX or any(not isinstance(x, str) or not NAME.match(x) for x in value):
                raise LibraryError(400, f"{key} is a list of sculpt names")
            state[key] = list(dict.fromkeys(value))
        _save_state(state)
    return {"pinned": state["pinned"], "recentHidden": state["recentHidden"]}


def _iso(seconds: float) -> str:
    return datetime.fromtimestamp(seconds, timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def _summary(sid: str, opened: dict | None = None) -> dict:
    files = _files(sid)
    stat = files["sculpt.fsc"].stat()
    info = _read_json(files["info.json"])
    shown = files["thumb"] if files["thumb"].is_file() else files["picture.png"] if files["picture.png"].is_file() else None
    seen = (opened if opened is not None else _state()["opened"]).get(sid)
    count = lambda key: info[key] if isinstance(info.get(key), int) else None
    return {
        "id": sid,
        "title": sid,
        "objects": count("objects"),
        "triangles": count("triangles"),
        "bytes": stat.st_size,
        "modified": _iso(stat.st_mtime),
        "opened": _iso(seen) if seen else None,
        "recent": max(stat.st_mtime, seen or 0),
        "template": info.get("template") if isinstance(info.get("template"), str) else None,
        "thumb": shown is not None,
        "thumbAt": int(shown.stat().st_mtime * 1000) if shown else 0,
    }


def sculpts() -> list[dict]:
    with _lock:
        opened = _state()["opened"]
        items = []
        for f in folder().glob("*.fsc"):
            if not f.is_file() or not NAME.match(f.stem):
                continue
            try:
                items.append(_summary(f.stem, opened))
            except (OSError, LibraryError):
                continue
    items.sort(key=lambda x: x["recent"], reverse=True)
    return items


def _exists(sid: str) -> bool:
    return (folder() / f"{sid}.fsc").exists()


def _unique(base: str) -> str:
    base = checked(base)
    if not _exists(base):
        return base
    for n in range(2, 1000):
        tail = f" {n}"
        name = f"{base[:64 - len(tail)].rstrip(' .')}{tail}"
        if not _exists(name):
            return name
    raise LibraryError(409, "too many sculpts share that name")


def _suffixed(sid: str, suffix: str) -> str:
    return f"{sid[:64 - len(suffix)].rstrip(' .')}{suffix}"


def _need_engine() -> None:
    if not engine.available():
        raise LibraryError(503, "the engine module is not built; run ./formsculpt engine build")


def _view(run, size: tuple[int, int]) -> tuple[bytes | None, dict]:
    reply = run("get_view", {"width": size[0], "height": size[1], "background": "transparent", "shading": "clay"})
    if not reply.get("ok") or not reply["result"].get("png_base64"):
        return None, {}
    return base64.b64decode(reply["result"]["png_base64"]), reply["result"]


def _store(sid: str, data: bytes, run, template: str | None, wide: bool) -> None:
    files = _files(sid)
    _write(files["sculpt.fsc"], data)
    picture, facts = _view(run, PICTURE_SIZE)
    if picture:
        _write(files["picture.png"], picture)
    elif files["picture.png"].exists():
        files["picture.png"].unlink()
    if wide:
        thumb, _ = _view(run, THUMB_SIZE)
        if thumb:
            _write(files["thumb"], thumb)
        elif files["thumb"].exists():
            files["thumb"].unlink()
    info = {"objects": facts.get("objects"), "triangles": facts.get("triangles")}
    if template:
        info["template"] = template
    elif isinstance(_read_json(files["info.json"]).get("template"), str):
        info["template"] = _read_json(files["info.json"])["template"]
    _write(files["info.json"], json.dumps(info).encode("utf-8"))


def _scratch_run(name: str, args: dict) -> dict:
    return json.loads(_scratch.dispatch(name, json.dumps(args)))


def _build(template: dict):
    """The template's scene in a second engine of this module's own, so the scene the window shows is never touched."""
    global _scratch
    if _scratch is None:
        _scratch = fsc_engine.Engine()
    reply = _scratch_run("new_scene", {})
    for name, args in template["steps"]:
        reply = _scratch_run(name, args)
        if not reply.get("ok"):
            break
    if not reply.get("ok"):
        raise LibraryError(500, (reply.get("error") or {}).get("message", "the template could not be built"))


def template(tid: str) -> dict:
    for t in TEMPLATES:
        if t["id"] == tid:
            return t
    raise LibraryError(404, "no template with that id")


def templates() -> dict:
    listed = [{k: v for k, v in t.items() if k != "steps"} | {"source": "builtin", "thumb": engine.available()} for t in TEMPLATES]
    return {"categories": list(dict.fromkeys(t["category"] for t in TEMPLATES)), "templates": listed, "default": DEFAULT_TEMPLATE}


def template_picture(tid: str) -> bytes:
    found = template(tid)
    _need_engine()
    with _scratch_lock:
        have = _template_pictures.get(tid)
        if have:
            return have
        cache = config.data_dir() / "LightBox" / ".templates" / f"{tid}-{config.VERSION}.png"
        if cache.is_file():
            data = cache.read_bytes()
        else:
            _build(found)
            data, _ = _view(_scratch_run, THUMB_SIZE)
            _scratch_run("new_scene", {})
            if not data:
                raise LibraryError(500, "the template could not be drawn")
            try:
                cache.parent.mkdir(parents=True, exist_ok=True)
                _write(cache, data)
            except OSError:
                pass
        _template_pictures[tid] = data
        return data


def create(title: str | None, tid: str | None) -> dict:
    found = template(tid or DEFAULT_TEMPLATE)
    _need_engine()
    with _lock:
        sid = _unique(title.strip() if isinstance(title, str) and title.strip() else found["name"])
        with _scratch_lock:
            _build(found)
            _store(sid, _scratch.save_bytes(), _scratch_run, found["id"], True)
            _scratch_run("new_scene", {})
        return _summary(sid)


def _free(name: str, but: str | None = None) -> str:
    name = checked(name)
    low = name.lower()
    if any(f.stem.lower() == low and f.stem != but for f in folder().glob("*.fsc")):
        raise LibraryError(409, "a sculpt with that name is already in the library")
    return name


def create_from_scene(title: str | None, wide: bool = True, exact: bool = False) -> dict:
    _need_engine()
    if engine.object_count() == 0:
        raise LibraryError(400, "the scene is empty; there is nothing to save")
    with _lock:
        sid = _free(title or "") if exact else _unique(title.strip() if isinstance(title, str) and title.strip() else UNTITLED)
        _store(sid, engine.save_bytes(), engine.dispatch, None, wide)
        _mark_opened(sid)
        return _summary(sid)


def save(sid: str, wide: bool = True) -> dict:
    sid = checked(sid)
    _need_engine()
    with _lock:
        _store(sid, engine.save_bytes(), engine.dispatch, None, wide)
        _mark_opened(sid)
        return _summary(sid)


def _mark_opened(sid: str) -> None:
    state = _state()
    state["opened"][sid] = time.time()
    _save_state(state)


def _have(sid: str) -> dict[str, Path]:
    files = _files(checked(sid))
    if not files["sculpt.fsc"].is_file():
        raise LibraryError(404, "no sculpt with that name in the library")
    return files


def open_sculpt(sid: str) -> dict:
    _need_engine()
    with _lock:
        files = _have(sid)
        reply = engine.open_bytes(files["sculpt.fsc"].read_bytes())
        if not reply.get("ok"):
            raise LibraryError(400, (reply.get("error") or {}).get("message", "that sculpt could not be opened"))
        _mark_opened(sid.strip())
        return {"ok": True, "sculpt": _summary(sid.strip()), "result": reply.get("result")}


def duplicate(sid: str) -> dict:
    with _lock:
        files = _have(sid)
        copy = _unique(_suffixed(sid.strip(), " copy"))
        target = _files(copy)
        for key, source in files.items():
            if source.is_file():
                _write(target[key], source.read_bytes())
        return _summary(copy)


def rename(sid: str, title: str) -> dict:
    with _lock:
        files = _have(sid)
        sid = sid.strip()
        name = _free(title if isinstance(title, str) else "", sid)
        if name == sid:
            return _summary(sid)
        target = _files(name)
        for key in ("picture.png", "thumb", "info.json", "sculpt.fsc"):
            if files[key].is_file():
                os.replace(files[key], target[key])
            elif target[key].is_file():
                target[key].unlink()
        state = _state()
        for key in ("pinned", "recentHidden"):
            state[key] = [name if x == sid else x for x in state[key]]
        if sid in state["opened"]:
            state["opened"][name] = state["opened"].pop(sid)
        _save_state(state)
        return _summary(name)


def remove(sid: str) -> dict:
    with _lock:
        files = _have(sid)
        sid = sid.strip()
        root = _trash()
        when = int(time.time())
        while (root / f"{sid}-{when}").exists():
            when += 1
        entry = _within(root / f"{sid}-{when}", root)
        entry.mkdir()
        info = _read_json(files["info.json"])
        for key, source in files.items():
            if source.is_file():
                os.replace(source, entry / key)
        _write(entry / "info.json", json.dumps({**info, "id": sid, "deletedAt": when}).encode("utf-8"))
        _forget(sid)
    return {"ok": True, "entry": entry.name}


def _entry(name: str) -> tuple[Path, str, int]:
    m = ENTRY.match(name) if isinstance(name, str) else None
    root = _trash()
    if not m:
        raise LibraryError(400, "that is not a trash entry")
    path = _within(root / name, root)
    if not path.is_dir() or not (path / "sculpt.fsc").is_file():
        raise LibraryError(404, "That sculpt is no longer in the trash.")
    return path, m.group(1), int(m.group(2))


def trash() -> list[dict]:
    items = []
    with _lock:
        for d in _trash().iterdir():
            try:
                path, sid, when = _entry(d.name)
            except LibraryError:
                continue
            info = _read_json(path / "info.json")
            items.append({
                "entry": d.name,
                "id": sid,
                "title": sid,
                "objects": info["objects"] if isinstance(info.get("objects"), int) else None,
                "deletedAt": when,
                "thumb": (path / "thumb").is_file() or (path / "picture.png").is_file(),
            })
    items.sort(key=lambda x: (x["deletedAt"], x["entry"]), reverse=True)
    return items


def restore(name: str) -> dict:
    with _lock:
        path, sid, _ = _entry(name)
        target = sid if not _exists(sid) else _unique(_suffixed(sid, " restored"))
        files = _files(target)
        info = {k: v for k, v in _read_json(path / "info.json").items() if k not in ("id", "deletedAt")}
        for key in ("picture.png", "thumb"):
            if (path / key).is_file():
                os.replace(path / key, files[key])
        _write(files["info.json"], json.dumps(info).encode("utf-8"))
        os.replace(path / "sculpt.fsc", files["sculpt.fsc"])
        shutil.rmtree(path, ignore_errors=True)
        return _summary(target)


def purge(name: str) -> dict:
    with _lock:
        path, _, _ = _entry(name)
        shutil.rmtree(path)
    return {"ok": True}


def empty_trash() -> dict:
    removed = 0
    with _lock:
        root = _trash()
        for d in list(root.iterdir()):
            if not ENTRY.match(d.name) or d.is_symlink() or not d.is_dir():
                continue
            shutil.rmtree(_within(d, root))
            removed += 1
    return {"ok": True, "removed": removed}


def _kind(data: bytes) -> str | None:
    if data[:8] == b"\x89PNG\r\n\x1a\n":
        return "image/png"
    if data[:3] == b"\xff\xd8\xff":
        return "image/jpeg"
    if data[:4] == b"RIFF" and data[8:12] == b"WEBP":
        return "image/webp"
    return None


def put_thumb(sid: str, data: bytes) -> dict:
    if len(data) > THUMB_MAX:
        raise LibraryError(413, "a thumbnail is at most 2 MB")
    if _kind(data) is None:
        raise LibraryError(415, "a thumbnail is a PNG, JPEG or WebP picture")
    with _lock:
        files = _have(sid)
        _write(files["thumb"], data)
    return {"ok": True}


def thumb(sid: str) -> tuple[bytes, str]:
    files = _have(sid)
    for key in ("thumb", "picture.png"):
        if files[key].is_file():
            data = files[key].read_bytes()
            return data, _kind(data) or "image/png"
    raise LibraryError(404, "no picture for this sculpt")
