import sys
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import family  # noqa: E402
from family import engine_proc, guard_client, repo_version, repo_versions  # noqa: E402,F401
from app.main import app  # noqa: E402


def pytest_sessionfinish(session, exitstatus):
    family.cleanup_sandbox()
from app.services import engine  # noqa: E402


@pytest.fixture()
def client() -> TestClient:
    # The service keeps one engine session; every test starts from an empty scene.
    if engine.available():
        engine.dispatch("new_scene", {})
    return TestClient(app, base_url=family.own_origin())


def call(client: TestClient, command: str, /, **args) -> dict:
    # Positional-only, so a command's own `name` argument never collides with the command.
    reply = client.post("/api/engine/dispatch", json={"name": command, "args": args}).json()
    assert reply["ok"], reply
    return reply["result"]
