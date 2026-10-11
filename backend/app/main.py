from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles

from app.core import config
from app.core.isolation import CrossOriginIsolation
from app.routers import engine, system


@asynccontextmanager
async def lifespan(_: FastAPI):
    yield


app = FastAPI(title=config.APP_NAME, version=config.VERSION, lifespan=lifespan)
app.add_middleware(CORSMiddleware, allow_origins=config.cors_origins(), allow_methods=["*"], allow_headers=["*"])
app.add_middleware(CrossOriginIsolation)
app.include_router(system.router, prefix="/api")
app.include_router(engine.router, prefix="/api/engine")

dist = config.dist_dir()
if dist is not None:
    app.mount("/", StaticFiles(directory=dist, html=True), name="frontend")
