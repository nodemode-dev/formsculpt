"""The Home screen's library through its routes: sculpts made from templates and from the scene, saved, opened,
duplicated, moved to the Trash, restored, deleted for good, and names that must never reach outside the library."""

from urllib.parse import quote

import pytest

from app.services import engine
from tests.conftest import call

pytestmark = pytest.mark.skipif(not engine.available(), reason="the engine module is not built; run ./formsculpt engine build")

PICTURES = (b"\x89PNG\r\n\x1a\n", b"\xff\xd8\xff", b"RIFF")
BAD_TITLES = ["../evil", "..", "...", ".", "a/b", "a\\b", "/abs", "con", "CON.txt", "nul", "x" * 65]
BAD_IDS = ["..", "...", "%2E%2E", "..%2Fevil", "%2E%2E%2F%2E%2E%2Fkeep", "a%2Fb", "a%5Cb", "con"]


@pytest.fixture()
def lib(client, tmp_path, monkeypatch):
    monkeypatch.setenv("FSC_DATA_DIR", str(tmp_path / "data"))
    yield client
    client.post("/api/engine/dispatch", json={"name": "new_scene", "args": {}})


def project(tmp_path):
    return tmp_path / "data" / "LightBox" / "Project"


def outside(tmp_path):
    root = project(tmp_path)
    return sorted(str(p.relative_to(tmp_path)) for p in tmp_path.rglob("*") if p.is_file() and not p.is_relative_to(root))


def ok(r):
    assert r.status_code == 200, r.text
    return r.json()


def make(c, title, template="cube"):
    return ok(c.post("/api/sculpts", json={"title": title, "template": template}))


def listed(c):
    return {s["id"]: s for s in ok(c.get("/api/sculpts"))["sculpts"]}


def trashed(c):
    return ok(c.get("/api/sculpt-trash"))["sculpts"]


def scene(c):
    return [(o["name"], o["vertices"], o["triangles"], o["bounds"]) for o in call(c, "get_scene")["objects"]]


def opened(c, sid):
    return ok(c.post(f"/api/sculpts/{quote(sid)}/open"))


def skip_without_rename(r):
    if r.status_code == 405 or (r.status_code == 404 and r.json().get("detail") == "Not Found"):
        pytest.skip("this tree has no rename route")


def test_an_empty_library_lists_nothing_and_its_folder_is_in_the_data_folder(lib, tmp_path):
    body = ok(lib.get("/api/sculpts"))
    assert body["sculpts"] == [] and trashed(lib) == []
    assert body["folder"] == str(project(tmp_path))


def test_every_template_makes_a_sculpt_that_lists_with_a_preview_and_opens(lib):
    templates = ok(lib.get("/api/sculpt-templates"))["templates"]
    assert len(templates) >= 5
    for t in templates:
        call(lib, "new_scene")
        made = make(lib, f"From {t['id']}", t["id"])
        assert made["template"] == t["id"] and made["objects"] >= 1 and made["thumb"] is True, t["id"]
        picture = lib.get(f"/api/sculpts/{quote(made['id'])}/thumb")
        assert picture.status_code == 200 and picture.content.startswith(PICTURES), t["id"]
        assert scene(lib) == [], f"{t['id']}: making a sculpt changed the scene on screen"
        opened(lib, made["id"])
        assert len(scene(lib)) == made["objects"], t["id"]
    assert set(listed(lib)) == {f"From {t['id']}" for t in templates}


def test_an_unknown_template_is_refused_with_404_and_makes_nothing(lib):
    assert lib.post("/api/sculpts", json={"title": "x", "template": "no_such_template"}).status_code == 404
    assert listed(lib) == {}


def test_a_saved_sculpt_opens_again_with_the_edited_mesh(lib):
    made = make(lib, "Edited", "sphere")
    opened(lib, made["id"])
    first = call(lib, "get_scene")["objects"][0]["id"]
    call(lib, "sculpt_stroke", id=first, brush="inflate", points=[[0, 1, 0]], radius=0.5, strength=1)
    call(lib, "add_primitive", kind="torus", detail=2)
    edited = scene(lib)
    ok(lib.post(f"/api/sculpts/{quote(made['id'])}/save"))
    call(lib, "new_scene")
    assert scene(lib) == []
    opened(lib, made["id"])
    assert scene(lib) == edited
    assert listed(lib)["Edited"]["objects"] == 2


