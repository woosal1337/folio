import hashlib
import uuid

import pytest


def test_production_refuses_default_jwt_secret():
    from app.core.config import DEFAULT_JWT_SECRET, Settings, enforce_production_config

    ok = Settings(environment="production", jwt_secret="a-real-secret")
    enforce_production_config(ok)

    bad = Settings(environment="production", jwt_secret=DEFAULT_JWT_SECRET)
    with pytest.raises(RuntimeError, match="FOLIO_JWT_SECRET"):
        enforce_production_config(bad)


async def test_health(client):
    resp = await client.get("/health")
    assert resp.status_code == 200
    assert resp.json()["status"] == "ok"


async def test_capabilities_reports_stub_engine(client):
    resp = await client.get("/v1/capabilities")
    assert resp.status_code == 200
    body = resp.json()
    assert body["engine"] == "stub"
    assert "max_upload_bytes" in body


async def test_register_login_me(client):
    email = f"user-{uuid.uuid4().hex}@example.com"
    resp = await client.post(
        "/v1/auth/register", json={"email": email, "password": "supersecret"}
    )
    assert resp.status_code == 200, resp.text
    tokens = resp.json()
    headers = {"Authorization": f"Bearer {tokens['access_token']}"}

    resp = await client.get("/v1/auth/me", headers=headers)
    assert resp.status_code == 200
    assert resp.json()["email"] == email

    resp = await client.post(
        "/v1/auth/login", json={"email": email, "password": "wrongpass"}
    )
    assert resp.status_code == 401


async def test_upload_transcribe_pipeline(client):
    email = f"user-{uuid.uuid4().hex}@example.com"
    resp = await client.post(
        "/v1/auth/register", json={"email": email, "password": "supersecret"}
    )
    headers = {"Authorization": f"Bearer {resp.json()['access_token']}"}

    client_id = uuid.uuid4().hex
    resp = await client.post(
        "/v1/recordings",
        json={"client_id": client_id, "label": "Test", "duration_seconds": 5},
        headers=headers,
    )
    assert resp.status_code == 200, resp.text
    rec_id = resp.json()["id"]

    data = b"RIFF----WAVEfake-audio-bytes"
    sha = hashlib.sha256(data).hexdigest()
    resp = await client.put(
        f"/v1/recordings/{rec_id}/channels/mic",
        content=data,
        headers={
            **headers,
            "Upload-Offset": "0",
            "Upload-Complete": "true",
            "X-Content-Sha256": sha,
        },
    )
    assert resp.status_code == 200, resp.text
    assert resp.json()["complete"] is True

    resp = await client.post(
        f"/v1/recordings/{rec_id}/transcribe", json={}, headers=headers
    )
    assert resp.status_code == 200, resp.text
    job = resp.json()
    assert job["status"] == "queued"

    from app.workers.transcribe_worker import process_next

    assert await process_next() is True

    resp = await client.get(f"/v1/jobs/{job['id']}", headers=headers)
    assert resp.json()["status"] == "succeeded"

    resp = await client.get(
        f"/v1/recordings/{rec_id}/transcript", headers=headers
    )
    assert resp.status_code == 200
    transcript = resp.json()
    assert transcript["channels"][0]["channel"] == "mic"
    assert transcript["channels"][0]["segments"][0]["text"] == "[stub transcription]"


async def test_resumable_offset_conflict(client):
    email = f"user-{uuid.uuid4().hex}@example.com"
    resp = await client.post(
        "/v1/auth/register", json={"email": email, "password": "supersecret"}
    )
    headers = {"Authorization": f"Bearer {resp.json()['access_token']}"}
    client_id = uuid.uuid4().hex
    resp = await client.post(
        "/v1/recordings", json={"client_id": client_id}, headers=headers
    )
    rec_id = resp.json()["id"]

    resp = await client.put(
        f"/v1/recordings/{rec_id}/channels/system",
        content=b"first-chunk",
        headers={**headers, "Upload-Offset": "0"},
    )
    assert resp.status_code == 200
    assert resp.json()["offset"] == len(b"first-chunk")

    resp = await client.put(
        f"/v1/recordings/{rec_id}/channels/system",
        content=b"bad",
        headers={**headers, "Upload-Offset": "999"},
    )
    assert resp.status_code == 409
