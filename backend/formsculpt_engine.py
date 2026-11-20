import json
import os
import sys

if sys.stdin is None:
    sys.stdin = open(os.devnull, encoding="utf-8")
if sys.stdout is None:
    sys.stdout = open(os.devnull, "w", encoding="utf-8")
if sys.stderr is None:
    sys.stderr = open(os.devnull, "w", encoding="utf-8")

LOG_NAME = "formsculpt-engine.log"
LOG_BYTES = 1_048_576


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


def start_log(data):
    import logging
    from logging.handlers import RotatingFileHandler

    folder = data / "Logs"
    folder.mkdir(parents=True, exist_ok=True)
    handler = RotatingFileHandler(folder / LOG_NAME, maxBytes=LOG_BYTES, backupCount=2, encoding="utf-8")
    handler.setFormatter(logging.Formatter("%(asctime)s %(levelname)s %(name)s %(message)s"))
    root = logging.getLogger()
    root.addHandler(handler)
    root.setLevel(logging.INFO)
    logging.getLogger("formsculpt").info("engine started, pid %s, data folder %s", os.getpid(), data)
    return folder


def log_config() -> dict:
    import copy

    from uvicorn.config import LOGGING_CONFIG

    config = copy.deepcopy(LOGGING_CONFIG)
    config["loggers"]["uvicorn"]["propagate"] = True
    for formatter in config["formatters"].values():
        formatter["use_colors"] = False
    return config


def release(lock) -> None:
    from app.core import startup

    if startup.number(startup.read_lock(lock).get("pid")) == os.getpid():
        try:
            lock.unlink()
        except OSError:
            return


def watch_shell(shell: int, lock) -> None:
    """Exit when the desktop shell that started this engine is gone. The shell's own cleanup runs only on a clean exit,
    and a SIGKILL to PyInstaller's bootloader does not reach this child, so a crashed, killed or force-quit shell
    would otherwise leave the engine running."""
    import logging

    from app.core import startup

    def leave() -> None:
        logging.getLogger("formsculpt").info("the app that started this engine is gone (process %s), stopping", shell)
        release(lock)
        unpacked = getattr(sys, "_MEIPASS", "")
        if os.path.basename(unpacked).startswith("_MEI"):
            import shutil

            shutil.rmtree(unpacked, ignore_errors=True)
        os._exit(0)

    startup.watch(shell, leave)


def sweep_stale_unpacks() -> None:
    """Removes the temporary folders earlier launches left behind. The one-file launcher names its folder `_MEI` + its
    process id as eight hex digits + a suffix (six random characters, or a counter on Windows), and deletes it when it
    exits; a launcher that was forced closed never did, and on Windows the engine cannot delete the files it still has
    loaded. A folder whose process is gone belongs to nobody; one whose id is in use is kept, and so is this engine's
    own. Runs in the background so the engine starts at once."""
    unpacked = getattr(sys, "_MEIPASS", "")
    if not os.path.basename(unpacked).startswith("_MEI"):
        return
    import re
    import shutil
    import threading

    name_form = r"_MEI(0[0-9a-f]{7})[0-9A-Za-z]{1,12}" if os.name == "nt" else r"_MEI([0-9a-f]{8})[A-Za-z0-9]{6}"

    def alive(pid: int) -> bool:
        if os.name == "nt":
            return parent_alive(pid)
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return False
        except PermissionError:
            return False  # another user's process now holds the id, so this user's launcher is gone
        return True

    def run() -> None:
        root, own = os.path.dirname(unpacked), os.path.basename(unpacked)
        try:
            names = os.listdir(root)
        except OSError:
            return
        for name in names:
            m = re.fullmatch(name_form, name)
            if not m or name == own or alive(int(m.group(1), 16)):
                continue
            shutil.rmtree(os.path.join(root, name), ignore_errors=True)

    threading.Thread(target=run, name="unpack-sweep", daemon=True).start()


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


def serve() -> int:
    import logging
    from pathlib import Path

    from app.core import config, startup

    try:
        data = config.data_dir()
        logs = start_log(data)
    except OSError as err:
        print(f"formsculpt cannot use its data folder: {err}", file=sys.stderr)
        return 5
    startup.keep_errors(logs)
    log = logging.getLogger("formsculpt")
    if startup.alone():
        beside = startup.app_beside(Path(sys.executable))
        if beside is not None and startup.open_app(beside):
            log.info("started without its app, so it opened %s and stopped", beside)
            return 0
        log.error("started without its app, and no app program was found near %s", sys.executable)
        print("formsculpt-engine is a part of FormSculpt and serves nothing on its own. Open FormSculpt instead.", file=sys.stderr)
        return 4
    shell = startup.shell_pid()
    try:
        lock, replaced = startup.claim(data, shell)
    except startup.Busy as err:
        log.error("%s", err)
        print(err, file=sys.stderr)
        return 2
    if replaced:
        log.info("ended an engine that was left running (process %s)", replaced)
    import signal

    def leave(_signum, _frame) -> None:
        release(lock)
        os._exit(0)

    signal.signal(signal.SIGTERM, leave)
    try:
        if shell > 1:
            watch_shell(shell, lock)
        return run_server()
    finally:
        release(lock)


def main() -> int:
    if "--selfcheck" in sys.argv:
        return selfcheck()
    try:
        return serve()
    except Exception:
        import logging

        logging.getLogger("formsculpt").exception("the engine stopped on an error")
        raise


def run_server() -> int:
    sweep_stale_unpacks()
    bundled = os.path.join(resource_dir(), "frontend_dist")
    if os.path.isdir(bundled):
        os.environ.setdefault("FSC_DIST", bundled)
    integrations = os.path.join(resource_dir(), "integrations")
    if os.path.isdir(integrations):
        os.environ.setdefault("FSC_INTEGRATIONS", integrations)
    os.environ.setdefault("FSC_DESKTOP", "1")
    import uvicorn

    from app.core import window_key
    from app.main import app as fastapi_app

    port = int(os.environ.get("FSC_PORT", "0") or 0) or free_port()
    os.environ["FSC_PORT"] = str(port)
    window_key.announce()
    uvicorn.run(fastapi_app, host="127.0.0.1", port=port, log_level="info", log_config=log_config())
    return 0


if __name__ == "__main__":
    import multiprocessing

    multiprocessing.freeze_support()
    sys.exit(main())
