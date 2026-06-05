import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test("Cloud-cost dialog is dormant on the main app when nothing's pending", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();

  const dialogs = await page.getByRole("alertdialog").count();
  expect(dialogs).toBe(0);
});
