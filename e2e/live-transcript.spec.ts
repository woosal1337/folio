/**
 * Live streaming transcript preview (GET-160). While a note is
 * capturing, the record dock shows a live-caption area. (Real captions
 * come from on-device Whisper over the mic tail; here we verify the dock
 * surfaces the live area in the capturing state.)
 */

import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test("the record dock shows a live-caption area while capturing", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  // Take notes → creates a note, opens it, and starts recording into it.
  await page.getByRole("button", { name: /take notes/i }).click();
  await expect(page).toHaveURL(/#\/editor\//);

  // The dock's live-transcript area appears once capture is active.
  await expect(page.getByText(/live transcript will appear here/i)).toBeVisible();
});
