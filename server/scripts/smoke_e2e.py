import hashlib
import sys
import time
import uuid

import httpx


def upload(client: httpx.Client, headers: dict, rec_id: str, name: str, path: str) -> None:
    data = open(path, "rb").read()
    sha = hashlib.sha256(data).hexdigest()
    mid = max(1, len(data) // 2)
    r = client.put(
        f"/v1/recordings/{rec_id}/channels/{name}",
        content=data[:mid],
        headers={**headers, "Upload-Offset": "0"},
    )
    r.raise_for_status()
    r = client.put(
        f"/v1/recordings/{rec_id}/channels/{name}",
        content=data[mid:],
        headers={
            **headers,
            "Upload-Offset": str(mid),
            "Upload-Complete": "true",
            "X-Content-Sha256": sha,
        },
    )
    r.raise_for_status()
    print(f"  uploaded {name}: {r.json()} ({len(data)} bytes)")


def main() -> int:
    args = sys.argv[1:]
    base = "http://127.0.0.1:8099"
    if args and args[0].startswith("http"):
        base = args.pop(0)
    if not args:
        print("usage: python scripts/smoke_e2e.py [BASE_URL] MIC_WAV [SYSTEM_WAV]")
        return 2
    mic = args[0]
    system = args[1] if len(args) > 1 else None

    email = f"e2e-{uuid.uuid4().hex[:8]}@example.com"
    password = "supersecret123"

    with httpx.Client(base_url=base, timeout=180) as client:
        caps = client.get("/v1/capabilities").json()
        print("capabilities:", caps)

        r = client.post("/v1/auth/register", json={"email": email, "password": password})
        r.raise_for_status()
        headers = {"Authorization": f"Bearer {r.json()['access_token']}"}
        print("registered:", email)

        client_id = uuid.uuid4().hex
        r = client.post(
            "/v1/recordings",
            json={"client_id": client_id, "label": "smoke-e2e"},
            headers=headers,
        )
        r.raise_for_status()
        rec_id = r.json()["id"]
        print("recording:", rec_id)

        upload(client, headers, rec_id, "mic", mic)
        if system:
            upload(client, headers, rec_id, "system", system)

        r = client.post(
            f"/v1/recordings/{rec_id}/transcribe", json={"language": None}, headers=headers
        )
        r.raise_for_status()
        job_id = r.json()["id"]
        print("job:", job_id)

        status = "queued"
        for _ in range(180):
            r = client.get(f"/v1/jobs/{job_id}", headers=headers)
            r.raise_for_status()
            body = r.json()
            status = body["status"]
            if status in ("succeeded", "failed"):
                print("job:", status, "progress:", body["progress"], "error:", body.get("error"))
                break
            time.sleep(1)

        r = client.get(f"/v1/recordings/{rec_id}/transcript", headers=headers)
        print("transcript HTTP", r.status_code)
        print(r.text)
        return 0 if status == "succeeded" and r.status_code == 200 else 1


if __name__ == "__main__":
    raise SystemExit(main())
