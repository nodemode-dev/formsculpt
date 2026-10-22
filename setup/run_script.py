#!/usr/bin/env python3
"""./formsculpt script FILE [JSON]: runs a Python script against formsculpt.

The script sees `fsc` (fsc.call(name, **args) runs any registry command by its MCP name) and `args` (the JSON given
after the file, or {}). With the engine running (the window or ./formsculpt backend), the script drives that engine
and the window updates as it goes; otherwise it runs an engine in this process, and `fsc.call("save_project", ...)`
or an export keeps the result.
"""

import json
import os
import sys
import urllib.error
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "backend"))

from app.services.scripting import Fsc, ScriptError, run_file  # noqa: E402

URL = os.environ.get("FSC_ENGINE_URL", "http://127.0.0.1:8196")


def http_dispatch(name: str, args: dict) -> dict:
    req = urllib.request.Request(f"{URL}/api/engine/dispatch", data=json.dumps({"name": name, "args": args}).encode(), headers={"content-type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=600) as r:
            return json.load(r)
    except urllib.error.HTTPError as err:
        return json.load(err)


def engine_running() -> bool:
    try:
        with urllib.request.urlopen(f"{URL}/api/version", timeout=2) as r:
            return r.status == 200
    except (urllib.error.URLError, OSError):
        return False


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
        print(__doc__.strip())
        return 0 if len(sys.argv) >= 2 else 2
    path = os.path.join(os.environ.get("FSC_CWD", os.getcwd()), sys.argv[1])
    if not os.path.isfile(path):
        print(f"[formsculpt script] no such file: {path}", file=sys.stderr)
        return 2
    try:
        args = json.loads(sys.argv[2]) if len(sys.argv) > 2 else {}
    except json.JSONDecodeError as err:
        print(f"[formsculpt script] the arguments are not JSON: {err}", file=sys.stderr)
        return 2
    if engine_running():
        fsc = Fsc(http_dispatch)
        where = f"the running engine at {URL}"
    else:
        try:
            import fsc_engine
        except ImportError:
            print("[formsculpt script] no engine is running and the engine module is not built (./formsculpt engine build)", file=sys.stderr)
            return 1
        engine = fsc_engine.Engine()
        fsc = Fsc(lambda name, a: json.loads(engine.dispatch(name, json.dumps(a))))
        where = "an engine in this process"
    print(f"[formsculpt script] {os.path.basename(path)} on {where}", file=sys.stderr)
    try:
        ran = run_file(path, fsc, args)
    except ScriptError as err:
        print(f"[formsculpt script] {err}", file=sys.stderr)
        return 1
    except Exception as err:  # the script's own error, reported briefly with where it happened
        tb = err.__traceback__
        while tb and tb.tb_next:
            tb = tb.tb_next
        where = f" (line {tb.tb_lineno} of {os.path.basename(tb.tb_frame.f_code.co_filename)})" if tb else ""
        print(f"[formsculpt script] {type(err).__name__}: {err}{where}", file=sys.stderr)
        return 1
    sys.stdout.write(ran["output"])
    if ran["result"] is not None:
        print(json.dumps(ran["result"], indent=2, default=str))
    return 0


if __name__ == "__main__":
    sys.exit(main())
