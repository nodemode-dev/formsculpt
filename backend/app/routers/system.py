import platform
import sys

from fastapi import APIRouter

from app.core import config

router = APIRouter()


@router.get("/health")
def health() -> dict:
    return {"ok": True}


@router.get("/version")
def version() -> dict:
    return {"app": config.APP_NAME, "version": config.VERSION}


@router.get("/capabilities")
def capabilities() -> dict:
    return {
        "app": config.APP_NAME,
        "version": config.VERSION,
        "desktop": config.is_desktop(),
        "python": sys.version.split()[0],
        "platform": platform.system().lower(),
        "engine": {"native": False, "commands": []},
        "frontend_built": config.dist_dir() is not None,
    }
