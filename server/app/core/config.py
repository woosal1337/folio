from functools import lru_cache

from pydantic_settings import BaseSettings, SettingsConfigDict

DEFAULT_JWT_SECRET = "change-me-in-production"


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_prefix="FOLIO_", env_file=".env", extra="ignore"
    )

    app_name: str = "Folio Server"
    version: str = "0.1.0"
    environment: str = "development"

    jwt_secret: str = DEFAULT_JWT_SECRET
    jwt_algorithm: str = "HS256"
    access_token_ttl_minutes: int = 30
    refresh_token_ttl_days: int = 30
    allow_registration: bool = True

    database_url: str = "sqlite+aiosqlite:///./data/folio.db"

    storage_backend: str = "local"
    storage_dir: str = "./data/blobs"
    max_upload_bytes: int = 2 * 1024 * 1024 * 1024

    whisper_engine: str = "auto"
    whisper_model: str = "base"
    whisper_device: str = "auto"
    whisper_compute_type: str = "auto"
    diarization_enabled: bool = False

    worker_poll_interval_seconds: float = 2.0
    run_worker_in_process: bool = False

    cors_origins: str = "*"

    @property
    def cors_origin_list(self) -> list[str]:
        return [o.strip() for o in self.cors_origins.split(",") if o.strip()]

    @property
    def is_production(self) -> bool:
        return self.environment.lower() in {"production", "prod"}


def enforce_production_config(settings: Settings) -> None:
    if settings.is_production and settings.jwt_secret == DEFAULT_JWT_SECRET:
        raise RuntimeError(
            "FOLIO_JWT_SECRET is still the default value — set a strong secret "
            "before running in production"
        )


@lru_cache
def get_settings() -> Settings:
    return Settings()
