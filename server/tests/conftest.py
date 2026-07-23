import os
import tempfile

_tmp = tempfile.mkdtemp(prefix="folio-test-")
os.environ.setdefault("FOLIO_JWT_SECRET", "test-secret-do-not-use")
os.environ["FOLIO_WHISPER_ENGINE"] = "stub"
os.environ["FOLIO_DATABASE_URL"] = f"sqlite+aiosqlite:///{_tmp}/test.db"
os.environ["FOLIO_STORAGE_DIR"] = f"{_tmp}/blobs"
os.environ["FOLIO_ALLOW_REGISTRATION"] = "true"

import pytest_asyncio  # noqa: E402
from httpx import ASGITransport, AsyncClient  # noqa: E402


@pytest_asyncio.fixture
async def client():
    from app.db.base import init_db
    from app.main import create_app

    await init_db()
    app = create_app()
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as http:
        yield http
