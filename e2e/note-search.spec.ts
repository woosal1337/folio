/**
 * Full-text search across note content (GET-165). A phrase that appears
 * only in a note's transcript (not its title) surfaces the note — with a
 * snippet — in both My Notes search and the Cmd-K palette.
 */

import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

const RECORDING = {
  session_dir: "/tmp/Attune/2026-05-28-budget",
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
  // The unique phrase lives only in the transcript body.
  transcript_text: "we approved the flamingo procurement for Q3",
};

test("My Notes search finds a phrase that only appears in the transcript", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/#/library");
  await expect(page.getByText("Budget meeting").first()).toBeVisible();

  const search = page.getByRole("textbox", { name: /search recordings/i });
  await search.fill("flamingo");

  // The note stays visible and shows the matched snippet.
  await expect(page.getByText("Budget meeting").first()).toBeVisible();
  await expect(page.getByText(/flamingo procurement/i).first()).toBeVisible();
});

test("My Notes search hides notes with no content match", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/#/library");
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

  // The note appears as a palette option with the snippet as its subtitle.
  await expect(
    page.getByRole("option").filter({ hasText: "Budget meeting" })
  ).toBeVisible();
  await expect(page.getByText(/flamingo procurement/i).first()).toBeVisible();
});
