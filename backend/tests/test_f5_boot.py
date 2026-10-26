from family import APP_ID, HEALTH_PATH, VERSION_KEY, VERSION_PATH, get_json


def test_engine_boots_on_a_free_port_with_an_empty_data_folder(engine_proc):
    status, _ = get_json(engine_proc["base"] + HEALTH_PATH)
    assert status == 200
    assert engine_proc["proc"].poll() is None


def test_engine_reports_the_repository_version(engine_proc, repo_version):
    status, body = get_json(engine_proc["base"] + VERSION_PATH)
    assert status == 200
    assert str(body[VERSION_KEY]).lstrip("v") == repo_version.lstrip("v")


def test_engine_reports_its_native_module_at_the_same_version(engine_proc, repo_version):
    status, body = get_json(engine_proc["base"] + "/api/capabilities")
    assert status == 200
    assert body["engine"]["native"] is True
    assert body["engine"]["version"].lstrip("v") == repo_version.lstrip("v")


def test_engine_keeps_out_of_the_home_folder(engine_proc):
    home = engine_proc["data"].parent / "home"
    assert not [p.name for p in home.iterdir() if APP_ID in p.name.lower()]
