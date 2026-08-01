import { expect, test } from "@playwright/test";

import { ipcCalls, readSettings, setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.getByRole("button", { name: /^settings$/i }).click();
});

test("Profile — display name persists to the backend on blur (account_update)", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^profile$/i }).click();
  const nameInput = page.getByLabel(/display name/i);
  await nameInput.fill("Ege Çelebi (e2e)");

  await nameInput.blur();

  await expect
    .poll(async () => (await ipcCalls(page, "account_update")).length)
    .toBeGreaterThanOrEqual(1);
  const calls = await ipcCalls(page, "account_update");
  expect(calls[calls.length - 1].args).toMatchObject({
    displayName: "Ege Çelebi (e2e)",
  });
});

test("Profile — editing then re-opening keeps the saved display name", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^profile$/i }).click();
  const nameInput = page.getByLabel(/display name/i);
  await nameInput.fill("Persisted Name");
  await nameInput.blur();
  await expect
    .poll(async () => (await ipcCalls(page, "account_update")).length)
    .toBeGreaterThanOrEqual(1);

  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("button", { name: /^profile$/i }).click();
  await expect(page.getByLabel(/display name/i)).toHaveValue("Persisted Name");
});

test("Profile — email surfaces from the auth store identity", async ({ page }) => {
  await page.getByRole("button", { name: /^profile$/i }).click();
  await expect(page.getByText("ege@clinora.ai")).toBeVisible();
});
