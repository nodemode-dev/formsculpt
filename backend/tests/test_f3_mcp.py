import json
import subprocess

from family import MCP_PATH, READ_TOOL, STDIO_CMD, tool_catalogue

HOST = {"host": "127.0.0.1:8260"}


def post(client, method, params=None, rid=1, headers=None):
    return client.post(MCP_PATH, json={"jsonrpc": "2.0", "id": rid, "method": method, "params": params or {}}, headers={**HOST, **(headers or {})})


def test_initialize_without_origin_answers(guard_client):
    r = post(guard_client, "initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "family-test", "version": "1"}})
    assert r.status_code == 200
    result = r.json()["result"]
    assert result["protocolVersion"]
    assert result["serverInfo"]["name"] == "formsculpt"
    assert "tools" in result["capabilities"]


def test_initialize_from_a_foreign_origin_is_refused(guard_client):
    assert post(guard_client, "initialize", headers={"origin": "http://evil.example"}).status_code == 403


def test_tools_list_matches_the_catalogue(guard_client):
    listed = post(guard_client, "tools/list").json()["result"]["tools"]
    catalogue = tool_catalogue()
    assert len(listed) == len(catalogue) > 100
    assert sorted(t["name"] for t in listed) == sorted(t["name"] for t in catalogue)
    for tool in listed:
        assert tool["description"].strip()
        assert tool["inputSchema"]["type"] == "object"


def test_read_only_tool_returns_a_valid_result(guard_client):
    name, args = READ_TOOL
    result = post(guard_client, "tools/call", {"name": name, "arguments": args}).json()["result"]
    assert not result.get("isError")
    assert result["content"] and all(c["type"] in ("text", "image", "resource", "resource_link") for c in result["content"])
    assert "objects" in json.loads(result["content"][0]["text"])


def test_unknown_method_is_a_jsonrpc_error(guard_client):
    assert post(guard_client, "no/such/method").json()["error"]["code"] == -32601


def test_a_bad_argument_comes_back_as_a_tool_error_not_a_crash(guard_client):
    from family import set_policy

    set_policy("allowed")
    try:
        result = post(guard_client, "tools/call", {"name": "add_primitive", "arguments": {"kind": "no_such_shape"}}).json()["result"]
    finally:
        set_policy("ask")
    assert result["isError"] is True


def test_stdio_relay_forwards_to_the_running_engine(engine_proc):
    env = dict(engine_proc["env"], FSC_MCP_URL=engine_proc["base"] + MCP_PATH, FSC_MCP_AUTOSTART="0")
    lines = [
        {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "family-test", "version": "1"}}},
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
    ]
    proc = subprocess.run(STDIO_CMD, input="".join(json.dumps(m) + "\n" for m in lines), capture_output=True, text=True, timeout=60, env=env)
    replies = {m.get("id"): m for m in (json.loads(l) for l in proc.stdout.splitlines() if l.strip().startswith("{"))}
    assert 2 in replies, f"relay gave no tools/list reply: {proc.stdout[-2000:]} {proc.stderr[-2000:]}"
    assert len(replies[2]["result"]["tools"]) == len(tool_catalogue())
