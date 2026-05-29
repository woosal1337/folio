/**
 * Enhanced-notes templates (GET-164). A transcribed note exposes a
 * template picker in its header; choosing one persists via
 * `set_note_template` and the selection sticks.
 */

import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("pick a template on a note and it persists", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    recordings: [
      {
        session_dir: "/tmp/Attune/2026-05-28-templated",
        label: "2026-05-28-templated",
        duration_seconds: 900,
        mic_bytes: 1_000_000,
        system_bytes: null,
        mic_sample_rate: 16_000,
        system_sample_rate: null,
        created_at: "2026-05-28T14:00:00Z",
        has_transcript: true,
        suggested_title: "Sprint sync",
        suggested_tags: [],
      },
    ],
  });

  // Open the note from My Notes so the editor has its summary in state.
  await page.goto("/#/library");
  await page.getByText("Sprint sync").first().click();
  await expect(page).toHaveURL(/#\/editor\//);

  // The header template picker defaults to General; switch to Standup.
  const picker = page.getByRole("combobox", { name: /enhanced-notes template/i });
  await expect(picker).toHaveValue("generic");
  await picker.selectOption("standup");

  await expect
    .poll(async () => (await ipcCalls(page, "set_note_template")).length)
    .toBeGreaterThanOrEqual(1);
  await expect(picker).toHaveValue("standup");
});
