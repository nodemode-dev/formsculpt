import threading
import time

import pytest

from family import CHANGING_TOOL, DECLINED_TEXT, INAPP_HEADER, MCP_PATH, NO_WINDOW_TEXT, READ_TOOL, inapp_token, set_policy

HOST = {"host": "127.0.0.1:8260"}


def rpc(client, method, params=None, headers=None, rid=1):
    r = client.post(MCP_PATH, json={"jsonrpc": "2.0", "id": rid, "method": method, "params": params or {}}, headers={**HOST, **(headers or {})})
    assert r.status_code == 200, r.text
    return r.json()


def call(client, tool, headers=None):
    name, args = tool
    return rpc(client, "tools/call", {"name": name, "arguments": args}, headers, rid=2)


def text_of(reply):
    result = reply.get("result") or {}
    return " ".join(c.get("text", "") for c in result.get("content", []) if isinstance(c, dict))


def refused(reply, needle):
    return needle in text_of(reply) and bool(reply["result"].get("isError"))


@pytest.fixture(autouse=True)
def policy_reset():
    from app.services import engine

    set_policy("ask")
    yield
    set_policy("ask")
    engine.dispatch("new_scene", {})


def test_read_only_tool_never_asks(guard_client):
    reply = call(guard_client, READ_TOOL)
    assert "error" not in reply
    assert not reply["result"].get("isError")


def test_changing_tool_with_no_window_is_refused(guard_client):
    assert refused(call(guard_client, CHANGING_TOOL), NO_WINDOW_TEXT)


def test_a_refused_changing_tool_does_not_run(guard_client):
    from app.services import engine

    call(guard_client, ("add_primitive", {"kind": "cube"}))
    assert engine.object_count() == 0


def test_blocked_policy_refuses_a_changing_tool(guard_client):
    set_policy("blocked")
    assert refused(call(guard_client, CHANGING_TOOL), DECLINED_TEXT)


def test_a_decline_blocks_every_changing_tool_for_the_cooldown(guard_client):
    from app.services import engine

    set_policy("blocked")
    for tool in (CHANGING_TOOL, ("add_primitive", {"kind": "sphere"}), ("new_scene", {})):
        assert refused(call(guard_client, tool), DECLINED_TEXT)
    assert engine.object_count() == 0


def test_a_decline_still_lets_a_read_only_tool_through(guard_client):
    set_policy("blocked")
    assert not call(guard_client, READ_TOOL)["result"].get("isError")


@pytest.mark.parametrize("token", ["", "wrong-token", "x" * 64])
def test_blocked_policy_is_not_skipped_by_a_wrong_token(guard_client, token):
    set_policy("blocked")
    assert refused(call(guard_client, CHANGING_TOOL, {INAPP_HEADER: token}), DECLINED_TEXT)


def test_allowed_policy_lets_a_changing_tool_through(guard_client):
    from app.services import engine

    set_policy("allowed")
    reply = call(guard_client, ("add_primitive", {"kind": "cube"}))
    assert not reply["result"].get("isError")
    assert engine.object_count() == 1


def test_inapp_token_skips_the_ask(guard_client):
    from app.services import engine

    reply = call(guard_client, ("add_primitive", {"kind": "cube"}), {INAPP_HEADER: inapp_token()})
    assert not reply["result"].get("isError")
    assert engine.object_count() == 1


@pytest.mark.parametrize("token", ["", "wrong-token", "x" * 64])
def test_wrong_inapp_token_does_not_skip_the_ask(guard_client, token):
    assert refused(call(guard_client, CHANGING_TOOL, {INAPP_HEADER: token}), NO_WINDOW_TEXT)


def test_the_window_answering_yes_lets_the_waiting_call_through(guard_client):
    from app.services import engine

    guard_client.get("/api/mcp/window")
    out = {}
    worker = threading.Thread(target=lambda: out.update(call(guard_client, ("add_primitive", {"kind": "cube"}))), daemon=True)
    worker.start()
    deadline = time.monotonic() + 20
    asked = None
    while time.monotonic() < deadline and not asked:
        asked = guard_client.get("/api/mcp/window").json()["consent"]
        time.sleep(0.05)
    assert asked and asked["tool"] == "add_primitive"
    assert engine.object_count() == 0
    guard_client.post("/api/mcp/consent", json={"allow": True})
    worker.join(20)
    assert not out["result"].get("isError")
    assert engine.object_count() == 1


def test_the_window_answering_no_refuses_and_starts_the_cooldown(guard_client):
    guard_client.get("/api/mcp/window")
    out = {}
    worker = threading.Thread(target=lambda: out.update(call(guard_client, ("add_primitive", {"kind": "cube"}))), daemon=True)
    worker.start()
    deadline = time.monotonic() + 20
    asked = None
    while time.monotonic() < deadline and not asked:
        asked = guard_client.get("/api/mcp/window").json()["consent"]
        time.sleep(0.05)
    assert asked
    guard_client.post("/api/mcp/consent", json={"allow": False})
    worker.join(20)
    assert refused(out, "declined")
    guard_client.get("/api/mcp/window")
    assert refused(call(guard_client, CHANGING_TOOL), DECLINED_TEXT)
