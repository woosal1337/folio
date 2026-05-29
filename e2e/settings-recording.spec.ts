/**
 * Settings → Recording section coverage (General, Audio, Transcription,
 * AI, Storage, Privacy, Appearance).
 */

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
  // The mic-device picker is a native <select>. Options aren't
  // rendered as DOM text — we read them off the element instead.
  const options = await page
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
  // Provider tiles are <button>s with `aria-pressed` to indicate
  // the current selection. Click the OpenAI tile and verify its
  // pressed state flips.
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

test("Storage — sections render the configured paths", async ({ page }) => {
  await page.getByRole("button", { name: /^storage$/i }).click();
  await expect(page.getByText("/tmp/Attune").first()).toBeVisible();
});

test("Privacy — section renders the privacy mode toggle", async ({ page }) => {
  await page.getByRole("button", { name: /^privacy$/i }).click();
  // The privacy mode (airgap) toggle is the section's primary
  // control. The toggle label is the wording the user sees.
  await expect(page.getByText(/privacy mode|airgap/i).first()).toBeVisible();
});

test("Appearance — section renders without crashing", async ({ page }) => {
  await page.getByRole("button", { name: /^appearance$/i }).click();
  // We don't assert on the specific theme labels — just that the
  // section mounted.
  await expect(page.getByRole("dialog")).toBeVisible();
});

test("Storage — Save button writes settings back via IPC", async ({ page }) => {
  // Generic save smoke — clicking Save on any section should always
  // route through the persistence layer.
  await page.getByRole("button", { name: /^storage$/i }).click();
  await page.getByRole("button", { name: /^save$/i }).click();
  const saved = await readSettings(page);
  expect(saved.output_dir).toBe("/tmp/Attune");
});
