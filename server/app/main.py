import asyncio
import contextlib

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.api.routes import auth, health, jobs, recordings
from app.core.config import enforce_production_config, get_settings
from app.core.logging import configure_logging
from app.db.base import init_db


def create_app() -> FastAPI:
    settings = get_settings()
    enforce_production_config(settings)
    configure_logging()

    @contextlib.asynccontextmanager
    async def lifespan(_: FastAPI):
        await init_db()
        worker_task: asyncio.Task | None = None
        if settings.run_worker_in_process:
            from app.workers.transcribe_worker import run_worker

            worker_task = asyncio.create_task(run_worker())
        try:
            yield
        finally:
            if worker_task is not None:
                worker_task.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await worker_task

    app = FastAPI(title=settings.app_name, version=settings.version, lifespan=lifespan)
    app.add_middleware(
        CORSMiddleware,
        allow_origins=settings.cors_origin_list,
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )
    app.include_router(health.router)
    app.include_router(auth.router)
    app.include_router(recordings.router)
    app.include_router(jobs.router)
    return app


app = create_app()
