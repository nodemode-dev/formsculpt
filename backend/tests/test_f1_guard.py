import pytest
from starlette.websockets import WebSocketDisconnect

from family import ENGINE_PORT, HEALTH_PATH, WINDOW_ORIGINS, WS_PATH, own_origin

FOREIGN_ORIGINS = [
    "http://evil.example",
    "https://evil.example",
    f"http://localhost.evil.com:{ENGINE_PORT}",
    f"http://127.0.0.1.evil.com:{ENGINE_PORT}",
    "http://localhost:3099",
    "http://127.0.0.1:3099",
    "null",
    "file://",
]

FOREIGN_HOSTS = ["evil.example", f"evil.example:{ENGINE_PORT}", f"localhost.evil.com:{ENGINE_PORT}", f"192.168.1.10:{ENGINE_PORT}"]
LOOPBACK = f"127.0.0.1:{ENGINE_PORT}"


def test_foreign_origins_are_not_in_the_apps_own_list():
    assert not set(FOREIGN_ORIGINS) & set(WINDOW_ORIGINS)


def test_the_window_origin_list_is_the_one_the_app_ships():
    from app.core import config

    assert sorted(config.window_origins()) == sorted(WINDOW_ORIGINS)


@pytest.mark.parametrize("host", FOREIGN_HOSTS)
def test_foreign_host_is_refused(guard_client, host):
    assert guard_client.get(HEALTH_PATH, headers={"host": host}).status_code == 403


@pytest.mark.parametrize("origin", FOREIGN_ORIGINS)
def test_foreign_origin_is_refused(guard_client, origin):
    assert guard_client.get(HEALTH_PATH, headers={"origin": origin}).status_code == 403


@pytest.mark.parametrize("origin", FOREIGN_ORIGINS)
def test_foreign_origin_cannot_post(guard_client, origin):
    assert guard_client.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}}, headers={"origin": origin}).status_code == 403


def test_a_refused_request_changes_nothing(guard_client):
    from app.services import engine

    guard_client.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})
    guard_client.post("/api/engine/dispatch", json={"name": "add_primitive", "args": {"kind": "cube"}}, headers={"origin": "http://evil.example"})
    assert engine.object_count() == 0


def test_cross_site_request_without_origin_is_refused(guard_client):
    assert guard_client.get(HEALTH_PATH, headers={"sec-fetch-site": "cross-site"}).status_code == 403


@pytest.mark.parametrize("value", ["CROSS-SITE", " cross-site "])
def test_cross_site_marker_is_read_loosely(guard_client, value):
    assert guard_client.get(HEALTH_PATH, headers={"sec-fetch-site": value}).status_code == 403


@pytest.mark.parametrize("origin", [own_origin(), *WINDOW_ORIGINS])
def test_allowed_origin_passes(guard_client, origin):
    assert guard_client.get(HEALTH_PATH, headers={"origin": origin}).status_code == 200


@pytest.mark.parametrize("host", [f"localhost:{ENGINE_PORT}", f"[::1]:{ENGINE_PORT}"])
def test_own_origin_under_another_loopback_name_passes(guard_client, host):
    assert guard_client.get(HEALTH_PATH, headers={"host": host, "origin": f"http://{host}"}).status_code == 200


def test_own_origin_must_match_the_host_it_was_sent_to(guard_client):
    assert guard_client.get(HEALTH_PATH, headers={"host": LOOPBACK, "origin": f"http://127.0.0.1:{ENGINE_PORT + 1}"}).status_code == 403


@pytest.mark.parametrize("site", ["same-origin", "same-site", "none"])
def test_browser_request_without_origin_passes_unless_cross_site(guard_client, site):
    assert guard_client.get(HEALTH_PATH, headers={"sec-fetch-site": site}).status_code == 200


def test_plain_client_without_origin_passes(guard_client):
    assert guard_client.get(HEALTH_PATH).status_code == 200


@pytest.mark.parametrize("host", [LOOPBACK, f"localhost:{ENGINE_PORT}", f"[::1]:{ENGINE_PORT}"])
def test_loopback_hosts_pass(guard_client, host):
    assert guard_client.get(HEALTH_PATH, headers={"host": host}).status_code == 200


def test_testclient_default_host_is_not_loopback_and_is_refused():
    from fastapi.testclient import TestClient

    from app.main import app

    with TestClient(app) as bare:
        assert bare.get(HEALTH_PATH).status_code == 403


@pytest.mark.parametrize("headers", [{"host": "evil.example"}, {"host": LOOPBACK, "origin": "http://evil.example"}, {"host": LOOPBACK, "origin": "null"}, {"host": LOOPBACK, "origin": "http://localhost:3099"}])
def test_foreign_websocket_closes_4403(guard_client, headers):
    with pytest.raises(WebSocketDisconnect) as refused:
        with guard_client.websocket_connect(WS_PATH, headers=headers):
            pytest.fail("the websocket was accepted")
    assert refused.value.code == 4403


@pytest.mark.parametrize("origin", [own_origin(), *WINDOW_ORIGINS])
def test_loopback_websocket_with_an_allowed_origin_is_accepted(guard_client, origin):
    with guard_client.websocket_connect(WS_PATH, headers={"host": LOOPBACK, "origin": origin}) as ws:
        ws.send_json({"type": "end"})
