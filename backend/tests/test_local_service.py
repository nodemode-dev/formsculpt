"""The service stays on this machine and answers bad requests as bad requests: serve and boot listen on loopback only
whatever the environment says, the export route and the MCP endpoint answer malformed input with 400 (never 500),
stroke samples that are not numbers leave the mesh finite, and a NodeMode send whose target accepts and never
answers does not hold up other requests."""

import json
import math
import os
import signal
import socket
import struct
import subprocess
import sys
import threading
import time

import httpx
import pytest
from fastapi.testclient import TestClient

from family import BACKEND, HEALTH_PATH, MCP_PATH, REPO, WINDOW_ORIGINS, WS_PATH, free_port, own_origin
from app.services import engine

needs_engine = pytest.mark.skipif(not engine.available(), reason="the engine module is not built; run ./formsculpt engine build")
needs_bash = pytest.mark.skipif(sys.platform == "win32", reason="serve and boot are bash scripts here; the .bat twins are not run")
HOSTILE = {"FSC_HOST": "0.0.0.0", "HOST": "0.0.0.0", "UVICORN_HOST": "0.0.0.0", "FSC_BIND": "0.0.0.0", "FSC_LISTEN": "0.0.0.0"}
MCP_HEADERS = {"host": "127.0.0.1:8260"}


def other_address():
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
        try:
            s.connect(("192.0.2.1", 9))
            address = s.getsockname()[0]
        except OSError:
            return None
    return None if address.startswith("127.") or address == "0.0.0.0" else address


def reachable(host, port):
    try:
        with socket.create_connection((host, port), timeout=1):
            return True
    except OSError:
        return False


def started(cmd, tmp_path):
    port = free_port()
    data = tmp_path / "data"
    data.mkdir()
    env = dict(os.environ, **HOSTILE, FSC_PORT=str(port), FSC_DATA_DIR=str(data), HOME=str(tmp_path))
    proc = subprocess.Popen(cmd, cwd=REPO, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline and proc.poll() is None:
        try:
            if httpx.get(f"http://127.0.0.1:{port}{HEALTH_PATH}", timeout=1).status_code == 200:
                return proc, port
        except httpx.HTTPError:
            pass
        time.sleep(0.2)
    stop(proc)
    raise AssertionError(f"{cmd} did not answer on 127.0.0.1:{port} within 90 s")


def stop(proc):
    try:
        os.killpg(proc.pid, signal.SIGTERM)
        proc.wait(timeout=15)
    except ProcessLookupError:
        pass
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid, signal.SIGKILL)
        proc.wait(timeout=5)


@needs_bash
@pytest.mark.parametrize("script", ["serve", "boot"])
def test_serve_and_boot_listen_on_loopback_only_whatever_the_environment_says(tmp_path, script):
    other = other_address()
    if other is None:
        pytest.skip("this machine has no address besides loopback to try")
    if script == "serve":
        if not (REPO / "frontend" / "dist" / "index.html").exists():
            pytest.skip("serve builds the window first when frontend/dist is missing; build it to run this")
        cmd = ["bash", str(REPO / "formsculpt"), "serve"]
    else:
        cmd = ["bash", str(BACKEND / "boot_backend")]
    proc, port = started(cmd, tmp_path)
    try:
        assert reachable("127.0.0.1", port)
        assert not reachable(other, port), f"{script} answers on {other}:{port}"
    finally:
        stop(proc)


@pytest.fixture()
def lenient():
    from app.main import app

    with TestClient(app, base_url=own_origin(), raise_server_exceptions=False) as c:
        c.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})
        yield c
        c.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})


@needs_engine
@pytest.mark.parametrize("ids", ["abc", "1,abc", "1.5", "one", "[1]", "1;2"])
def test_an_export_with_ids_that_are_not_numbers_is_a_400_with_a_reason(lenient, ids):
    lenient.post("/api/engine/dispatch", json={"name": "add_primitive", "args": {"kind": "cube"}})
    r = lenient.get("/api/engine/export", params={"format": "stl", "ids": ids})
    assert r.status_code == 400 and r.json()["detail"]
    assert lenient.get(HEALTH_PATH).status_code == 200


