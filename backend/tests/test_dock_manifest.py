import hashlib
import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DOCK = REPO / "frontend" / "src" / "dock"


def test_the_shared_dock_files_match_their_manifest():
    kept = json.loads((DOCK / "MANIFEST.json").read_text(encoding="utf-8"))
    assert kept["module"] == "dock" and kept["version"] == "2.2.1"
    files = sorted(p.name for p in DOCK.iterdir() if p.is_file() and p.name != "MANIFEST.json")
    assert sorted(kept["files"]) == files
    for name, digest in kept["files"].items():
        assert hashlib.sha256((DOCK / name).read_bytes()).hexdigest() == digest, name


def test_the_engine_reports_the_panel_version_it_was_built_for():
    from app.services import agent_programs

    assert agent_programs.CONTRACT == "2.2.1"


def test_the_guide_says_the_other_three_programs_get_a_button():
    text = (DOCK / "strings.ts").read_text(encoding="utf-8")
    assert text.count("Codex, Cursor and Antigravity each get a button of their own here when they are on this computer.") == 2
