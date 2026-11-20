import os
import re
import subprocess
import sys

import pytest
from fastapi.testclient import TestClient
from starlette.websockets import WebSocketDisconnect

from family import BACKEND, INAPP_HEADER, WINDOW_KEY_HEADER, free_port, own_origin, window_headers

READS = {"GET", "HEAD", "OPTIONS"}
OPEN = {("POST", "/mcp"), ("POST", "/api/window/key")}
REFUSED = {"detail": "Only the formsculpt window can do this"}
WRONG = {WINDOW_KEY_HEADER: "x" * 43}
LOOPBACK = {"host": "127.0.0.1:8260"}
MCP_SESSION = {"mcp-protocol-version": "2025-11-25", "mcp-session-id": "outside-agent", "content-type": "application/json"}


@pytest.fixture()
def bare():
    from app.main import app

    with TestClient(app, base_url=own_origin()) as c:
        yield c


@pytest.fixture()
def tab_token():
    from app.services import agents

    token = agents.issue()
    yield token
    agents.revoke(token)


@pytest.fixture()
def fresh_consent():
    from app.services import agents

    agents.reset()
    yield agents
    agents.reset()


def write_operations():
    from app.main import app

    return [(m.upper(), path) for path, item in app.openapi()["paths"].items() for m in item if m.upper() not in READS]


def test_the_open_list_is_short_and_named():
    from app.core import window_key

    assert set(window_key.OPEN) == {"/mcp", "/api/window/key"}


def test_every_write_route_needs_the_window_key(bare):
    ops = write_operations()
    assert len(ops) >= 28
    answered = []
    for method, path in ops:
        if (method, path) in OPEN:
            continue
        url = re.sub(r"\{[^}]+\}", "x", path)
        for headers in ({}, WRONG):
            r = bare.request(method, url, json={}, headers=headers)
            if r.status_code != 403 or r.json() != REFUSED:
                answered.append(f"{method} {path} {r.status_code}")
    assert not answered, "write routes that answer without the window key: " + ", ".join(answered)


@pytest.mark.parametrize("caller", ["local program", "agent MCP session", "in-app agent tab", "wrong key"])
def test_no_caller_but_the_window_can_answer_the_agent_prompt(bare, tab_token, fresh_consent, caller):
    headers = {"local program": {}, "agent MCP session": MCP_SESSION, "in-app agent tab": {INAPP_HEADER: tab_token}, "wrong key": WRONG}[caller]
    r = bare.post("/api/mcp/consent", json={"allow": True}, headers=headers)
    assert r.status_code == 403 and r.json() == REFUSED
    assert fresh_consent.status()["allowed"] is None


@pytest.mark.parametrize("caller", ["local program", "agent MCP session", "in-app agent tab"])
def test_no_caller_but_the_window_can_open_a_terminal_or_run_a_command(bare, tab_token, caller):
    headers = {"local program": {}, "agent MCP session": MCP_SESSION, "in-app agent tab": {INAPP_HEADER: tab_token}}[caller]
    for path, body in (("/api/commands/terminal.open", {}), ("/api/commands/chat.send", {"text": "hello"}), ("/api/engine/dispatch", {"name": "new_scene", "args": {}})):
        r = bare.post(path, json=body, headers=headers)
        assert r.status_code == 403 and r.json() == REFUSED, path


def test_the_window_can_answer_the_agent_prompt(bare, fresh_consent):
    r = bare.post("/api/mcp/consent", json={"allow": False}, headers=window_headers())
    assert r.status_code == 200
    assert fresh_consent.status()["allowed"] is False


def keyed_reads():
    from app.core import window_key
    from app.main import app

    return [path for path, item in app.openapi()["paths"].items() if "get" in item and window_key.needs_key("GET", re.sub(r"\{[^}]+\}", "x", path))]


def test_every_keyed_read_needs_the_window_key(bare):
    paths = keyed_reads()
    assert {"/api/events", "/api/chat/image/{name}", "/api/mcp/window", "/api/events/recent"} <= set(paths)
    answered = []
    for path in paths:
        url = re.sub(r"\{[^}]+\}", "x.png", path)
        for headers in ({}, WRONG):
            r = bare.get(url, headers=headers)
            if r.status_code != 403 or r.json() != REFUSED:
                answered.append(f"GET {path} {r.status_code}")
    assert not answered, "keyed reads that answer without the window key: " + ", ".join(answered)


STREAM_CALLERS = ["local program", "agent MCP session", "in-app agent tab", "wrong key", "made-up ticket", "spent ticket"]


def stream_caller(caller, tab_token):
    from app.core import window_key

    if caller == "spent ticket":
        used = window_key.ticket()
        assert window_key.spend(used)
        return {}, used
    return {"local program": {}, "agent MCP session": MCP_SESSION, "in-app agent tab": {INAPP_HEADER: tab_token}, "wrong key": WRONG, "made-up ticket": {}}[caller], ("z" * 32 if caller == "made-up ticket" else "")


@pytest.mark.parametrize("caller", STREAM_CALLERS)
@pytest.mark.parametrize("path", ["/api/events", "/api/chat/image/x.png"])
def test_no_caller_but_the_window_can_read_chat_replies_or_images(bare, tab_token, caller, path):
    headers, given = stream_caller(caller, tab_token)
    r = bare.get(path, headers=headers, params={"ticket": given} if given else None)
    assert r.status_code == 403 and r.json() == REFUSED


def test_a_ticket_needs_the_window_key(bare):
    assert bare.post("/api/window/ticket", headers=WRONG).status_code == 403
    r = bare.post("/api/window/ticket", headers=window_headers())
    assert r.status_code == 200 and len(r.json()["ticket"]) >= 24


