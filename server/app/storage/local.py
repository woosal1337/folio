import hashlib
from pathlib import Path


class LocalStorage:
    def __init__(self, root: str) -> None:
        self.root = Path(root)
        self.root.mkdir(parents=True, exist_ok=True)

    def _part(self, key: str) -> Path:
        return self.root / (key + ".part")

    def _final(self, key: str) -> Path:
        return self.root / key

    def current_size(self, key: str) -> int:
        final = self._final(key)
        if final.exists():
            return final.stat().st_size
        part = self._part(key)
        return part.stat().st_size if part.exists() else 0

    def is_complete(self, key: str) -> bool:
        return self._final(key).exists()

    def append(self, key: str, offset: int, data: bytes) -> int:
        part = self._part(key)
        part.parent.mkdir(parents=True, exist_ok=True)
        existing = part.stat().st_size if part.exists() else 0
        if offset != existing:
            raise ValueError(f"offset mismatch: expected {existing}, got {offset}")
        with open(part, "ab") as fh:
            fh.write(data)
        return part.stat().st_size

    def finalize(self, key: str, expected_sha256: str | None = None) -> tuple[int, str]:
        part = self._part(key)
        if not part.exists():
            raise FileNotFoundError(f"no staged upload for {key}")
        digest = hashlib.sha256()
        with open(part, "rb") as fh:
            for chunk in iter(lambda: fh.read(1024 * 1024), b""):
                digest.update(chunk)
        actual = digest.hexdigest()
        if expected_sha256 and expected_sha256.lower() != actual:
            raise ValueError("sha256 mismatch")
        final = self._final(key)
        final.parent.mkdir(parents=True, exist_ok=True)
        part.replace(final)
        return final.stat().st_size, actual

    def local_path(self, key: str) -> Path:
        return self._final(key)

    def delete(self, key: str) -> None:
        for path in (self._final(key), self._part(key)):
            if path.exists():
                path.unlink()