def test_saving_the_scene_as_a_new_sculpt_lists_it_with_its_object_count(lib):
    call(lib, "add_primitive", kind="cube")
    call(lib, "add_primitive", kind="cone")
    made = ok(lib.post("/api/sculpts", json={"title": "Two parts", "fromScene": True}))
    assert made["id"] == "Two parts" and listed(lib)["Two parts"]["objects"] == 2


def test_saving_the_scene_with_no_title_still_gets_a_name_and_a_second_one_does_not_overwrite_it(lib):
    call(lib, "add_primitive", kind="cube")
    first = ok(lib.post("/api/sculpts", json={"fromScene": True}))
    call(lib, "add_primitive", kind="torus")
    second = ok(lib.post("/api/sculpts", json={"title": "  ", "fromScene": True}))
    assert first["id"].strip() and second["id"] != first["id"]
    sculpts = listed(lib)
    assert set(sculpts) == {first["id"], second["id"]}
    assert sculpts[first["id"]]["objects"] == 1 and sculpts[second["id"]]["objects"] == 2


def test_saving_an_empty_scene_as_a_sculpt_is_refused(lib):
    assert lib.post("/api/sculpts", json={"title": "Nothing", "fromScene": True}).status_code == 400
    assert listed(lib) == {}


def test_a_second_sculpt_of_the_same_name_gets_a_unique_name(lib):
    a, b = make(lib, "Twin"), make(lib, "Twin")
    assert a["id"] == "Twin" and b["id"] != "Twin" and b["id"].startswith("Twin")
    assert set(listed(lib)) == {a["id"], b["id"]}


def test_save_as_with_a_taken_name_is_refused_and_keeps_the_first_sculpt(lib):
    first = make(lib, "Taken", "torus")
    call(lib, "add_primitive", kind="cube")
    r = lib.post("/api/sculpts", json={"title": "Taken", "fromScene": True, "exact": True})
    assert r.status_code == 409
    assert listed(lib)["Taken"]["triangles"] == first["triangles"]


def test_names_with_spaces_dots_dashes_and_underscores_are_kept(lib):
    for title in ["My sculpt", "v1.2 final", "head-study_03"]:
        assert make(lib, title)["id"] == title
    assert set(listed(lib)) == {"My sculpt", "v1.2 final", "head-study_03"}


@pytest.mark.parametrize("title", BAD_TITLES)
def test_a_name_that_could_leave_the_library_is_refused_with_400(lib, tmp_path, title):
    for body in ({"title": title, "template": "cube"}, {"title": title, "fromScene": True}):
        call(lib, "add_primitive", kind="cube")
        r = lib.post("/api/sculpts", json=body)
        assert r.status_code == 400, (body, r.status_code, r.text)
    assert listed(lib) == {}
    assert outside(tmp_path) == []


def test_a_name_with_letters_outside_ascii_is_kept_or_refused_with_400_never_written_elsewhere(lib, tmp_path):
    for title in ["Été", "頭", "Kopf ü"]:
        r = lib.post("/api/sculpts", json={"title": title, "template": "cube"})
        assert r.status_code in (200, 400), (title, r.status_code)
        if r.status_code == 200:
            assert r.json()["id"] in listed(lib)
    assert outside(tmp_path) == []


