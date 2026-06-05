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

  await page.getByRole("button", { name: /take notes/i }).click();
  await expect(page).toHaveURL(/#\/editor\//);

  await expect(page.getByText(/live transcript will appear here/i)).toBeVisible();
});

test("the live-caption area stays hidden while capturing when the toggle is off", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /take notes/i }).click();
  await expect(page).toHaveURL(/#\/editor\//);

  await expect(page.getByText(/live transcript will appear here/i)).toHaveCount(0);
});
