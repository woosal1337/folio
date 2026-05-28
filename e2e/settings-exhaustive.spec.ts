/**
 * Exhaustive settings coverage — every toggle / select / input the
 * user can edit must:
 *   1. Render with the seeded initial value.
 *   2. Toggle / change in the UI.
 *   3. On Save, write the corresponding field on the in-memory
 *      settings backend with the new value.
 *
 * Settings-store sync is the contract between the UI and the Tauri
 * process (and through it the attune-api backend). This file is the
 * regression net for that contract.
 */

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
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
  await page.getByRole("button", { name: /^settings$/i }).click();
});

// ---- Preferences -------------------------------------------------

test("Preferences → live_meeting_indicator persists when toggled", async ({ page }) => {
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("switch", { name: /live meeting indicator/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).live_meeting_indicator).toBe(false);
});

test("Preferences → open_at_login persists when toggled", async ({ page }) => {
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("switch", { name: /open attune when you log in/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).open_at_login).toBe(true);
});

test("Preferences → move_aside_in_meetings persists when toggled", async ({ page }) => {
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("switch", { name: /move attune aside in meetings/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).move_aside_in_meetings).toBe(true);
});

test("Preferences → privacy_tier_band_enabled persists when toggled", async ({ page }) => {
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("switch", { name: /privacy tier colour band/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).privacy_tier_band_enabled).toBe(true);
});

test("Preferences → always_open_shared_links persists when toggled off", async ({ page }) => {
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("switch", { name: /always open shared links in attune/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).always_open_shared_links).toBe(false);
});

test("Preferences → default_link_sharing changes to anyone_with_link", async ({ page }) => {
  await page.getByRole("button", { name: /^preferences$/i }).click();
  const select = page.getByLabel(/default link sharing/i);
  await select.selectOption("anyone_with_link");
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).default_link_sharing).toBe("anyone_with_link");
});

test("Preferences → auto_delete_period switches to 30 days", async ({ page }) => {
  await page.getByRole("button", { name: /^preferences$/i }).click();
  const select = page.getByLabel(/auto-delete transcripts/i);
  await select.selectOption({ value: "30" });
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).auto_delete_period_days).toBe(30);
});

// ---- Calendar ----------------------------------------------------

test("Calendar → show_upcoming_meetings_in_menubar persists when toggled off", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^calendar$/i }).click();
  await page.getByRole("switch", { name: /show upcoming meetings in menu bar/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).show_upcoming_meetings_in_menubar).toBe(false);
});

// ---- Notifications -----------------------------------------------

test("Notifications → notify_scheduled_meetings persists when toggled off", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^notifications$/i }).click();
  await page.getByRole("switch", { name: /scheduled meetings/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).notify_scheduled_meetings).toBe(false);
});

test("Notifications → notify_auto_detected_meetings persists when toggled off", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^notifications$/i }).click();
  await page.getByRole("switch", { name: /auto-detected meetings/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).notify_auto_detected_meetings).toBe(false);
});

// ---- Audio -------------------------------------------------------

test("Audio → voice_processing_enabled persists when toggled off", async ({ page }) => {
  await page.getByRole("button", { name: /^audio$/i }).click();
  await page.getByRole("switch", { name: /voice processing/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).voice_processing_enabled).toBe(false);
});

// ---- Transcription -----------------------------------------------

test("Transcription → switching provider persists transcriber=openai", async ({ page }) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();
  await page.getByRole("button", { name: /openai whisper api/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).transcriber).toBe("openai");
});

test("Transcription → auto_transcribe_enabled persists when toggled off", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();
  // The auto-transcribe row pairs a Label with htmlFor, which gives
  // its Switch an accessibility name we can match.
  await page.getByRole("switch", { name: /auto-?transcribe/i }).first().click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).auto_transcribe_enabled).toBe(false);
});

test("Transcription → auto_vad_enabled persists when toggled off", async ({ page }) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();
  await page.getByRole("switch", { name: /voice activity detection|strip silence/i })
    .first()
    .click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).auto_vad_enabled).toBe(false);
});

// ---- AI -----------------------------------------------------------

test("AI → master toggle off ripples to every agent flag", async ({ page }) => {
  // The beforeEach already opened Settings with `auto_*_enabled=false`
  // (the freshSettings default). The master toggle in section-ai.tsx
  // reads `masterOn = ALL auto_* flags are true`. We pre-flip them to
  // true via the IPC before the user clicks the master toggle off.
  await page.getByRole("button", { name: /^ai$/i }).click();
  // Mutate the in-page settings to "all-on" via a direct save, then
  // click Save in the UI to re-read.
  await page.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    const s = w.__ATTUNE_SETTINGS__ as Record<string, unknown>;
    s.auto_summarize_enabled = true;
    s.auto_extract_tasks_enabled = true;
    s.auto_extract_memories_enabled = true;
    s.auto_name_enabled = true;
  });
  // Force the SettingsModal to reload from the stub by closing +
  // re-opening Settings.
  await page.getByRole("button", { name: /^cancel$/i }).click();
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^ai$/i }).click();
  // Now the master toggle reads checked=true. Click to flip off.
  await page.getByRole("switch", { name: /ai on every recording/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
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
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).briefing_language).toBe("tr");
});

// ---- Privacy ------------------------------------------------------

test("Privacy → privacy_mode toggle flips airgap on", async ({ page }) => {
  await page.getByRole("button", { name: /^privacy$/i }).click();
  // The Privacy panel renders two switches; the first is the airgap
  // (privacy mode). The label inside the row is "Privacy mode".
  const switches = page.getByRole("switch");
  await switches.first().click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).privacy_mode).toBe(true);
});

test("Privacy → share_aggregate_stats stays off by default (opt-in only)", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^privacy$/i }).click();
  // Two switches: privacy_mode + share_aggregate_stats. Confirm the
  // second one defaults to OFF.
  const switches = page.getByRole("switch");
  await expect(switches.nth(1)).not.toBeChecked();
});

// ---- Storage ------------------------------------------------------

test("Storage → wav_retention_days input persists when edited", async ({ page }) => {
  await page.getByRole("button", { name: /^storage$/i }).click();
  // Numeric retention input. Find by inputmode + currency-style hint.
  const input = page.locator("input[inputmode='numeric']").first();
  await input.fill("7");
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).wav_retention_days).toBe(7);
});

// ---- Workspace General -------------------------------------------

test("Workspace General → workspace_auto_join persists when toggled off", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^general$/i }).nth(1).click();
  await page.getByRole("switch", { name: /allow teammates to join automatically/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  expect((await readSettings(page)).workspace_auto_join).toBe(false);
});

// ---- Round-trip drift check --------------------------------------

test("save_settings — saving with no changes is a no-op for unchanged fields", async ({
  page,
}) => {
  const before = await readSettings(page);
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("button", { name: /^save$/i }).last().click();
  const after = await readSettings(page);
  // Every field that wasn't touched must round-trip identical.
  for (const k of Object.keys(before) as Array<keyof typeof before>) {
    expect(after[k], `field ${k} drifted on a no-op save`).toEqual(before[k]);
  }
});

test("freshSettings helper returns a complete + valid settings shape", async () => {
  const s = freshSettings();
  // Required string + enum invariants.
  expect(s.theme).toMatch(/^(light|dark)$/);
  expect(s.transcriber).toMatch(/^(local_whisper|openai)$/);
  expect(s.auto_delete_period_days).toBe(90);
  expect(s.default_link_sharing).toBe("workspace_only");
});