@pytest.mark.parametrize("bad", BAD_IDS)
def test_a_sculpt_or_trash_id_that_could_leave_the_library_is_refused(lib, tmp_path, bad):
    keep = tmp_path / "data" / "LightBox" / "keep.fsc"
    keep.parent.mkdir(parents=True)
    keep.write_bytes(b"not the library's")
    (tmp_path / "keep.fsc").write_bytes(b"not the library's either")
    make(lib, "Inside")
    before = outside(tmp_path)
    for method, path in [
        ("POST", f"/api/sculpts/{bad}/open"), ("POST", f"/api/sculpts/{bad}/save"), ("POST", f"/api/sculpts/{bad}/duplicate"),
        ("DELETE", f"/api/sculpts/{bad}"), ("GET", f"/api/sculpts/{bad}/thumb"), ("PUT", f"/api/sculpts/{bad}/thumb"),
        ("POST", f"/api/sculpt-trash/{bad}/restore"), ("DELETE", f"/api/sculpt-trash/{bad}"), ("POST", f"/api/sculpt-trash/{bad}-1/restore"),
    ]:
        r = lib.request(method, path, content=b"\x89PNG\r\n\x1a\n" if method == "PUT" else None)
        assert 400 <= r.status_code < 500, (method, path, r.status_code, r.text)
    assert outside(tmp_path) == before
    assert keep.read_bytes() == b"not the library's"
    assert set(listed(lib)) == {"Inside"}


def test_opening_a_sculpt_that_is_not_there_is_404_and_keeps_the_scene(lib):
    call(lib, "add_primitive", kind="cube")
    assert lib.post("/api/sculpts/Missing/open").status_code == 404
    assert len(scene(lib)) == 1


def test_duplicate_makes_a_copy_that_is_edited_on_its_own(lib):
    made = make(lib, "Original", "sphere")
    copy = ok(lib.post(f"/api/sculpts/{quote(made['id'])}/duplicate"))
    assert copy["id"] == "Original copy" and copy["triangles"] == made["triangles"]
    opened(lib, copy["id"])
    call(lib, "add_primitive", kind="cube")
    ok(lib.post(f"/api/sculpts/{quote(copy['id'])}/save"))
    opened(lib, made["id"])
    assert len(scene(lib)) == 1
    opened(lib, copy["id"])
    assert len(scene(lib)) == 2
    assert listed(lib)["Original"]["objects"] == 1 and listed(lib)["Original copy"]["objects"] == 2


def test_trash_moves_a_sculpt_out_of_the_list_with_what_a_restore_needs(lib, tmp_path):
    made = make(lib, "Doomed", "torus")
    make(lib, "Stays")
    moved = ok(lib.delete("/api/sculpts/Doomed"))
    assert moved["ok"] is True and moved["entry"]
    assert set(listed(lib)) == {"Stays"}
    [entry] = trashed(lib)
    assert entry["entry"] == moved["entry"] and entry["id"] == "Doomed" and entry["title"] == "Doomed"
    assert entry["objects"] == made["objects"] and isinstance(entry["deletedAt"], int) and entry["deletedAt"] > 0
    assert not (project(tmp_path) / "Doomed.fsc").exists()


def test_the_trash_count_follows_each_move_to_the_trash(lib):
    for n, title in enumerate(["One", "Two", "Three"], start=1):
        make(lib, title)
        ok(lib.delete(f"/api/sculpts/{title}"))
        assert len(trashed(lib)) == n
    assert listed(lib) == {}


def test_restore_brings_the_sculpt_back_unchanged(lib):
    made = make(lib, "Back", "cone")
    opened(lib, "Back")
    before = scene(lib)
    entry = ok(lib.delete("/api/sculpts/Back"))["entry"]
    call(lib, "new_scene")
    restored = ok(lib.post(f"/api/sculpt-trash/{quote(entry)}/restore"))
    assert restored["id"] == "Back" and restored["triangles"] == made["triangles"]
    assert trashed(lib) == [] and set(listed(lib)) == {"Back"}
    opened(lib, "Back")
    assert scene(lib) == before


def test_restore_onto_a_name_taken_since_keeps_both(lib):
    make(lib, "Name", "torus")
    entry = ok(lib.delete("/api/sculpts/Name"))["entry"]
    newer = make(lib, "Name", "cube")
    restored = ok(lib.post(f"/api/sculpt-trash/{quote(entry)}/restore"))
    assert restored["id"] != "Name" and restored["id"].startswith("Name")
    sculpts = listed(lib)
    assert set(sculpts) == {"Name", restored["id"]}
    assert sculpts["Name"]["triangles"] == newer["triangles"]


