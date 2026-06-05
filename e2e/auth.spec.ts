import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("sign out from Settings → Profile routes back to signup", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");

  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();

  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^profile$/i }).click();
  await expect(page.getByText("ege@clinora.ai")).toBeVisible();

  await page
    .getByRole("button", { name: /^sign out$/i })
    .first()
    .click();

  await expect(page.getByRole("heading", { name: /welcome to folio/i })).toBeVisible();
  await expect(page.getByRole("navigation")).toHaveCount(0);

  const calls = await ipcCalls(page, "auth_logout");
  expect(calls).toHaveLength(1);
});

test("re-sign-in after sign out lands directly on main app (no workspace setup)", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");

  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^profile$/i }).click();
  await page
    .getByRole("button", { name: /^sign out$/i })
    .first()
    .click();

  await expect(page.getByRole("heading", { name: /welcome to folio/i })).toBeVisible();
  await page.getByPlaceholder(/you@company\.com/i).fill("ege@clinora.ai");
  await page
    .getByRole("button", { name: /^continue$/i })
    .first()
    .click();
  await page.locator('input[id="code-0"]').fill("000000");
  await page.getByRole("button", { name: /verify and continue/i }).click();

  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
});

test("auth_status hydrates at boot — signed-in user skips signup", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();

  const calls = await ipcCalls(page, "auth_status");
  expect(calls.length).toBeGreaterThanOrEqual(1);
});
