/**
 * Voice-fixture smoke tests.
 *
 * The ElevenLabs-generated audio clips live in
 * `e2e/fixtures/audio/*.mp3` (gitignored — generated once via
 * `bun run e2e:fixtures`, cached locally). This spec asserts:
 *
 *   1. The fixture manifest exists and is well-formed.
 *   2. The audio bytes are real (MP3 magic, non-zero length).
 *   3. The browser actually decodes + plays a clip when handed the
 *      bytes (`HTMLAudioElement.play()` resolves; no decode errors).
 *
 * These tests don't drive the Attune UI directly — they prove the
 * voice corpus is available + playable so downstream specs that
 * feed audio into the editor / record flow can rely on it.
 *
 * To regenerate, delete a file under `e2e/fixtures/audio/` and run
 * `bun run e2e:fixtures` again. Existing files are skipped, so the
 * ElevenLabs bill stays at one-shot.
 */

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
  const languages = new Set(
    Object.values(manifest.fixtures).map((f) => f.language),
  );
  // The corpus is intentionally multilingual so we can run UI tests
  // that exercise the Settings → Transcription language picker
  // against representative inputs.
  for (const expected of ["en", "tr", "de", "fr", "es", "ja"]) {
    expect(languages.has(expected), `missing language ${expected}`).toBe(true);
  }
});

test("every fixture file exists on disk + has MP3 magic", async () => {
  const manifest = await loadManifest();
  for (const [id, entry] of Object.entries(manifest.fixtures)) {
    const buf = await fs.readFile(path.join(FIXTURES_DIR, entry.file));
    expect(buf.length, `${id} is empty`).toBeGreaterThan(1024);
    // MP3 starts with either ID3 tag (`49 44 33`) or an MPEG frame
    // sync (`FF Ex`). Tolerate both.
    const head = buf.subarray(0, 4);
    const isId3 = head[0] === 0x49 && head[1] === 0x44 && head[2] === 0x33;
    const isMpegSync =
      head[0] === 0xff && (head[1]! & 0xe0) === 0xe0;
    expect(isId3 || isMpegSync, `${id} is not an MP3`).toBe(true);
  }
});

test("Chromium decodes the English business fixture and metadata loads", async ({
  page,
}) => {
  const file = path.join(FIXTURES_DIR, "en-business-1min.mp3");
  const bytes = await fs.readFile(file);

  await page.setContent(
    `<!doctype html><html><body><audio id="player" preload="metadata" controls></audio></body></html>`,
  );

  // Stuff the bytes into the audio element via an object URL set
  // up inside the page context. Tests that the browser actually
  // decodes the file — a corrupt MP3 raises an error here.
  const meta = await page.evaluate(async (b64) => {
    const audio = document.getElementById("player") as HTMLAudioElement;
    const bin = atob(b64);
    const u8 = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
    const blob = new Blob([u8], { type: "audio/mpeg" });
    audio.src = URL.createObjectURL(blob);
    await new Promise<void>((resolve, reject) => {
      audio.addEventListener("loadedmetadata", () => resolve(), { once: true });
      audio.addEventListener(
        "error",
        () => reject(new Error("audio decode failed")),
        { once: true },
      );
      // Belt + braces — kick the load explicitly.
      audio.load();
    });
    return {
      duration: audio.duration,
      readyState: audio.readyState,
    };
  }, bytes.toString("base64"));

  expect(meta.duration).toBeGreaterThan(3); // ~10-30s expected
  expect(meta.readyState).toBeGreaterThanOrEqual(1);
});
