/**
 * Live streaming transcript preview (GET-160), now gated behind the
 * "Live transcription" Beta toggle (off by default, PR #264). While a
 * note is capturing AND the toggle is on, the record dock shows a
 * live-caption area. When the toggle is off the dock shows no caption
 * preview (the backend emits none either).
 */

import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test("the live-caption area appears while capturing when the Beta toggle is on", async ({
  page,
}) => {
  await setupScenario(page, {
    startSignedIn: true,
    initialSettings: { live_transcript_enabled: true },
  });
  await page.goto("/");
  // Take notes → creates a note, opens it, and starts recording into it.
  await page.getByRole("button", { name: /take notes/i }).click();
  await expect(page).toHaveURL(/#\/editor\//);

  await expect(page.getByText(/live transcript will appear here/i)).toBeVisible();
});

test("the live-caption area stays hidden while capturing when the toggle is off", async ({
  page,
}) => {
  // Default settings → live_transcript_enabled is false.
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /take notes/i }).click();
  await expect(page).toHaveURL(/#\/editor\//);

  // Capturing, but no caption preview because the Beta toggle is off.
  await expect(page.getByText(/live transcript will appear here/i)).toHaveCount(0);
});
