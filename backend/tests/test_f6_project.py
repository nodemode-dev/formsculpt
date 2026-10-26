import io
import struct
import zipfile

import pytest


@pytest.fixture()
def scene(guard_client):
    guard_client.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})
    yield guard_client
    guard_client.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})


def run(c, name, /, **args):
    reply = c.post("/api/engine/dispatch", json={"name": name, "args": args}).json()
    assert reply["ok"], reply
    return reply["result"]


def entries(data):
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        return {i.filename: z.read(i.filename) for i in z.infolist()}


def body(mesh):
    vertices, indices, _, flags = struct.unpack("<IIII", mesh[4:20])
    return (vertices, indices, flags, mesh[20:])


def reopen(c, data):
    up = c.post("/api/engine/import", files={"file": ("work.fsc", io.BytesIO(data), "application/octet-stream")})
    assert up.status_code == 200 and up.json()["ok"], up.text


def build(c):
    sphere = run(c, "add_primitive", kind="sphere", detail=4)["object"]["id"]
    run(c, "sculpt_stroke", id=sphere, brush="inflate", points=[[0, 1, 0]], radius=0.5, strength=1)
    run(c, "sculpt_stroke", id=sphere, brush="mask", points=[[1, 0, 0]], radius=0.4, strength=1)
    run(c, "paint_fill", id=sphere, color="#2060ff", opacity=0.8)
    run(c, "layer_add", id=sphere, name="Detail")
    run(c, "sculpt_stroke", id=sphere, brush="draw", points=[[0, 0, 1]], radius=0.3, strength=1)
    torus = run(c, "add_primitive", kind="torus", detail=3)["object"]["id"]
    run(c, "transform_object", id=torus, translate=[3, 0, 0])
    run(c, "multires_divide", id=torus)
    return sphere, torus


def test_save_open_save_gives_the_same_project(scene):
    build(scene)
    first = scene.get("/api/engine/project").content
    reopen(scene, first)
    second = scene.get("/api/engine/project").content
    a, b = entries(first), entries(second)
    assert sorted(a) == sorted(b)
    assert {k: v for k, v in a.items() if v != b[k]} == {}


def test_the_reopened_scene_equals_the_original(scene):
    sphere, torus = build(scene)
    before = run(scene, "get_scene")["objects"]
    meshes = {o["id"]: body(scene.get(f"/api/engine/mesh/{o['id']}").content) for o in before}
    reopen(scene, scene.get("/api/engine/project").content)
    after = run(scene, "get_scene")["objects"]
    assert [o["id"] for o in after] == [o["id"] for o in before]
    for a, b in zip(before, after):
        for key in ("name", "vertices", "triangles", "bounds", "layers"):
            assert a[key] == b[key], key
        assert body(scene.get(f"/api/engine/mesh/{b['id']}").content) == meshes[b["id"]]


def test_a_second_open_does_not_pile_up_objects(scene):
    build(scene)
    data = scene.get("/api/engine/project").content
    for _ in range(2):
        reopen(scene, data)
    assert len(run(scene, "get_scene")["objects"]) == 2


def test_a_truncated_project_is_refused_and_leaves_the_scene(scene):
    build(scene)
    data = scene.get("/api/engine/project").content
    before = scene.get("/api/engine/project").content
    up = scene.post("/api/engine/import", files={"file": ("broken.fsc", io.BytesIO(data[: len(data) // 2]), "application/octet-stream")})
    assert up.status_code != 200 or not up.json()["ok"]
    assert len(run(scene, "get_scene")["objects"]) == 2
    assert entries(scene.get("/api/engine/project").content) == entries(before)
