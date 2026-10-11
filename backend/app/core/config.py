import os
from pathlib import Path

from app import __version__

APP_NAME = "formsculptor"
VERSION = __version__
FRONTEND_PORT = 5196
BACKEND_PORT = 8196
REPO = Path(__file__).resolve().parents[3]


def dist_dir() -> Path | None:
    configured = os.environ.get("FSC_DIST")
    if configured:
        path = Path(configured)
        return path if (path / "index.html").is_file() else None
    path = REPO / "frontend" / "dist"
    return path if (path / "index.html").is_file() else None


def is_desktop() -> bool:
    return os.environ.get("FSC_DESKTOP") == "1"


def cors_origins() -> list[str]:
    configured = os.environ.get("FSC_CORS_ORIGINS")
    if configured:
        return [o.strip() for o in configured.split(",") if o.strip()]
    return [f"http://localhost:{FRONTEND_PORT}", f"http://127.0.0.1:{FRONTEND_PORT}"]
