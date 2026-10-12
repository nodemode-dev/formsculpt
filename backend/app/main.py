import asyncio
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles

from app.core import config
from app.core.isolation import CrossOriginIsolation
from app.routers import engine, mcp, nodemode, plugins, system
from app.services import autosave


@asynccontextmanager
async def lifespan(_: FastAPI):
    task = asyncio.create_task(autosave.loop())
    yield
    task.cancel()


app = FastAPI(title=config.APP_NAME, version=config.VERSION, lifespan=lifespan)
app.add_middleware(CORSMiddleware, allow_origins=config.cors_origins(), allow_methods=["*"], allow_headers=["*"])
app.add_middleware(CrossOriginIsolation)
app.include_router(system.router, prefix="/api")
app.include_router(engine.router, prefix="/api/engine")
app.include_router(nodemode.router)
app.include_router(mcp.router)
app.include_router(plugins.router, prefix="/api")

dist = config.dist_dir()
if dist is not None:
    app.mount("/", StaticFiles(directory=dist, html=True), name="frontend")
