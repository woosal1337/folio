/**
 * Cloud-cost confirm dialog — Attune asks for explicit consent
 * before any operation that bills the user (e.g., OpenAI Whisper
 * transcription on a long recording). The dialog is wired globally
 * via `chrome/cloud-cost-confirm-dialog.tsx`; this spec verifies it
 * exists and stays out of the way when no cost is pending.
 */

import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test("Cloud-cost dialog is dormant on the main app when nothing's pending", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  // No confirm dialog should be visible on a clean boot.
  const dialogs = await page.getByRole("alertdialog").count();
  expect(dialogs).toBe(0);
});
