from app.core.config import get_settings
from app.storage.local import LocalStorage

_storage: LocalStorage | None = None


def get_storage() -> LocalStorage:
    global _storage
    if _storage is None:
        settings = get_settings()
        _storage = LocalStorage(settings.storage_dir)
    return _storage
