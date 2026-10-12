"""The engine service: its routes, the MCP endpoint and scripting, through FastAPI's test client."""

import io
import json
import struct
import zipfile

import pytest

from app import __version__
from app.services import engine
from tests.conftest import call

pytestmark = pytest.mark.skipif(not engine.available(), reason="the engine module is not built; run ./formsculptor engine build")


def test_version_and_health(client):
    assert client.get("/api/version").json() == {"app": "formsculptor", "version": __version__}
    assert client.get("/api/health").status_code == 200


def test_catalogue_matches_the_engine(client):
    cat = client.get("/api/engine/catalogue").json()
    assert cat["version"] == __version__
    names = {c["name"] for c in cat["commands"]}
    assert {"add_primitive", "sculpt_stroke", "record_animation", "cut_shape"} <= names


def test_dispatch_reports_errors_as_data(client):
    reply = client.post("/api/engine/dispatch", json={"name": "no_such_command", "args": {}}).json()
    assert reply == {"ok": False, "error": {"code": "unknown_command", "message": "no command named no_such_command"}}


def test_mesh_buffers_match_the_summary(client):
    o = call(client, "add_primitive", kind="sphere", detail=3)["object"]
    r = client.get(f"/api/engine/mesh/{o['id']}")
    assert r.status_code == 200
    assert len(r.content) > o["vertices"] * 24
    assert client.get("/api/engine/mesh/9999").status_code == 404


def test_smooth_preview_subdivides_for_display_only(client):
    o = call(client, "add_primitive", kind="cube")["object"]
    plain = client.get(f"/api/engine/mesh/{o['id']}").content
    for level in (1, 2):
        r = client.get(f"/api/engine/mesh/{o['id']}", params={"smooth": level})
        assert r.status_code == 200 and r.content[:4] == b"FSCM"
        _, indices, revision, flags = struct.unpack("<IIII", r.content[4:20])
        assert indices == o["triangles"] * 3 * 4**level
        assert revision == o["revision"] and flags & 1 == 0
    # The sculpted mesh is unchanged, and the level is bounded.
    assert client.get(f"/api/engine/mesh/{o['id']}").content == plain
    assert client.get(f"/api/engine/mesh/{o['id']}", params={"smooth": 4}).status_code == 422
    assert client.get("/api/engine/mesh/9999", params={"smooth": 1}).status_code == 404


def test_view_png(client):
    call(client, "add_primitive", kind="torus")
    r = client.get("/api/engine/view", params={"view": "front", "width": 96, "height": 96})
    assert r.status_code == 200 and r.content[:8] == b"\x89PNG\r\n\x1a\n"


def test_turntable_animation_and_history_routes(client):
    o = call(client, "add_primitive", kind="sphere", detail=3)["object"]
    call(client, "sculpt_stroke", id=o["id"], brush="inflate", points=[[0, 1, 0]], radius=0.5, strength=1)
    r = client.get("/api/engine/turntable", params={"frames": 8, "size": 64})
    assert r.status_code == 200 and r.content[:8] == b"\x89PNG\r\n\x1a\n"
    keys = [{"time": 0, "yaw": 0}, {"time": 0.5, "yaw": 90}]
    r = client.get("/api/engine/animation", params={"keys": json.dumps(keys), "fps": 8, "size": 64})
    assert r.status_code == 200 and b"acTL" in r.content[:200]
    assert client.get("/api/engine/animation", params={"keys": "not json"}).status_code == 400
    r = client.get("/api/engine/history_movie", params={"size": 64})
    assert r.status_code == 200 and r.content[:8] == b"\x89PNG\r\n\x1a\n"


def test_import_and_export_by_upload(client):
    call(client, "add_primitive", kind="cube", detail=2)
    r = client.get("/api/engine/export", params={"format": "stl"})
    assert r.status_code == 200
    up = client.post("/api/engine/import", files={"file": ("cube.stl", io.BytesIO(r.content), "application/octet-stream")})
    assert up.status_code == 200
    assert len(call(client, "get_scene")["objects"]) == 2


def _glb_json(data: bytes) -> dict:
    (length,) = struct.unpack("<I", data[12:16])
    return json.loads(data[20 : 20 + length])


