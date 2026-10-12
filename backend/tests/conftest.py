import sys
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from app.main import app  # noqa: E402
from app.services import engine  # noqa: E402


@pytest.fixture()
def client() -> TestClient:
    # The service keeps one engine session; every test starts from an empty scene.
    if engine.available():
        engine.dispatch("new_scene", {})
    return TestClient(app)


def call(client: TestClient, name: str, **args) -> dict:
    reply = client.post("/api/engine/dispatch", json={"name": name, "args": args}).json()
    assert reply["ok"], reply
    return reply["result"]
