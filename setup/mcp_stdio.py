#!/usr/bin/env python3
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

URL = os.environ.get("FSC_MCP_URL", "http://127.0.0.1:8260/mcp")
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
_started = False


def log(text: str) -> None:
    print(f"[formsculpt mcp] {text}", file=sys.stderr, flush=True)


def start_engine() -> bool:
    global _started
    if _started or os.environ.get("FSC_MCP_AUTOSTART", "1") == "0" or os.environ.get("FSC_MCP_URL"):
        return False
    _started = True
    command = "serve" if os.path.isfile(os.path.join(ROOT, "frontend", "dist", "index.html")) else "backend"
    path = os.path.join(tempfile.gettempdir(), "formsculpt-engine.log")
    with open(path, "ab") as out:
        log(f"starting the engine (./formsculpt {command}); its log: {path}")
        launcher = os.path.join(ROOT, "formsculpt.bat" if os.name == "nt" else "formsculpt")
        subprocess.Popen([launcher, command], cwd=ROOT, stdin=subprocess.DEVNULL, stdout=out, stderr=out,
                         start_new_session=(os.name != "nt"), creationflags=(0x208 if os.name == "nt" else 0))
    probe = URL.rsplit("/mcp", 1)[0] + "/api/version"
    for _ in range(120):
        time.sleep(0.5)
        try:
            with urllib.request.urlopen(probe, timeout=2) as r:
                if r.status == 200:
                    return True
        except (urllib.error.URLError, OSError):
            continue
    return False


def forward(line: str) -> str | None:
    try:
        msg = json.loads(line)
    except ValueError:
        return json.dumps({"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "Parse error"}})
    params = (msg.get("params") or {}) if isinstance(msg, dict) else {}
    version = (params.get("_meta") or {}).get("io.modelcontextprotocol/protocolVersion") or params.get("protocolVersion") or "2025-11-25"
    req = urllib.request.Request(URL, data=line.encode(), method="POST", headers={"Content-Type": "application/json", "Accept": "application/json", "MCP-Protocol-Version": version})
    try:
        with urllib.request.urlopen(req, timeout=660) as r:
            body = r.read().decode()
    except urllib.error.HTTPError as exc:
        body = exc.read().decode() or ""
    except (urllib.error.URLError, OSError) as exc:
        log(f"the engine is not answering at {URL}: {exc}")
        if start_engine():
            return forward(line)
        if not isinstance(msg, dict) or msg.get("id") is None:
            return None
        return json.dumps({"jsonrpc": "2.0", "id": msg.get("id"), "error": {"code": -32000, "message": "The formsculpt engine is not running and could not be started: run ./formsculpt backend in the formsculpt folder, then retry."}})
    if not isinstance(msg, dict) or msg.get("id") is None:
        return None
    return body.strip() or None


def main() -> None:
    for line in sys.stdin:
        if not line.strip():
            continue
        out = forward(line)
        if out:
            sys.stdout.write(out + "\n")
            sys.stdout.flush()


if __name__ == "__main__":
    main()