@needs_engine
@pytest.mark.parametrize("ids", [
    "-1",
    "0",
    "99999999999999999999",
    "4294967297",
    "9999",
])
def test_an_export_with_ids_out_of_range_is_refused_and_never_a_500(lenient, ids):
    lenient.post("/api/engine/dispatch", json={"name": "add_primitive", "args": {"kind": "cube"}})
    r = lenient.get("/api/engine/export", params={"format": "stl", "ids": ids})
    assert 400 <= r.status_code < 500 and r.json()["detail"], (r.status_code, r.text[:200])
    assert lenient.get(HEALTH_PATH).status_code == 200


def mcp(c, body):
    if isinstance(body, (bytes, str)):
        return c.post(MCP_PATH, content=body, headers={**MCP_HEADERS, "content-type": "application/json"})
    return c.post(MCP_PATH, json=body, headers=MCP_HEADERS)


@pytest.mark.parametrize("body, code", [
    (b"not json", -32700),
    (b"{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": ", -32700),
    ([], -32600),
    ([{"jsonrpc": "2.0", "id": 1, "method": "ping"}], -32600),
    ("\"a string\"", -32600),
    ({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": [1]}, -32602),
    ({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": "get_scene"}, -32602),
    ({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": [{"name": "get_scene"}]}, -32602),
])
def test_a_malformed_mcp_request_is_a_400_with_a_jsonrpc_error(guard_client, body, code):
    r = mcp(guard_client, body)
    assert r.status_code == 400, (r.status_code, r.text[:200])
    assert r.json()["error"]["code"] == code and r.json()["error"]["message"]


@pytest.mark.parametrize("params", [
    {"name": ["get_scene"], "arguments": {}},
    {"name": {"tool": "get_scene"}, "arguments": {}},
    {"name": None},
    {"name": "get_scene", "arguments": [1]},
    {"name": "get_scene", "arguments": "x"},
])
def test_a_tools_call_with_a_name_or_arguments_of_the_wrong_kind_is_a_jsonrpc_error_not_a_500(guard_client, params):
    r = mcp(guard_client, {"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": params})
    assert r.status_code in (200, 400), (r.status_code, r.text[:200])
    assert r.json()["id"] == 7 and r.json()["error"]["code"] == -32602
    assert guard_client.get(HEALTH_PATH).status_code == 200


def test_a_tools_call_whose_arguments_are_not_an_object_says_so(guard_client):
    r = mcp(guard_client, {"jsonrpc": "2.0", "id": 8, "method": "tools/call", "params": {"name": "get_scene", "arguments": [1]}})
    error = r.json()["error"]
    assert error["code"] == -32602 and "arguments" in error["message"] and "No tool named" not in error["message"]


def finite_positions(c, object_id):
    data = c.get(f"/api/engine/mesh/{object_id}").content
    vertices = int.from_bytes(data[4:8], "little")
    values = struct.unpack(f"<{vertices * 3}f", data[20 : 20 + vertices * 12])
    return vertices > 0 and all(math.isfinite(v) for v in values)


@needs_engine
@pytest.mark.parametrize("brush", ["draw", "grab", "inflate"])
def test_stroke_socket_samples_that_are_not_numbers_leave_the_mesh_finite(lenient, brush):
    made = lenient.post("/api/engine/dispatch", json={"name": "add_primitive", "args": {"kind": "sphere", "detail": 3}}).json()
    oid = made["result"]["object"]["id"]
    samples = [
        '{"type":"sample","seq":1,"origin":[0,0,5],"dir":[0,0,-1],"pressure":NaN}',
        '{"type":"sample","seq":2,"origin":[NaN,0,5],"dir":[0,0,-1],"pressure":1}',
        '{"type":"sample","seq":3,"origin":[0.1,0,5],"dir":[0,0,-1],"pressure":1}',
        '{"type":"sample","seq":4,"origin":[0.3,0,5],"dir":[0,0,-1],"pressure":NaN}',
        '{"type":"sample","seq":5,"origin":[NaN,0,5],"dir":[0,0,-1],"pressure":1}',
        '{"type":"sample","seq":6,"origin":[0.5,0,5],"dir":[0,Infinity,-1],"pressure":1}',
        '{"type":"sample","seq":7,"origin":[0.6,0,5],"dir":[0,0,-1],"pressure":Infinity}',
        '{"type":"sample","seq":8,"origin":[0.7,0,5],"dir":[0,0,-1],"pressure":1}',
    ]
    with lenient.websocket_connect(WS_PATH, headers={**MCP_HEADERS, "origin": WINDOW_ORIGINS[1]}) as ws:
        ws.send_text(json.dumps({"type": "begin", "id": oid, "brush": {"brush": brush, "radius": 0.4, "strength": 1}}))
        for sample in samples:
            ws.send_text(sample)
        ws.send_text('{"type":"end"}')
        for _ in range(100):
            message = ws.receive()
            if message.get("text") and json.loads(message["text"]).get("type") == "end":
                break
        else:
            pytest.fail("the stroke never ended")
    assert finite_positions(lenient, oid)
    assert lenient.get(HEALTH_PATH).status_code == 200


class Silent:
    """A local socket that accepts connections and never reads or answers."""

    def __init__(self):
        self.sock = socket.socket()
        self.sock.bind(("127.0.0.1", 0))
        self.sock.listen(8)
        self.sock.settimeout(0.2)
        self.port = self.sock.getsockname()[1]
        self.accepted = []
        self.done = threading.Event()
        self.thread = threading.Thread(target=self._run, daemon=True)
        self.thread.start()

    def _run(self):
        while not self.done.is_set():
            try:
                conn, _ = self.sock.accept()
                self.accepted.append(conn)
            except OSError:
                pass

    def close(self):
        self.done.set()
        self.thread.join(timeout=5)
        for conn in self.accepted:
            conn.close()
        self.sock.close()


@pytest.fixture()
def silent():
    s = Silent()
    yield s
    s.close()


def wait_for(condition, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if condition():
            return True
        time.sleep(0.05)
    return False


@needs_engine
def test_a_nodemode_send_to_a_target_that_never_answers_does_not_hold_up_other_requests(engine_proc, silent):
    base = engine_proc["base"]
    with httpx.Client(base_url=base, timeout=10) as c:
        c.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})
        assert c.post("/api/engine/dispatch", json={"name": "add_primitive", "args": {"kind": "cube", "detail": 1}}).json()["ok"]
    outcome = {}

    def send():
        with httpx.Client(base_url=base, timeout=120) as c:
            outcome["response"] = c.post("/api/nodemode/send", json={"target": f"http://127.0.0.1:{silent.port}", "title": "silent"})

    sender = threading.Thread(target=send, daemon=True)
    sender.start()
    try:
        assert wait_for(lambda: silent.accepted, 60), "the send never reached the silent target"
        with httpx.Client(base_url=base, timeout=10) as c:
            for _ in range(5):
                t = time.monotonic()
                assert c.get(HEALTH_PATH).status_code == 200
                assert c.post("/api/engine/dispatch", json={"name": "get_scene", "args": {}}).json()["ok"]
                assert time.monotonic() - t < 2, "another request waited on the send"
            assert sender.is_alive(), "the send ended before the silent target could hold it"
        sender.join(timeout=90)
        assert not sender.is_alive(), "the send did not give up on a target that never answers"
        r = outcome["response"]
        assert r.status_code == 502 and "NodeMode" in r.json()["detail"]
    finally:
        sender.join(timeout=90)
        with httpx.Client(base_url=base, timeout=10) as c:
            c.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})
    assert engine_proc["proc"].poll() is None


@needs_engine
def test_nodemode_status_of_a_target_that_never_answers_says_it_is_not_running(guard_client, silent):
    t = time.monotonic()
    r = guard_client.get("/api/nodemode/status", params={"target": f"http://127.0.0.1:{silent.port}"})
    assert r.status_code == 200 and r.json()["running"] is False and r.json()["detail"]
    assert time.monotonic() - t < 30