def test_paint_has_seven_floats_and_old_projects_open_opaque(client):
    o = call(client, "add_primitive", kind="cube")["object"]
    call(client, "paint_fill", id=o["id"], color="#ff8800", opacity=0.5, emissive=0.25)
    mesh = client.get(f"/api/engine/mesh/{o['id']}").content
    vertices, indices, _, flags = struct.unpack("<IIII", mesh[4:20])
    paint_at = 20 + vertices * 24 + indices * 4 + vertices * 4
    first = struct.unpack("<7f", mesh[paint_at : paint_at + 28])
    assert flags & 2 and abs(first[5] - 0.5) < 1e-6 and abs(first[6] - 0.25) < 1e-6
    # A project from before 0.26.0 held five floats of paint per vertex; rewrite this one that way and open it.
    saved = client.get("/api/engine/project").content
    src, out = zipfile.ZipFile(io.BytesIO(saved)), io.BytesIO()
    with zipfile.ZipFile(out, "w") as dst:
        for item in src.infolist():
            data = src.read(item.filename)
            if item.filename.endswith("paint.f32"):
                floats = struct.unpack(f"<{len(data) // 4}f", data)
                data = b"".join(struct.pack("<5f", *floats[i : i + 5]) for i in range(0, len(floats), 7))
            dst.writestr(item, data)
    up = client.post("/api/engine/import", files={"file": ("old.fsc", io.BytesIO(out.getvalue()), "application/octet-stream")})
    assert up.status_code == 200 and up.json()["ok"], up.text
    doc = _glb_json(client.get("/api/engine/export", params={"format": "glb"}).content)
    assert "alphaMode" not in doc["materials"][0] and "emissiveFactor" not in doc["materials"][0]


def test_boolean_preview_draws_without_changing_the_scene(client):
    a = call(client, "add_primitive", kind="sphere", detail=3)["object"]["id"]
    b = call(client, "add_primitive", kind="cube")["object"]["id"]
    call(client, "transform_object", id=b, translate=[0.6, 0, 0])
    steps = len(call(client, "get_history")["steps"])
    r = client.get("/api/engine/boolean_preview", params={"a": a, "b": b, "op": "difference"})
    assert r.status_code == 200 and r.content[:4] == b"FSCM"
    vertices, indices, revision, flags = struct.unpack("<IIII", r.content[4:20])
    assert vertices > 0 and indices % 3 == 0 and revision == 0 and flags & 1
    assert len(call(client, "get_scene")["objects"]) == 2 and len(call(client, "get_history")["steps"]) == steps
    assert client.get("/api/engine/boolean_preview", params={"a": a, "b": 999, "op": "union"}).status_code == 404
    assert client.get("/api/engine/boolean_preview", params={"a": a, "b": b, "op": "xor"}).status_code == 422
    plane = call(client, "add_primitive", kind="plane")["object"]["id"]
    refused = client.get("/api/engine/boolean_preview", params={"a": plane, "b": b, "op": "union"})
    assert refused.status_code == 400 and "closed manifold" in refused.json()["detail"]


def test_path_render_and_substance_routes(client, tmp_path, monkeypatch):
    s = call(client, "add_primitive", kind="sphere", detail=3)["object"]["id"]
    r = client.get("/api/engine/view", params={"render": "path", "samples": 4, "width": 64, "height": 64})
    assert r.status_code == 200 and r.content[:8] == b"\x89PNG\r\n\x1a\n"
    assert client.get("/api/engine/view", params={"render": "raytrace"}).status_code == 422
    monkeypatch.setenv("FSC_SUBSTANCE_DIR", str(tmp_path))
    assert client.post("/api/engine/substance", json={"id": s}).json()["error"]["code"] == "no_uvs"
    call(client, "uv_unwrap", id=s)
    out = client.post("/api/engine/substance", json={"id": s}).json()
    assert out["ok"] and (tmp_path / "Sphere" / "Sphere_low.obj").exists()
    assert client.post("/api/engine/substance", json={"id": 999}).status_code == 404


def test_usda_export_route(client):
    call(client, "add_primitive", kind="cone")
    r = client.get("/api/engine/export", params={"format": "usda"})
    assert r.status_code == 200 and r.content.startswith(b"#usda 1.0")
    up = client.post("/api/engine/import", files={"file": ("cone.usda", io.BytesIO(r.content), "application/octet-stream")})
    assert up.status_code == 200 and up.json()["ok"]
    assert len(call(client, "get_scene")["objects"]) == 2


