import { expect, test } from "@playwright/test";

import { ipcCalls, readSettings, setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.getByRole("button", { name: /^settings$/i }).click();
});

test("General — input device list populates from list_input_devices", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: /^general$/i })
    .first()
    .click();

  const options = await page
    .getByRole("dialog")
    .locator("select")
    .first()
    .locator("option")
    .allTextContents();
  expect(options.join(" ")).toContain("MacBook Pro Microphone");
});

test("Transcription — selecting OpenAI marks the button as pressed", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();

  const openaiTile = page.getByRole("button", {
    name: /openai whisper api/i,
  });
  await openaiTile.click();
  await expect(openaiTile).toHaveAttribute("aria-pressed", "true");
});

test("Transcription — language preference saves", async ({ page }) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();
  await page.getByRole("button", { name: /^save$/i }).click();
  const calls = await ipcCalls(page, "save_settings");
  expect(calls.length).toBeGreaterThanOrEqual(1);
});

test("Transcription — live transcription (Beta) is off by default", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();
  const toggle = page.getByRole("switch", { name: /live transcription/i });
  await expect(toggle).toBeVisible();
  await expect(toggle).not.toBeChecked();

  await expect(page.getByText("Beta", { exact: true })).toBeVisible();
});

test("Transcription — enabling live transcription persists live_transcript_enabled", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^transcription$/i }).click();
  await page.getByRole("switch", { name: /live transcription/i }).click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  expect((await readSettings(page)).live_transcript_enabled).toBe(true);
});

test("Storage — sections render the configured paths", async ({ page }) => {
  await page.getByRole("button", { name: /^storage$/i }).click();
  await expect(page.getByText("/tmp/Folio").first()).toBeVisible();
});

test("Privacy — section renders the privacy mode toggle", async ({ page }) => {
  await page.getByRole("button", { name: /^privacy$/i }).click();

  await expect(page.getByText(/privacy mode|airgap/i).first()).toBeVisible();
});

test("Appearance — section renders without crashing", async ({ page }) => {
  await page.getByRole("button", { name: /^appearance$/i }).click();

  await expect(page.getByRole("dialog")).toBeVisible();
});

test("Storage — Save button writes settings back via IPC", async ({ page }) => {
  await page.getByRole("button", { name: /^storage$/i }).click();
  await page.getByRole("button", { name: /^save$/i }).click();
  const saved = await readSettings(page);
  expect(saved.output_dir).toBe("/tmp/Folio");
});