def test_a_chat_image_answers_the_window_and_a_ticket_once(bare):
    assert bare.get("/api/chat/image/x.png", headers=window_headers()).status_code == 404
    given = bare.post("/api/window/ticket", headers=window_headers()).json()["ticket"]
    first = bare.get("/api/chat/image/x.png", params={"ticket": given})
    assert first.status_code == 404 and first.json()["error"] == "No such chat image"
    again = bare.get("/api/chat/image/x.png", params={"ticket": given})
    assert again.status_code == 403 and again.json() == REFUSED


def test_a_ticket_older_than_its_life_is_refused(bare, monkeypatch):
    from app.core import window_key

    given = window_key.ticket()
    monkeypatch.setitem(window_key.TICKETS, given, window_key.TICKETS[given] - window_key.TICKET_LIFE - 1)
    assert bare.get("/api/chat/image/x.png", params={"ticket": given}).status_code == 403


@pytest.mark.parametrize("how", ["window key", "ticket"])
def test_the_event_stream_admits_the_window(how):
    import asyncio

    from app.core import window_key

    reached = []

    async def inner(scope, receive, send):
        reached.append(scope["path"])

    headers = [(window_key.HEADER.encode(), window_key.KEY.encode())] if how == "window key" else []
    query = f"ticket={window_key.ticket()}".encode() if how == "ticket" else b""
    scope = {"type": "http", "method": "GET", "path": "/api/events", "headers": headers, "query_string": query}
    asyncio.run(window_key.WindowKeyGuard(inner)(scope, None, None))
    assert reached == ["/api/events"]


def test_the_window_poll_needs_the_key(bare):
    assert bare.get("/api/mcp/window").status_code == 403
    assert bare.get("/api/mcp/window", headers=window_headers()).status_code == 200


def test_plain_reads_stay_open(bare):
    assert bare.get("/api/health").status_code == 200
    assert bare.get("/api/mcp/status").status_code == 200


def test_the_mcp_endpoint_stays_open_to_outside_agents(bare):
    r = bare.post("/mcp", json={"jsonrpc": "2.0", "id": 1, "method": "ping", "params": {}})
    assert r.status_code == 200 and r.json()["result"] == {}


def test_bootstrap_answers_once_then_refuses(bare, monkeypatch):
    from app.core import window_key

    monkeypatch.setitem(window_key.STATE, "claimed", False)
    monkeypatch.setitem(window_key.STATE, "secondClaim", False)
    first = bare.post("/api/window/key")
    assert first.status_code == 200 and first.json()["key"] == window_key.KEY
    assert bare.post("/api/window/key").status_code == 409
    assert bare.get("/api/window/key").json()["secondClaim"] is True


def test_bootstrap_does_not_exist_when_the_shell_reads_the_key(bare, monkeypatch):
    from app.core import window_key

    monkeypatch.setenv(window_key.STDOUT_ENV, "1")
    monkeypatch.setitem(window_key.STATE, "claimed", False)
    assert bare.post("/api/window/key").status_code == 404


@pytest.mark.parametrize("path", ["/api/terminal/none", "/api/engine/stroke"])
def test_a_socket_without_the_key_is_closed(bare, path):
    for first in ({"type": "key", "key": "y" * 43}, {"type": "input", "data": "true\r"}):
        with pytest.raises(WebSocketDisconnect) as refused:
            with bare.websocket_connect(path, headers=LOOPBACK) as ws:
                ws.send_json(first)
                ws.receive_text()
        assert refused.value.code == 4403


def test_a_terminal_socket_with_the_key_is_admitted(bare):
    with pytest.raises(WebSocketDisconnect) as closed:
        with bare.websocket_connect("/api/terminal/none", headers=LOOPBACK) as ws:
            ws.send_json({"type": "key", "key": window_headers()[WINDOW_KEY_HEADER]})
            ws.receive_text()
    assert closed.value.code == 4404


def test_a_stroke_socket_with_the_key_is_admitted(bare):
    with bare.websocket_connect("/api/engine/stroke", headers=LOOPBACK) as ws:
        ws.send_json({"type": "key", "key": window_headers()[WINDOW_KEY_HEADER]})
        ws.send_json({"type": "begin"})
        reply = ws.receive_json()
    assert reply["type"] == "error"


def engine_lines(tmp_path, **extra):
    env = {k: v for k, v in os.environ.items() if not k.startswith("FSC_")}
    env.update(FSC_DATA_DIR=str(tmp_path), FSC_PORT=str(free_port()), PYTHONUNBUFFERED="1", **extra)
    proc = subprocess.Popen([sys.executable, "formsculpt_engine.py"], cwd=BACKEND, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    lines = []
    try:
        while not any("http://127.0.0.1:" in line for line in lines):
            line = proc.stdout.readline()
            assert line, "".join(lines)
            lines.append(line)
    finally:
        proc.terminate()
        proc.wait(timeout=10)
    return lines


def test_engine_prints_the_key_line_once_before_its_address(tmp_path):
    lines = engine_lines(tmp_path, FSC_WINDOW_KEY_STDOUT="1")
    keyed = [i for i, line in enumerate(lines) if line.startswith("formsculpt-window-key: ")]
    assert len(keyed) == 1
    assert keyed[0] < len(lines) - 1
    assert len(lines[keyed[0]].split(": ", 1)[1].strip()) >= 32


def test_engine_started_from_source_prints_no_key(tmp_path):
    assert not any("window-key" in line for line in engine_lines(tmp_path))
