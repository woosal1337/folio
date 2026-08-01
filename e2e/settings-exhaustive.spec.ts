import { expect, test } from "@playwright/test";

import {
  freshSettings,
  ipcCalls,
  readSettings,
  setupScenario,
} from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.getByRole("button", { name: /^settings$/i }).click();
});

test("Audio → voice_processing_enabled persists when toggled off", async ({ page }) => {
  await page.getByRole("button", { name: /^audio$/i }).click();
  await page.getByRole("switch", { name: /voice processing/i }).click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  expect((await readSettings(page)).voice_processing_enabled).toBe(false);
});

test("Transcription → switching provider persists transcriber=openai", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();
  await page.getByRole("button", { name: /openai whisper api/i }).click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  expect((await readSettings(page)).transcriber).toBe("openai");
});

test("Transcription → auto_transcribe_enabled persists when toggled off", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();

  await page
    .getByRole("switch", { name: /auto-?transcribe/i })
    .first()
    .click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  expect((await readSettings(page)).auto_transcribe_enabled).toBe(false);
});

test("Transcription → auto_vad_enabled persists when toggled off", async ({ page }) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();
  await page
    .getByRole("switch", { name: /voice activity detection|strip silence/i })
    .first()
    .click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  expect((await readSettings(page)).auto_vad_enabled).toBe(false);
});

test("AI → master toggle off ripples to every agent flag", async ({ page }) => {
  await page.getByRole("button", { name: /^ai$/i }).click();

  await page.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    const s = w.__FOLIO_SETTINGS__ as Record<string, unknown>;
    s.auto_summarize_enabled = true;
    s.auto_extract_tasks_enabled = true;
    s.auto_extract_memories_enabled = true;
    s.auto_name_enabled = true;
  });

  await page.getByRole("button", { name: /^cancel$/i }).click();
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^ai$/i }).click();

  await page.getByRole("switch", { name: /ai on every recording/i }).click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  const saved = await readSettings(page);
  expect(saved.auto_summarize_enabled).toBe(false);
  expect(saved.auto_extract_tasks_enabled).toBe(false);
  expect(saved.auto_extract_memories_enabled).toBe(false);
  expect(saved.auto_name_enabled).toBe(false);
});

test("AI → briefing_language switches to Turkish", async ({ page }) => {
  await page.getByRole("button", { name: /^ai$/i }).click();
  const select = page.getByLabel(/briefing language/i);
  await select.selectOption("tr");
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  expect((await readSettings(page)).briefing_language).toBe("tr");
});

test("Privacy → privacy_mode toggle flips airgap on", async ({ page }) => {
  await page.getByRole("button", { name: /^privacy$/i }).click();

  const switches = page.getByRole("switch");
  await switches.first().click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  expect((await readSettings(page)).privacy_mode).toBe(true);
});

test("Storage → wav_retention_days input persists when edited", async ({ page }) => {
  await page.getByRole("button", { name: /^storage$/i }).click();

  const input = page.locator("input[inputmode='numeric']").first();
  await input.fill("7");
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  expect((await readSettings(page)).wav_retention_days).toBe(7);
});

test("save_settings — saving with no changes is a no-op for unchanged fields", async ({
  page,
}) => {
  const before = await readSettings(page);
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  const after = await readSettings(page);

  for (const k of Object.keys(before) as Array<keyof typeof before>) {
    expect(after[k], `field ${k} drifted on a no-op save`).toEqual(before[k]);
  }
});

test("freshSettings helper returns a complete + valid settings shape", async () => {
  const s = freshSettings();

  expect(s.theme).toMatch(/^(light|dark)$/);
  expect(s.transcriber).toMatch(/^(local_whisper|openai)$/);
  expect(s.privacy_mode).toBe(false);
  expect(s.remote_auto_upload).toBe(false);
});
