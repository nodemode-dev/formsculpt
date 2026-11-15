"""Python scripting and plugins.

A script or plugin drives formsculpt through `fsc`, an object with one method per idea: `fsc.call(name, **args)`
runs any registry command (the same names as the MCP tools) and returns its result, raising ScriptError when the
command refuses. Plugins are Python files in `plugins/` beside the app or in `~/.formsculpt/plugins` (or the folder
named by FSC_PLUGIN_DIR); the first line of a plugin's docstring is its title, and it defines `run(fsc, args)`.
Scripts run from the command line (`./formsculpt script file.py`) get the same `fsc`, bound to the running engine
over HTTP or, when none is running, to an engine in the script's own process.
"""

from __future__ import annotations

import ast
import contextlib
import io
import os
import runpy
import sys
from pathlib import Path
from typing import Any, Callable

ROOT = Path(getattr(sys, "_MEIPASS", None) or Path(__file__).resolve().parents[3])


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
    dirs.append(Path(extra) if extra else Path.home() / ".formsculpt" / "plugins")
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
    # PARAMS and CLICK are read as literals, never by running the file: listing plugins must not execute them.
    params, click = {}, False
    for node in tree.body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            try:
                value = ast.literal_eval(node.value)
            except (ValueError, SyntaxError):
                continue
            if node.targets[0].id == "PARAMS" and isinstance(value, dict):
                params = {k: _param(v) for k, v in value.items() if isinstance(k, str) and isinstance(v, dict)}
            elif node.targets[0].id == "CLICK":
                click = value is True
    return {"name": path.stem, "title": title or path.stem.replace("_", " ").capitalize(), "description": rest.strip(), "path": str(path), "params": params, "click": click}


def _param(spec: dict) -> dict:
    """A plugin field, kept to what the window can draw: number, integer, boolean, choice or text."""
    kind = spec.get("type") if spec.get("type") in ("number", "integer", "boolean", "choice", "text") else "number"
    out = {"type": kind, "label": str(spec.get("label", ""))[:40]}
    for key in ("default", "min", "max", "step"):
        if isinstance(spec.get(key), (int, float, bool, str)):
            out[key] = spec[key]
    if kind == "choice":
        out["options"] = [str(o) for o in spec.get("options", []) if isinstance(o, (str, int, float))][:32]
    return out


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
        module = runpy.run_path(str(path), init_globals={"fsc": fsc, "args": args or {}}, run_name="__formsculpt__")
        result = module["run"](fsc, args or {}) if callable(module.get("run")) else None
    return {"result": result, "output": out.getvalue()}
