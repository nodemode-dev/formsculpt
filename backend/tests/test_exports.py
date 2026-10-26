import io
import json
import struct

import httpx
import pytest

KINDS = ["sphere", "uv_sphere", "cube", "cylinder", "cone", "torus"]


@pytest.fixture()
def scene(guard_client):
    guard_client.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})
    yield guard_client
    guard_client.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})


def run(c, name, /, **args):
    reply = c.post("/api/engine/dispatch", json={"name": name, "args": args}).json()
    assert reply["ok"], reply
    return reply["result"]


def add(c, kind):
    return run(c, "add_primitive", kind=kind, detail=2)["object"]


@pytest.mark.parametrize("kind", KINDS)
@pytest.mark.parametrize("fmt", ["stl", "obj", "glb"])
def test_an_exported_primitive_reopens_with_its_triangle_count(scene, kind, fmt):
    made = add(scene, kind)
    exported = scene.get("/api/engine/export", params={"format": fmt})
    assert exported.status_code == 200 and exported.content
    run(scene, "delete_object", id=made["id"])
    up = scene.post("/api/engine/import", files={"file": (f"{kind}.{fmt}", io.BytesIO(exported.content), "application/octet-stream")})
    assert up.status_code == 200 and up.json()["ok"], up.text
    objects = run(scene, "get_scene")["objects"]
    assert len(objects) == 1
    assert objects[0]["triangles"] == made["triangles"]


@pytest.mark.parametrize("kind", KINDS)
def test_binary_stl_is_the_size_its_triangle_count_says(scene, kind):
    made = add(scene, kind)
    data = scene.get("/api/engine/export", params={"format": "stl"}).content
    (count,) = struct.unpack("<I", data[80:84])
    assert count == made["triangles"]
    assert len(data) == 84 + 50 * count


def test_stl_export_does_not_take_the_engine_down(engine_proc):
    base = engine_proc["base"]
    with httpx.Client(base_url=base, timeout=60) as c:
        assert c.post("/api/engine/dispatch", json={"name": "add_primitive", "args": {"kind": "torus", "detail": 2}}).json()["ok"]
        r = c.get("/api/engine/export", params={"format": "stl"})
        assert r.status_code == 200 and len(r.content) > 84
        assert c.get("/api/health").status_code == 200
    assert engine_proc["proc"].poll() is None


def test_gltf_export_names_the_triangles_it_wrote(scene):
    made = add(scene, "cone")
    data = scene.get("/api/engine/export", params={"format": "glb"}).content
    (length,) = struct.unpack("<I", data[12:16])
    doc = json.loads(data[20 : 20 + length])
    primitive = doc["meshes"][0]["primitives"][0]
    assert doc["accessors"][primitive["indices"]]["count"] == made["triangles"] * 3


def test_export_of_an_empty_scene_is_an_error_not_a_crash(scene):
    r = scene.get("/api/engine/export", params={"format": "stl"})
    assert r.status_code in (200, 400)
    assert scene.get("/api/health").status_code == 200
