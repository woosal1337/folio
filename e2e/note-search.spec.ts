import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

const RECORDING = {
  session_dir: "/tmp/Folio/2026-05-28-budget",
  label: "2026-05-28-budget",
  duration_seconds: 1200,
  mic_bytes: 1_000_000,
  system_bytes: null,
  mic_sample_rate: 16_000,
  system_sample_rate: null,
  created_at: "2026-05-28T14:00:00Z",
  has_transcript: true,
  suggested_title: "Budget meeting",
  suggested_tags: [],

  transcript_text: "we approved the flamingo procurement for Q3",
};

test("Home search finds a phrase that only appears in the transcript", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/#/");
  await expect(page.getByText("Budget meeting").first()).toBeVisible();

  const search = page.getByRole("textbox", { name: /search recordings/i });
  await search.fill("flamingo");

  await expect(page.getByText("Budget meeting").first()).toBeVisible();
  await expect(page.getByText(/flamingo procurement/i).first()).toBeVisible();
});

test("Home search hides notes with no content match", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/#/");
  const search = page.getByRole("textbox", { name: /search recordings/i });
  await search.fill("zzz-nonexistent-term");
  await expect(page.getByText("Budget meeting")).toHaveCount(0);
});

test("Cmd-K surfaces a transcript-only phrase with a snippet", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();

  await page.keyboard.press("Meta+K");
  const input = page.getByPlaceholder(/search|command/i).first();
  await expect(input).toBeVisible();
  await input.fill("flamingo");

  await expect(
    page.getByRole("option").filter({ hasText: "Budget meeting" })
  ).toBeVisible();
  await expect(page.getByText(/flamingo procurement/i).first()).toBeVisible();
});