def test_delete_permanently_removes_the_entry_from_the_trash_and_the_disk(lib, tmp_path):
    make(lib, "Gone")
    make(lib, "Kept")
    entry = ok(lib.delete("/api/sculpts/Gone"))["entry"]
    assert [p for p in project(tmp_path).rglob("*") if "Gone" in p.name]
    ok(lib.delete(f"/api/sculpt-trash/{quote(entry)}"))
    assert trashed(lib) == []
    assert not [p for p in project(tmp_path).rglob("*") if "Gone" in p.name]
    assert set(listed(lib)) == {"Kept"}
    assert lib.delete(f"/api/sculpt-trash/{quote(entry)}").status_code == 404


def test_empty_trash_empties_it_and_changes_nothing_else(lib, tmp_path):
    for title in ["A", "B", "C"]:
        make(lib, title)
    ok(lib.delete("/api/sculpts/A"))
    ok(lib.delete("/api/sculpts/B"))
    keep = {p: p.read_bytes() for p in project(tmp_path).glob("C*") if p.is_file()}
    emptied = ok(lib.delete("/api/sculpt-trash"))
    assert emptied["ok"] is True and emptied["removed"] == 2
    assert trashed(lib) == [] and set(listed(lib)) == {"C"}
    assert {p: p.read_bytes() for p in project(tmp_path).glob("C*") if p.is_file()} == keep
    assert ok(lib.delete("/api/sculpt-trash"))["removed"] == 0


def test_opening_a_sculpt_puts_it_first_in_the_recent_order(lib):
    for title in ["Old", "Middle", "New"]:
        make(lib, title)
    assert list(listed(lib))[0] == "New"
    opened(lib, "Old")
    sculpts = ok(lib.get("/api/sculpts"))["sculpts"]
    assert sculpts[0]["id"] == "Old" and sculpts[0]["opened"]
    assert [s["opened"] for s in sculpts[1:]] == [None, None]


def test_a_project_from_an_older_version_in_the_folder_lists_and_opens(lib, tmp_path):
    call(lib, "add_primitive", kind="torus", detail=2)
    data = lib.get("/api/engine/project").content
    before = scene(lib)
    call(lib, "new_scene")
    folder = project(tmp_path)
    folder.mkdir(parents=True, exist_ok=True)
    (folder / "From before.fsc").write_bytes(data)
    assert "From before" in listed(lib)
    opened(lib, "From before")
    assert scene(lib) == before


def test_a_thumbnail_over_2_mb_is_refused_with_413(lib):
    make(lib, "Pic")
    r = lib.put("/api/sculpts/Pic/thumb", content=b"\x89PNG\r\n\x1a\n" + b"\0" * 2_000_001)
    assert r.status_code == 413


def test_a_thumbnail_that_is_not_a_picture_is_refused_with_415(lib):
    make(lib, "Pic")
    assert lib.put("/api/sculpts/Pic/thumb", content=b"<svg></svg>").status_code == 415


def test_a_sent_thumbnail_is_the_one_served(lib):
    make(lib, "Pic")
    picture = lib.get("/api/sculpts/Pic/thumb").content
    ok(lib.put("/api/sculpts/Pic/thumb", content=picture))
    served = lib.get("/api/sculpts/Pic/thumb")
    assert served.status_code == 200 and served.content == picture


def test_a_sculpt_renamed_lists_under_its_new_name_only(lib):
    made = make(lib, "Before", "torus")
    r = lib.post("/api/sculpts/Before/rename", json={"title": "After"})
    skip_without_rename(r)
    assert r.status_code == 200, r.text
    sculpts = listed(lib)
    assert "Before" not in sculpts and sculpts["After"]["triangles"] == made["triangles"]


@pytest.mark.parametrize("title", ["../evil", "..", "a/b", "con"])
def test_a_rename_to_a_name_that_could_leave_the_library_is_refused_with_400(lib, tmp_path, title):
    make(lib, "Stay")
    r = lib.post("/api/sculpts/Stay/rename", json={"title": title})
    skip_without_rename(r)
    assert r.status_code == 400
    assert set(listed(lib)) == {"Stay"} and outside(tmp_path) == []