def test_fbx_export_route(client):
    call(client, "add_primitive", kind="torus")
    r = client.get("/api/engine/export", params={"format": "fbx"})
    assert r.status_code == 200 and r.content.startswith(b"Kaydara FBX Binary  \x00")
    up = client.post("/api/engine/import", files={"file": ("torus.fbx", io.BytesIO(r.content), "application/octet-stream")})
    assert up.status_code == 200
    assert [o["vertices"] for o in call(client, "get_scene")["objects"]] == [2048, 2048]


def test_mcp_lists_and_calls_tools(client):
    headers = {"origin": "http://127.0.0.1:5196", "mcp-protocol-version": "2025-11-25"}
    init = client.post("/mcp", json={"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-11-25"}}, headers=headers).json()
    assert init["result"]["serverInfo"]["name"] == "formsculptor"
    tools = client.post("/mcp", json={"jsonrpc": "2.0", "id": 2, "method": "tools/list"}, headers=headers).json()["result"]["tools"]
    assert len(tools) == len(engine.catalogue()["commands"])
    scene = client.post("/mcp", json={"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "get_scene", "arguments": {}}}, headers=headers).json()
    assert not scene["result"].get("isError")
    view = client.post("/mcp", json={"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "get_view", "arguments": {"width": 64, "height": 64}}}, headers=headers).json()
    assert view["result"]["content"][0]["type"] == "image"
    missing = client.post("/mcp", json={"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "nope", "arguments": {}}}, headers=headers).json()
    assert missing["error"]["code"] == -32602


def test_mcp_refuses_other_origins(client):
    r = client.post("/mcp", json={"jsonrpc": "2.0", "id": 1, "method": "ping"}, headers={"origin": "https://example.com"})
    assert r.status_code == 403


def test_plugins_declare_fields_and_tools(client):
    plugins = {p["name"]: p for p in client.get("/api/plugins").json()["plugins"]}
    assert set(plugins["rock"]["params"]) == {"size", "seed"} and plugins["rock"]["click"] is False
    assert plugins["rivets"]["click"] is True and plugins["rivets"]["params"]["shape"]["options"] == ["sphere", "cylinder", "cube"]
    o = call(client, "add_primitive", kind="sphere", detail=4)["object"]
    hit = call(client, "pick", origin=[0, 0, 5], direction=[0, 0, -1])
    ran = client.post("/api/plugins/run", json={"name": "rivets", "args": {"point": hit["point"], "normal": hit["normal"], "count": 6}}).json()
    assert ran["ok"] and ran["result"]["copies"] == 6, ran
    assert o["id"] in {x["id"] for x in call(client, "get_scene")["objects"]}


def test_bridge_route_writes_to_the_bridge_folder(client, tmp_path, monkeypatch):
    monkeypatch.setenv("FSC_BRIDGE_DIR", str(tmp_path))
    o = call(client, "add_primitive", kind="cube", name="Block A")["object"]
    reply = client.post("/api/engine/bridge", json={"id": o["id"], "format": "obj"}).json()
    assert reply["ok"], reply
    assert reply["result"]["path"] == str(tmp_path / f"Block_A-{o['id']}.obj")
    assert (tmp_path / f"Block_A-{o['id']}.obj").stat().st_size > 0
    assert client.post("/api/engine/bridge", json={"id": 9999}).status_code == 404


def test_plugins_list_and_run(client):
    listed = {p["name"] for p in client.get("/api/plugins").json()["plugins"]}
    assert {"rock", "pedestal", "rivets"} <= listed
    # With nothing selected the pedestal refuses; after the rock (now active) it stands it on one.
    refused = client.post("/api/plugins/run", json={"name": "pedestal", "args": {}}).json()
    assert refused["ok"] is False
    ran = client.post("/api/plugins/run", json={"name": "rock", "args": {}}).json()
    assert ran["ok"], ran
    assert client.post("/api/plugins/run", json={"name": "pedestal", "args": {}}).json()["ok"]
    assert len(call(client, "get_scene")["objects"]) == 2
    assert client.post("/api/plugins/run", json={"name": "missing", "args": {}}).status_code == 404
