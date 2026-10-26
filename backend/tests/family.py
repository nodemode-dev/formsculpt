import importlib
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

import pytest

BACKEND = Path(__file__).resolve().parents[1]
REPO = BACKEND.parent

APP_ID = "formsculpt"
APP_MODULE = "app.main:app"
ENV_PREFIX = "FSC"
DATA_ENV = "FSC_DATA_DIR"
PORT_ENV = "FSC_PORT"
ENGINE_PORT = 8260
HEALTH_PATH = "/api/health"
VERSION_PATH = "/api/version"
VERSION_KEY = "version"
WINDOW_ORIGINS = ["http://localhost:5260", "http://127.0.0.1:5260", "http://localhost:5261", "http://127.0.0.1:5261", "tauri://localhost", "http://tauri.localhost"]
WS_PATH = "/api/engine/stroke"
MCP_PATH = "/mcp"
STDIO_CMD = [sys.executable, str(REPO / "setup" / "mcp_stdio.py")]
READ_TOOL = ("get_scene", {})
CHANGING_TOOL = ("undo", {})
INAPP_HEADER = "x-formsculpt-inapp"
VERSION_SITES = [
    ("README.md", r"badge/version-([0-9.]+)-"),
    ("CHANGELOG.md", r"^## (\S+) ·"),
    ("backend/app/__init__.py", r'^__version__ = "([^"]+)"'),
    ("frontend/package.json", r'^  "version": "([^"]+)"'),
    ("frontend/src/version.ts", r"^export const VERSION = '([^']+)'"),
    ("integrations/blender/formsculpt_link/blender_manifest.toml", r'^version = "([^"]+)"'),
    ("native/Cargo.toml", r'^version = "([^"]+)"'),
    ("src-tauri/Cargo.toml", r'^version = "([^"]+)"'),
    ("src-tauri/tauri.conf.json", r'^  "version": "([^"]+)"'),
    (".claude/skills/formsculpt-modeling/references/tools.md", r"\(formsculpt ([0-9.]+)\)"),
    (".agents/skills/formsculpt-modeling/references/tools.md", r"\(formsculpt ([0-9.]+)\)"),
]
STRIP_EXEMPT = []

NO_WINDOW_TEXT = "window is not open"
DECLINED_TEXT = "declined outside agents"

_SANDBOX = Path(tempfile.mkdtemp(prefix=f"{APP_ID}-tests-"))
for _name in ("data", "home", "bridge"):
    (_SANDBOX / _name).mkdir()
os.environ[DATA_ENV] = str(_SANDBOX / "data")
os.environ[PORT_ENV] = str(ENGINE_PORT)
os.environ["HOME"] = str(_SANDBOX / "home")
os.environ["USERPROFILE"] = str(_SANDBOX / "home")
for _key in [k for k in os.environ if k.startswith(f"{ENV_PREFIX}_") and k not in (DATA_ENV, PORT_ENV)]:
    os.environ.pop(_key)
sys.path.insert(0, str(BACKEND))


def sandbox():
    return _SANDBOX


def cleanup_sandbox():
    shutil.rmtree(_SANDBOX, ignore_errors=True)


def own_origin():
    return f"http://127.0.0.1:{ENGINE_PORT}"


def tool_catalogue():
    return importlib.import_module("app.routers.mcp").tools()


def set_policy(mode):
    agents = importlib.import_module("app.services.agents")
    agents.reset()
    agents._last_poll = 0.0
    if mode == "allowed":
        agents.answer(True)
    elif mode == "blocked":
        agents.answer(False)


def inapp_token():
    return importlib.import_module("app.services.agents").INAPP_TOKEN


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def get_json(url, timeout=2.0):
    with urllib.request.urlopen(url, timeout=timeout) as r:
        return r.status, json.loads(r.read().decode("utf-8") or "null")


def read_versions():
    found = {}
    for rel, pattern in VERSION_SITES:
        text = (REPO / rel).read_text(encoding="utf-8")
        match = re.search(pattern, text, re.MULTILINE)
        found[rel] = match.group(1) if match else None
    return found


@pytest.fixture(scope="session")
def repo_versions():
    return read_versions()


@pytest.fixture(scope="session")
def repo_version(repo_versions):
    return next(iter(repo_versions.values()))


@pytest.fixture(scope="session")
def guard_client():
    from fastapi.testclient import TestClient

    app = importlib.import_module("app.main").app
    with TestClient(app, base_url=own_origin()) as c:
        yield c


@pytest.fixture(scope="session")
def engine_proc():
    port = free_port()
    data = _SANDBOX / "engine-data"
    data.mkdir()
    env = dict(os.environ, **{DATA_ENV: str(data), PORT_ENV: str(port)})
    proc = subprocess.Popen(
        [sys.executable, "-m", "uvicorn", APP_MODULE, "--host", "127.0.0.1", "--port", str(port)],
        cwd=BACKEND, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    base = f"http://127.0.0.1:{port}"
    deadline = time.monotonic() + 90
    ready = False
    while time.monotonic() < deadline and proc.poll() is None:
        try:
            status, _ = get_json(base + HEALTH_PATH)
            ready = status == 200
        except OSError:
            ready = False
        if ready:
            break
        time.sleep(0.25)
    if not ready:
        proc.kill()
        out = proc.communicate(timeout=5)[0].decode("utf-8", "replace")
        raise RuntimeError(f"engine did not answer {HEALTH_PATH} within 90 s:\n{out[-4000:]}")
    yield {"base": base, "port": port, "data": data, "env": env, "proc": proc}
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
