import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

const RECORDING = {
  session_dir: "/tmp/Folio/2026-05-28-share",
  label: "2026-05-28-share",
  duration_seconds: 600,
  mic_bytes: 1_000_000,
  system_bytes: null,
  mic_sample_rate: 16_000,
  system_sample_rate: null,
  created_at: "2026-05-28T14:00:00Z",
  has_transcript: true,
  suggested_title: "Shareable note",
  suggested_tags: [],
};

test("Share / export writes Markdown and opens the share sheet", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/#/");
  await page.getByText("Shareable note").first().click();
  await expect(page).toHaveURL(/#\/editor\//);

  await expect(page.getByRole("button", { name: /transcript & audio/i })).toBeVisible();

  await page.getByRole("button", { name: /more actions/i }).click();
  await page.getByRole("menuitem", { name: /share \/ export/i }).click();

  await expect
    .poll(async () => (await ipcCalls(page, "export_note_markdown")).length)
    .toBeGreaterThanOrEqual(1);
  await expect
    .poll(async () => (await ipcCalls(page, "share_paths")).length)
    .toBeGreaterThanOrEqual(1);
});
