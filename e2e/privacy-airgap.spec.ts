import { expect, test } from "@playwright/test";

import { readSettings, setupScenario } from "./fixtures/scenario";

test("Privacy mode toggle saves `privacy_mode: true`", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^privacy$/i }).click();
  await page.getByRole("switch").first().click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  expect((await readSettings(page)).privacy_mode).toBe(true);
});

test("Privacy mode is OFF by default for new accounts", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  expect((await readSettings(page)).privacy_mode).toBe(false);
});
