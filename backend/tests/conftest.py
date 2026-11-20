import os
import shutil
import sys
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import family  # noqa: E402
from family import engine_proc, guard_client, repo_version, repo_versions  # noqa: E402,F401

AGENT_NAMES = ("claude", "codex", "cursor-agent", "agent", "agy")
AGENT_KINDS = ("claude", "codex", "cursor", "antigravity")
STAND_INS = family.sandbox() / "bin"
AGENT_CALLS = family.sandbox() / "agent-calls.log"
AGENT_SHELL = """#!/bin/sh
script=""
while [ $# -gt 0 ]; do if [ "$1" = "-c" ]; then shift; script="$1"; fi; shift; done
if [ -n "$script" ]; then exec /bin/sh -c "$script"; fi
exec /bin/sh
"""
PATH_WITHOUT_AGENTS = os.pathsep.join(d for d in os.environ.get("PATH", "").split(os.pathsep) if d and not any(os.path.exists(os.path.join(d, n)) for n in AGENT_NAMES))
if os.name != "nt":
    for _name in ("bin", "shell"):
        (family.sandbox() / _name).mkdir(exist_ok=True)
    for _name in AGENT_NAMES:
        (STAND_INS / _name).write_text(f'#!/bin/sh\necho "{_name} $*" >> "{AGENT_CALLS}"\nif [ "$1" = "--version" ]; then echo "0.0.0 (stand-in)"; exit 0; fi\nexit 1\n', encoding="utf-8")
        (STAND_INS / _name).chmod(0o755)
    (family.sandbox() / "shell" / "sh").write_text(AGENT_SHELL, encoding="utf-8")
    (family.sandbox() / "shell" / "sh").chmod(0o755)
    os.environ["SHELL"] = str(family.sandbox() / "shell" / "sh")
    os.environ["PATH"] = os.pathsep.join([str(STAND_INS), PATH_WITHOUT_AGENTS])
    for _name in ("NPM_CONFIG_PREFIX", "npm_config_prefix", "PNPM_HOME", "VOLTA_HOME", "BUN_INSTALL", "NVM_SYMLINK"):
        os.environ.pop(_name, None)

from app.main import app  # noqa: E402


def real_agent_programs() -> list[str]:
    from app.services import agent_lookup

    kept = agent_lookup.system_folders
    agent_lookup.system_folders = lambda platform=None: []
    try:
        found = [agent_lookup.find(kind, fresh=True) for kind in AGENT_KINDS]
    finally:
        agent_lookup.system_folders = kept
    real = [f"{f.title} at {f.path}" for f in found if f.path and not Path(f.path).is_relative_to(STAND_INS)]
    return real + [f"{name} at {hit}" for name in AGENT_NAMES if (hit := shutil.which(name)) and not Path(hit).is_relative_to(STAND_INS)]


def pytest_sessionstart(session):
    real = real_agent_programs()
    if real:
        family.cleanup_sandbox()
        pytest.exit("refusing to run: the tests would find a real agent program instead of a stand-in: " + "; ".join(real), returncode=4)


@pytest.fixture(autouse=True)
def only_stand_in_agent_programs(monkeypatch):
    from app.services import agent_lookup

    monkeypatch.setattr(agent_lookup, "system_folders", lambda platform=None: [])


def stand_in_calls() -> list[str]:
    try:
        return AGENT_CALLS.read_text(encoding="utf-8").splitlines()
    except OSError:
        return []


CALLS_AT_END: list[str] = []


def pytest_terminal_summary(terminalreporter):
    calls = CALLS_AT_END or stand_in_calls()
    terminalreporter.write_line(f"agent program stand-ins started {len(calls)} time(s): " + (", ".join(sorted(set(calls))) or "none"))


def pytest_sessionfinish(session, exitstatus):
    CALLS_AT_END.extend(stand_in_calls())
    family.cleanup_sandbox()
from app.services import engine  # noqa: E402


@pytest.fixture()
def client() -> TestClient:
    # The service keeps one engine session; every test starts from an empty scene.
    if engine.available():
        engine.dispatch("new_scene", {})
    return TestClient(app, base_url=family.own_origin(), headers=family.window_headers())


def call(client: TestClient, command: str, /, **args) -> dict:
    # Positional-only, so a command's own `name` argument never collides with the command.
    reply = client.post("/api/engine/dispatch", json={"name": command, "args": args}).json()
    assert reply["ok"], reply
    return reply["result"]
