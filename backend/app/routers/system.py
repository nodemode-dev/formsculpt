import io
import platform
import sys
import zipfile

from fastapi import APIRouter, HTTPException
from fastapi.responses import Response

from app.core import config
from app.services import engine

router = APIRouter()


@router.get("/integrations/blender")
def blender_addon() -> Response:
    """The Blender Live Link add-on as an extension zip (Blender: Get Extensions, Install from Disk)."""
    folder = config.blender_addon_dir()
    if folder is None:
        raise HTTPException(status_code=404, detail="the Blender add-on is not in this build")
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(folder.rglob("*")):
            if path.is_file() and "__pycache__" not in path.parts:
                archive.write(path, path.relative_to(folder).as_posix())
    name = f"formsculptor_link-{config.VERSION}.zip"
    return Response(buffer.getvalue(), media_type="application/zip", headers={"Content-Disposition": f'attachment; filename="{name}"'})


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
        "engine": engine.status(),
        "frontend_built": config.dist_dir() is not None,
    }
