import json
import os
import subprocess
import sys
import time
from pathlib import Path

from app.core import config, prefs
from app.services import agent_lookup, agents, terminal
from app.services.commands import CommandError, command

INSTALL = "Claude Code is not installed: install it from https://claude.com/claude-code, run claude once in a terminal to sign in, then open it here again"
INSTALL_LINE = agent_lookup.install("claude")
OVERRIDES = {"claude": ("FSC_CLAUDE_BIN",), "codex": ("FSC_CODEX_BIN",), "cursor": ("FSC_CURSOR_BIN",), "antigravity": ("FSC_ANTIGRAVITY_BIN",)}
SETTINGS = {"claude": "claudeBin"}
SHELL_WAIT = 10.0
CHECK_WAIT = 15.0
OTHERWISE = "give its path in the Agents settings"
TAB_SERVER = "formsculpt_tab"
NOTE = ("You are Claude Code running in the terminal dock inside formsculpt, the 3D sculpting app open on this computer. "
        "Sculpt and model with the formsculpt MCP server's tools: start with get_scene for object ids and bounds, look with get_view before and after a change, "
        "and read the sculpt_strategy prompt for the order of work. The tools act on the scene open in the window, the person sees every change happen, "
        "and each change is one undo step. Use the formsculpt-modeling skill when it is available.")
_cache: dict = {"at": 0.0, "value": None}


def _clean(env: dict) -> dict:
    env.pop(agents.INAPP_VAR, None)
    return env


def child_env(path: str | None = None) -> dict:
    front = [] if getattr(sys, "frozen", False) else [config.REPO]
    return _clean(agent_lookup.child_env(path, base=terminal.base_env(), front=front, shell_timeout=SHELL_WAIT))


def terminal_env(token: str, path: str | None = None, extra: dict | None = None) -> dict:
    return child_env(path) | {agents.INAPP_VAR: token} | (extra or {})


def located(kind: str, fresh: bool = False) -> agent_lookup.Found:
    setting = _pref(SETTINGS[kind]) if kind in SETTINGS else None
    found = agent_lookup.find(kind, setting=setting, override_vars=OVERRIDES.get(kind, ()), shell_timeout=SHELL_WAIT, fresh=fresh)
    return agent_lookup.check(found, child_env(found.path), CHECK_WAIT, fresh)


def usable(kind: str) -> agent_lookup.Found:
    found = located(kind)
    return found if found.state == "runs" else located(kind, fresh=True)


def said(found: agent_lookup.Found) -> str:
    return agent_lookup.sentence(found, agent_lookup.install(found.program), OTHERWISE if found.program in SETTINGS else "")


def _pref(key: str) -> str:
    value = prefs.load().get(key)
    return value.strip() if isinstance(value, str) else ""


def _runnable(path: str) -> bool:
    p = Path(path).expanduser()
    return p.is_file() and os.access(p, os.X_OK)


def cli() -> str | None:
    found = located("claude")
    return found.path if found.state == "runs" else None


def _signed_in(path: str) -> bool | None:
    if os.environ.get("ANTHROPIC_API_KEY") or os.environ.get("CLAUDE_CODE_OAUTH_TOKEN"):
        return True
    try:
        out = agent_lookup.run([path, "auth", "status"], child_env(path), CHECK_WAIT)
        data = json.loads(out.stdout)
    except (OSError, subprocess.SubprocessError, ValueError):
        return None
    return bool(data.get("loggedIn")) if isinstance(data, dict) and "loggedIn" in data else None


def info(fresh: bool = False) -> dict:
    if not fresh and _cache["value"] is not None and time.time() - _cache["at"] < 60:
        return _cache["value"]
    found = located("claude", fresh)
    signed = _signed_in(found.path) if found.state == "runs" and found.path else None
    _cache["value"] = agent_lookup.answer(found, INSTALL_LINE, OTHERWISE) | {"signedIn": signed, "install": "" if found.state == "runs" else INSTALL_LINE}
    _cache["at"] = time.time()
    return _cache["value"]


def mcp_config() -> str:
    return json.dumps({"mcpServers": {"formsculpt": {"type": "http", "url": config.engine_url() + "/mcp", "headers": {agents.INAPP_HEADER: agents.INAPP_TOKEN}}}})


