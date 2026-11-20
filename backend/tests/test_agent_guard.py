import os
import shutil

import pytest

import conftest

pytestmark = pytest.mark.skipif(os.name == "nt", reason="the stand-ins are POSIX shell scripts")


def test_only_stand_in_agent_programs_are_found():
    assert conftest.real_agent_programs() == []
    for name in conftest.AGENT_NAMES:
        assert shutil.which(name) == str(conftest.STAND_INS / name)
    assert os.environ["SHELL"] == str(conftest.family.sandbox() / "shell" / "sh")


def test_a_real_program_the_finder_would_reach_is_reported(monkeypatch):
    from app.services import agent_lookup

    def find(kind, **_):
        found = agent_lookup.Found(kind, agent_lookup.PROGRAMS[kind]["title"])
        if kind == "antigravity":
            found.path = "/opt/antigravity/bin/agy"
        return found

    monkeypatch.setattr(agent_lookup, "find", find)
    assert conftest.real_agent_programs() == ["Antigravity at /opt/antigravity/bin/agy"]
