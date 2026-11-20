import re

from app.services import agent_programs, terminal
from app.services.commands import CommandError, command

ID = {"type": "string", "description": "terminal id from terminal.open or terminal.list"}
ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07]*(\x07|\x1b\\)|\x1b[@-_]")


def _session(sid: str) -> terminal.Session:
    try:
        return terminal.get(sid)
    except KeyError:
        raise CommandError(f"No terminal {sid}; terminal.list gives the ids", 404) from None


@command("terminal.open", "New Terminal", {"cwd": {"type": "string"}, "cols": {"type": "integer"}, "rows": {"type": "integer"}, "title": {"type": "string"}},
         description="Starts the person's login shell in a real terminal in formsculpt's terminal dock (default folder: the home folder) and returns its id. The window only; outside agents cannot reach it.")
def open_(cwd: str | None = None, cols: int | None = None, rows: int | None = None, title: str | None = None) -> dict:
    try:
        return terminal.open_session(cwd, cols or 100, rows or 30, title).describe()
    except FileNotFoundError as exc:
        raise CommandError(str(exc), 404) from None


@command("terminal.list", "Terminals", description="Open terminals with their ids, titles, folders, kind (shell or claude) and whether the program is still running.")
def list_() -> dict:
    items = [s.describe() for s in terminal.SESSIONS.values()]
    return {"terminals": items, "count": len(items)}


@command("terminal.write", "Type into a Terminal", {"id": ID, "data": {"type": "string", "description": "text to type; end with \\r to press Enter"}},
         description="Types text into a terminal as if at the keyboard. The window shows it live.")
def write(id: str, data: str) -> dict:
    session = _session(id)
    if not session.alive:
        raise CommandError(f"Terminal {id} has exited; restart it or open a new one", 409)
    terminal.write(id, data)
    return {"written": len(data)}


@command("terminal.read", "Read a Terminal", {"id": ID, "chars": {"type": "integer", "description": "how much of the end to return, default 4000"}},
         description="The end of a terminal's output as plain text, colour codes removed.")
def read(id: str, chars: int | None = None) -> dict:
    session = _session(id)
    text = ANSI.sub("", session.scrollback()).replace("\r\n", "\n").replace("\r", "")
    n = max(1, min(int(chars or 4000), 200000))
    return {"id": id, "alive": session.alive, "text": text[-n:]}


@command("terminal.resize", "Resize a Terminal", {"id": ID, "cols": {"type": "integer"}, "rows": {"type": "integer"}})
def resize(id: str, cols: int, rows: int) -> dict:
    _session(id)
    terminal.resize(id, cols, rows)
    return {"id": id, "cols": cols, "rows": rows}


@command("terminal.restart", "Restart a Terminal", {"id": ID},
         description="Ends the terminal's program and starts it again in the same tab and folder; its output so far is cleared.")
def restart(id: str) -> dict:
    _session(id)
    return terminal.restart(id).describe()


@command("terminal.close", "Close a Terminal", {"id": ID}, description="Ends the program and closes the terminal; an id that is already gone is fine.")
def close(id: str) -> dict:
    terminal.close(id)
    return {"closed": id}


@command("terminal.agents", "Agent Programs", {"fresh": {"type": "boolean"}},
         description="Whether Claude Code, Codex, Cursor and Antigravity are installed here (version, install line, sign-in when it can tell), and the dock contract version this engine implements. The window only.")
def agents_status(fresh: bool | None = None) -> dict:
    return agent_programs.report(bool(fresh))


@command("terminal.agent", "Open an Agent Program", {"kind": {"type": "string", "description": "codex, cursor or antigravity"},
                                                      "cols": {"type": "integer"}, "rows": {"type": "integer"}},
         description="Starts Codex, Cursor or Antigravity in a terminal tab of formsculpt's dock, connected to formsculpt's MCP tools with a token that ends when the tab closes. The window only.")
def open_agent(kind: str, cols: int | None = None, rows: int | None = None) -> dict:
    return agent_programs.open_terminal(kind, max(10, min(int(cols or 100), 500)), max(4, min(int(rows or 30), 200)))
