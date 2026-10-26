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


def positions(c, object_id):
    data = c.get(f"/api/engine/mesh/{object_id}").content
    vertices = int.from_bytes(data[4:8], "little")
    return data[20 : 20 + vertices * 12]


@pytest.mark.parametrize("brush", ["draw", "inflate", "grab", "smooth", "clay", "pinch", "crease", "flatten"])
def test_undo_after_a_stroke_restores_every_vertex_exactly(scene, brush):
    o = run(scene, "add_primitive", kind="sphere", detail=4)["object"]
    before = positions(scene, o["id"])
    run(scene, "sculpt_stroke", id=o["id"], brush=brush, points=[[0, 1, 0], [0.3, 0.95, 0.1]], radius=0.6, strength=1)
    assert positions(scene, o["id"]) != before
    run(scene, "undo")
    assert positions(scene, o["id"]) == before


def test_undo_after_dynamic_topology_restores_the_mesh(scene):
    o = run(scene, "add_primitive", kind="sphere", detail=3)["object"]
    before = positions(scene, o["id"])
    run(scene, "sculpt_stroke", id=o["id"], brush="draw", points=[[0, 1, 0], [0.2, 0.97, 0.1]], radius=0.5, strength=1, dyntopo=0.15)
    assert run(scene, "get_scene")["objects"][0]["triangles"] > o["triangles"]
    run(scene, "undo")
    now = run(scene, "get_scene")["objects"][0]
    assert now["triangles"] == o["triangles"] and positions(scene, o["id"]) == before


def test_undo_then_redo_gives_back_the_stroke(scene):
    o = run(scene, "add_primitive", kind="sphere", detail=4)["object"]
    run(scene, "sculpt_stroke", id=o["id"], brush="inflate", points=[[0, 1, 0]], radius=0.5, strength=1)
    stroked = positions(scene, o["id"])
    run(scene, "undo")
    run(scene, "redo")
    assert positions(scene, o["id"]) == stroked
