import json
import os
import sys


def resource_dir() -> str:
    return getattr(sys, "_MEIPASS", os.path.dirname(os.path.abspath(__file__)))


def selfcheck() -> int:
    info = {"python": sys.version.split()[0]}
    for name in ("fastapi", "uvicorn"):
        try:
            module = __import__(name)
            info[name] = getattr(module, "__version__", "ok")
        except Exception as exc:
            info[name] = f"missing: {exc}"
    from app import __version__

    info["formsculpt"] = __version__
    try:
        import fsc_engine

        session = fsc_engine.Engine()
        reply = json.loads(session.dispatch("add_primitive", json.dumps({"kind": "sphere", "detail": 3})))
        info["engine"] = fsc_engine.version()
        info["engine_commands"] = len(json.loads(fsc_engine.catalogue())["commands"])
        info["engine_ok"] = bool(reply.get("ok"))
    except Exception as exc:
        info["engine"] = f"missing: {exc}"
        info["engine_ok"] = False
    try:
        from app.services import claude_code, cmd_chat, cmd_terminal, terminal

        info["terminal"] = bool(cmd_terminal and cmd_chat and claude_code and terminal.login_shell())
    except Exception as exc:
        info["terminal"] = f"missing: {exc}"
    info["dist_bundled"] = os.path.isdir(os.path.join(resource_dir(), "frontend_dist"))
    print(json.dumps(info))
    return 0 if info["engine_ok"] and info["dist_bundled"] and info["terminal"] is True else 1


def parent_alive(pid: int) -> bool:
    """Whether a process exists, without signalling it (os.kill on Windows would terminate it)."""
    if os.name == "nt":
        import ctypes

        kernel32 = ctypes.windll.kernel32
        handle = kernel32.OpenProcess(0x00100000, False, pid)  # SYNCHRONIZE
        if not handle:
            return False
        try:
            return kernel32.WaitForSingleObject(handle, 0) == 0x102  # WAIT_TIMEOUT: still running
        finally:
            kernel32.CloseHandle(handle)
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def watch_parent() -> None:
    """Exit when the desktop shell that started this engine is gone. The shell's own cleanup runs only on a clean exit,
    and a SIGKILL to PyInstaller's bootloader does not reach this child, so a crashed, killed or force-quit shell
    would otherwise leave the engine running."""
    raw = os.environ.get("FSC_PARENT_PID", "")
    if not raw.isdigit():
        return
    pid = int(raw)
    import threading
    import time

    def loop() -> None:
        while parent_alive(pid):
            time.sleep(1.5)
        unpacked = getattr(sys, "_MEIPASS", "")
        if os.path.basename(unpacked).startswith("_MEI"):
            import shutil

            shutil.rmtree(unpacked, ignore_errors=True)
        os._exit(0)

    threading.Thread(target=loop, name="parent-watch", daemon=True).start()


def free_port() -> int:
    import socket

    from app.core.config import BACKEND_PORT

    for want in (BACKEND_PORT, 0):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            if os.name != "nt":
                s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            try:
                s.bind(("127.0.0.1", want))
            except OSError:
                continue
            return s.getsockname()[1]
    raise OSError("no free port on 127.0.0.1")


def main() -> int:
    if "--selfcheck" in sys.argv:
        return selfcheck()
    watch_parent()
    bundled = os.path.join(resource_dir(), "frontend_dist")
    if os.path.isdir(bundled):
        os.environ.setdefault("FSC_DIST", bundled)
    integrations = os.path.join(resource_dir(), "integrations")
    if os.path.isdir(integrations):
        os.environ.setdefault("FSC_INTEGRATIONS", integrations)
    os.environ.setdefault("FSC_DESKTOP", "1")
    import uvicorn

    from app.main import app as fastapi_app

    port = int(os.environ.get("FSC_PORT", "0") or 0) or free_port()
    os.environ["FSC_PORT"] = str(port)
    uvicorn.run(fastapi_app, host="127.0.0.1", port=port, log_level="info")
    return 0


if __name__ == "__main__":
    import multiprocessing

    multiprocessing.freeze_support()
    sys.exit(main())
