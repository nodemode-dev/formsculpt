"""Python scripting and plugins.

A script or plugin drives formsculptor through `fsc`, an object with one method per idea: `fsc.call(name, **args)`
runs any registry command (the same names as the MCP tools) and returns its result, raising ScriptError when the
command refuses. Plugins are Python files in `plugins/` beside the app or in `~/.formsculptor/plugins` (or the folder
named by FSC_PLUGIN_DIR); the first line of a plugin's docstring is its title, and it defines `run(fsc, args)`.
Scripts run from the command line (`./formsculptor script file.py`) get the same `fsc`, bound to the running engine
over HTTP or, when none is running, to an engine in the script's own process.
"""

from __future__ import annotations

import ast
import contextlib
import io
import os
import runpy
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parents[3]


class ScriptError(RuntimeError):
    """A command refused; the message is the engine's."""


class Fsc:
    """The scripting handle: `call` runs a registry command by its MCP name and returns the result."""

    def __init__(self, dispatch: Callable[[str, dict], dict]):
        self._dispatch = dispatch

    def call(self, command: str, /, **args: Any) -> Any:
        # The command is positional-only, so commands that take a `name` argument (add_primitive, layers) work.
        reply = self._dispatch(command, args)
        if not reply.get("ok"):
            error = reply.get("error") or {}
            raise ScriptError(f"{command}: {error.get('message', 'failed')}")
        return reply.get("result")

    def scene(self) -> dict:
        return self.call("get_scene")

    def active(self) -> dict | None:
        scene = self.scene()
        return next((o for o in scene["objects"] if o["id"] == scene["active"]), None)


def plugin_dirs() -> list[Path]:
    dirs = [ROOT / "plugins"]
    extra = os.environ.get("FSC_PLUGIN_DIR")
    dirs.append(Path(extra) if extra else Path.home() / ".formsculptor" / "plugins")
    return [d for d in dirs if d.is_dir()]


def _describe(path: Path) -> dict | None:
    try:
        tree = ast.parse(path.read_text(encoding="utf-8"))
    except (OSError, SyntaxError, UnicodeDecodeError):
        return None
    if not any(isinstance(n, ast.FunctionDef) and n.name == "run" for n in tree.body):
        return None
    doc = (ast.get_docstring(tree) or "").strip()
    title, _, rest = doc.partition("\n")
    return {"name": path.stem, "title": title or path.stem.replace("_", " ").capitalize(), "description": rest.strip(), "path": str(path)}


def list_plugins() -> list[dict]:
    """Every plugin file that defines run(fsc, args), by name; a later folder's plugin replaces an earlier one's."""
    found: dict[str, dict] = {}
    for d in plugin_dirs():
        for path in sorted(d.glob("*.py")):
            info = _describe(path)
            if info:
                found[info["name"]] = info
    return sorted(found.values(), key=lambda p: p["title"].lower())


def run_file(path: str | Path, fsc: Fsc, args: dict | None = None) -> dict:
    """Runs a script or plugin file with `fsc` and `args` in scope, calling its run(fsc, args) if it has one; returns
    the result and anything it printed."""
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        module = runpy.run_path(str(path), init_globals={"fsc": fsc, "args": args or {}}, run_name="__formsculptor__")
        result = module["run"](fsc, args or {}) if callable(module.get("run")) else None
    return {"result": result, "output": out.getvalue()}
