from __future__ import annotations

import json
import os
import time
from pathlib import Path

from app.core import config
from app.services import agent_lookup, agents, claude_code, terminal
from app.services.commands import CommandError

PROGRAMS = {
    "codex": {"title": "Codex"},
    "cursor": {"title": "Cursor"},
    "antigravity": {"title": "Antigravity"},
}
TABS = tuple(PROGRAMS)
MCP_URL_VAR = "FSC_MCP_URL"
SERVER = claude_code.TAB_SERVER
CONTRACT = "2.2.0"
_cache: dict = {"at": 0.0, "value": None}


def mcp_url() -> str:
    return config.engine_url() + "/mcp"


def tab_folder(kind: str) -> Path:
    folder = config.data_dir() / "agent-tabs" / kind
    folder.mkdir(parents=True, exist_ok=True)
    return folder


def _mcp_file(folder: Path) -> Path:
    return folder / ".agents" / "mcp_config.json"


def _servers(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8") or "{}") if path.exists() else {}
    if not isinstance(data, dict) or not isinstance(data.get("mcpServers", {}), dict):
        raise OSError(f"{path} is not an MCP servers file formsculpt can read, so it was left alone")
    return data


def _replace(path: Path, data: dict) -> None:
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    part = path.with_name(path.name + f".part-{os.getpid()}")
    with os.fdopen(os.open(part, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w", encoding="utf-8") as fh:
        fh.write(json.dumps(data, indent=2) + "\n")
    os.replace(part, path)


def write_tab_entry(folder: Path, token: str) -> None:
    path = _mcp_file(folder)
    data = _servers(path)
    data.setdefault("mcpServers", {})[SERVER] = {"serverUrl": mcp_url(), "headers": {agents.INAPP_HEADER: token}}
    _replace(path, data)


def remove_tab_entry(folder: Path, token: str) -> None:
    path = _mcp_file(folder)
    try:
        data = _servers(path)
        entry = data.get("mcpServers", {}).get(SERVER)
        if not isinstance(entry, dict) or (entry.get("headers") or {}).get(agents.INAPP_HEADER) != token:
            return
        del data["mcpServers"][SERVER]
        _replace(path, data)
    except (OSError, ValueError):
        return


def _one(kind: str, fresh: bool) -> dict:
    found = claude_code.located(kind, fresh)
    install = agent_lookup.install(kind)
    return agent_lookup.answer(found, install) | {"signedIn": None, "install": "" if found.state == "runs" else install, "label": PROGRAMS[kind]["title"]}


def report(fresh: bool = False) -> dict:
    if fresh or _cache["value"] is None or time.time() - _cache["at"] >= 60:
        _cache.update(at=time.time(), value={kind: _one(kind, fresh) for kind in TABS})
    return {"claude": claude_code.info(fresh=fresh) | {"label": "Claude Code"}, **_cache["value"], "contract": CONTRACT}


def codex_args() -> list[str]:
    return ["-c", f'mcp_servers.{SERVER}.url="{mcp_url()}"',
            "-c", f'mcp_servers.{SERVER}.env_http_headers={{"{agents.INAPP_HEADER}"="{agents.INAPP_VAR}"}}']


def cursor_config(folder: str) -> None:
    path = Path(folder) / ".cursor" / "mcp.json"
    entry = {"url": "${env:" + MCP_URL_VAR + "}", "headers": {agents.INAPP_HEADER: "${env:" + agents.INAPP_VAR + "}"}}
    try:
        current = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
    except (OSError, ValueError):
        return
    servers = current.get("mcpServers", {}) if isinstance(current, dict) else None
    if not isinstance(servers, dict) or servers.get(SERVER) == entry:
        return
    try:
        _replace(path, {**current, "mcpServers": {**servers, SERVER: entry}})
    except OSError:
        return


def open_terminal(kind: str, cols: int, rows: int) -> dict:
    if kind not in TABS:
        raise CommandError(f"No agent program called {kind}; the kinds are {', '.join(TABS)}", 400)
    found = claude_code.usable(kind)
    if found.state != "runs" or not found.path:
        raise CommandError(claude_code.said(found), 424)
    _cache["value"] = None
    argv = agent_lookup.start_argv([found.path] + (codex_args() if kind == "codex" else []))
    own = tab_folder(kind) if kind == "antigravity" else None
    folder = str(own) if own else str(claude_code.workdir())
    if kind == "cursor":
        cursor_config(folder)
    token = agents.issue()
    env = claude_code.terminal_env(token, found.path, {MCP_URL_VAR: mcp_url()})
    try:
        if own:
            write_tab_entry(own, token)
        session = terminal.open_session(folder, cols, rows, PROGRAMS[kind]["title"], argv, env, kind)
    except OSError as exc:
        agents.revoke(token)
        if own:
            remove_tab_entry(own, token)
        raise CommandError(f"{PROGRAMS[kind]['title']} could not start: {exc}", 500) from None
    session.on_close.append(lambda: agents.revoke(token))
    if own:
        session.on_close.append(lambda: remove_tab_entry(own, token))
    return session.describe()
