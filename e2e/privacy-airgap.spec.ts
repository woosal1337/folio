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

test("Aggregate-stats opt-in is OFF by default — Attune does not collect telemetry", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  expect((await readSettings(page)).share_aggregate_stats).toBe(false);
});

test("Auto-delete period defaults to 90 days (GDPR Art. 5(1)(c) baseline)", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  expect((await readSettings(page)).auto_delete_period_days).toBe(90);
});

test("Default link sharing defaults to workspace_only — not 'anyone with link'", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  expect((await readSettings(page)).default_link_sharing).toBe("workspace_only");
});
