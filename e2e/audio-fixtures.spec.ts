import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { expect, test } from "@playwright/test";

const __filename = fileURLToPath(import.meta.url);
const FIXTURES_DIR = path.resolve(path.dirname(__filename), "fixtures/audio");

interface ManifestEntry {
  file: string;
  language: string;
  context: string;
  voice: string;
  chars: number;
  bytes?: number;
}

interface Manifest {
  generated_at: string;
  fixtures: Record<string, ManifestEntry>;
}

async function loadManifest(): Promise<Manifest> {
  const raw = await fs.readFile(path.join(FIXTURES_DIR, "manifest.json"), "utf8");
  return JSON.parse(raw) as Manifest;
}

test("manifest is present + lists at least one fixture per major language", async () => {
  const manifest = await loadManifest();
  const languages = new Set(Object.values(manifest.fixtures).map((f) => f.language));

  for (const expected of ["en", "tr", "de", "fr", "es", "ja"]) {
    expect(languages.has(expected), `missing language ${expected}`).toBe(true);
  }
});

test("every fixture file exists on disk + has MP3 magic", async () => {
  const manifest = await loadManifest();
  for (const [id, entry] of Object.entries(manifest.fixtures)) {
    const buf = await fs.readFile(path.join(FIXTURES_DIR, entry.file));
    expect(buf.length, `${id} is empty`).toBeGreaterThan(1024);

    const head = buf.subarray(0, 4);
    const isId3 = head[0] === 0x49 && head[1] === 0x44 && head[2] === 0x33;
    const isMpegSync = head[0] === 0xff && (head[1]! & 0xe0) === 0xe0;
    expect(isId3 || isMpegSync, `${id} is not an MP3`).toBe(true);
  }
});

test("Chromium decodes the English business fixture and metadata loads", async ({
  page,
}) => {
  const file = path.join(FIXTURES_DIR, "en-business-1min.mp3");
  const bytes = await fs.readFile(file);

  await page.setContent(
    `<!doctype html><html><body><audio id="player" preload="metadata" controls></audio></body></html>`
  );

  const meta = await page.evaluate(async (b64) => {
    const audio = document.getElementById("player") as HTMLAudioElement;
    const bin = atob(b64);
    const u8 = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
    const blob = new Blob([u8], { type: "audio/mpeg" });
    audio.src = URL.createObjectURL(blob);
    await new Promise<void>((resolve, reject) => {
      audio.addEventListener("loadedmetadata", () => resolve(), { once: true });
      audio.addEventListener("error", () => reject(new Error("audio decode failed")), {
        once: true,
      });

      audio.load();
    });
    return {
      duration: audio.duration,
      readyState: audio.readyState,
    };
  }, bytes.toString("base64"));

  expect(meta.duration).toBeGreaterThan(3);
  expect(meta.readyState).toBeGreaterThanOrEqual(1);
});
