import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("assign a note to a new folder from the header chip", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /quick note/i }).click();
  await expect(page).toHaveURL(/#\/editor\//);

  await page.getByRole("button", { name: /add to folder/i }).click();
  await page.getByRole("menuitem", { name: /new folder/i }).click();
  const nameField = page.getByRole("textbox", { name: /new folder name/i });
  await nameField.fill("Work");
  await nameField.press("Enter");

  await expect
    .poll(async () => (await ipcCalls(page, "set_note_folder")).length)
    .toBeGreaterThanOrEqual(1);

  await expect(page.getByText("Spaces", { exact: true })).toBeVisible();
  await expect
    .poll(async () => page.getByRole("button", { name: /^work$/i }).count())
    .toBeGreaterThanOrEqual(2);
});

test("filtering My Notes by folder shows only its notes", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    recordings: [
      {
        session_dir: "/tmp/Attune/2026-05-28-filed",
        label: "2026-05-28-filed",
        duration_seconds: 600,
        mic_bytes: 1_000_000,
        system_bytes: null,
        mic_sample_rate: 16_000,
        system_sample_rate: null,
        created_at: "2026-05-28T14:00:00Z",
        has_transcript: true,
        suggested_title: "Filed note",
        suggested_tags: [],
        folder: "Work",
      },
      {
        session_dir: "/tmp/Attune/2026-05-27-loose",
        label: "2026-05-27-loose",
        duration_seconds: 600,
        mic_bytes: 1_000_000,
        system_bytes: null,
        mic_sample_rate: 16_000,
        system_sample_rate: null,
        created_at: "2026-05-27T14:00:00Z",
        has_transcript: true,
        suggested_title: "Loose note",
        suggested_tags: [],
      },
    ],
  });

  await page.goto("/#/library");
  await expect(page.getByText("Filed note").first()).toBeVisible();
  await expect(page.getByText("Loose note").first()).toBeVisible();

  await page.goto("/#/library?folder=Work");
  await expect(page.getByRole("heading", { name: /^work$/i })).toBeVisible();
  await expect(page.getByText("Filed note").first()).toBeVisible();
  await expect(page.getByText("Loose note")).toHaveCount(0);
});