def workdir() -> Path:
    raw = _pref("claudeFolder") or os.environ.get("FSC_AGENT_DIR", "").strip()
    folder = Path(raw).expanduser() if raw else config.data_dir() / "Agent"
    try:
        folder.mkdir(parents=True, exist_ok=True)
    except OSError as e:
        raise CommandError(f"Claude Code cannot start in {folder}: {e.strerror or e}; choose another folder in the Agents settings", 400) from None
    return folder


def tab_mcp_config() -> str:
    return json.dumps({"mcpServers": {TAB_SERVER: {"type": "http", "url": config.engine_url() + "/mcp", "headers": {agents.INAPP_HEADER: "${" + agents.INAPP_VAR + "}"}}}})


def argv(path: str, prompt: str | None, resume: str | None = None) -> list[str]:
    cmd = [path, "--mcp-config", tab_mcp_config(), "--allowedTools", f"mcp__{TAB_SERVER}", "--append-system-prompt", NOTE]
    if resume:
        cmd += ["--resume", resume]
    if prompt:
        cmd += ["--", prompt]
    return agent_lookup.start_argv(cmd)


@command("claudecode.status", "Claude Code",
         description="Whether the Claude Code CLI is installed (its path and version), the folder it starts in, and the terminal tab it runs in when open.")
def status() -> dict:
    tab = next((s.describe() for s in terminal.SESSIONS.values() if s.kind == "claude"), None)
    chosen = _pref("claudeBin")
    note = f"{chosen} is not a program formsculpt can run, so it uses the one found on this computer" if chosen and not _runnable(chosen) else ""
    return info(fresh=True) | {"folder": str(workdir()), "terminal": tab, "binNote": note, "firstMessage": _pref("claudeFirstMessage")}


@command("claudecode.open", "Open Claude Code", {"prompt": {"type": "string", "description": "a first message to start the session with"},
                                                  "cols": {"type": "integer"}, "rows": {"type": "integer"}, "fresh": {"type": "boolean"},
                                                  "resume": {"type": "string", "description": "a Claude Code session id to continue, such as the Chat tab's"}},
         description="Starts the person's own Claude Code sign-in in a terminal tab of formsculpt's dock, connected to formsculpt's MCP tools without the outside-agent prompt. Reuses the running tab unless fresh is true or resume names a conversation to continue. The window only.")
def open_(prompt: str | None = None, cols: int | None = None, rows: int | None = None, fresh: bool | None = None, resume: str | None = None) -> dict:
    running = next((s for s in terminal.SESSIONS.values() if s.kind == "claude" and s.alive), None)
    if running is not None and not fresh and not resume:
        if prompt:
            terminal.write(running.id, prompt + "\r")
        return running.describe()
    found = usable("claude")
    if found.state != "runs" or not found.path:
        raise CommandError(said(found), 424)
    _cache["value"] = None
    prompt = prompt or _pref("claudeFirstMessage") or None
    token = agents.issue()
    try:
        session = terminal.open_session(str(workdir()), cols or 100, rows or 30, "Claude Code (from Chat)" if resume else "Claude Code",
                                        argv(found.path, prompt, resume), terminal_env(token, found.path), "claude")
    except BaseException:
        agents.revoke(token)
        raise
    session.on_close.append(lambda: agents.revoke(token))
    return session.describe()


@command("agents.prefs", "Agent Settings", {"claudeFolder": {"type": "string"}, "claudeBin": {"type": "string"}, "claudeFirstMessage": {"type": "string"}},
         description="Reads, or changes when values are given, the folder Claude Code starts in, the Claude Code program to run and a first message for new sessions. The window only.")
def agent_prefs(claudeFolder: str | None = None, claudeBin: str | None = None, claudeFirstMessage: str | None = None) -> dict:
    changes = {k: v for k, v in {"claudeFolder": claudeFolder, "claudeBin": claudeBin, "claudeFirstMessage": claudeFirstMessage}.items() if v is not None}
    current = prefs.update(changes) if changes else prefs.load()
    _cache["value"] = None
    agent_lookup.forget()
    return {k: current[k] for k in prefs.DEFAULTS}
