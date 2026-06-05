import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("deleting a space asks for confirmation first", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");

  await page.getByRole("button", { name: /new folder/i }).click();
  const nameField = page.getByRole("textbox", { name: /new folder name/i });
  await nameField.fill("Personal");
  await nameField.press("Enter");
  await expect(page.getByText("Spaces", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: /delete folder personal/i }).click();

  const dialog = page.getByRole("dialog");
  await expect(dialog.getByText(/delete the "personal" space/i)).toBeVisible();
  expect((await ipcCalls(page, "delete_folder")).length).toBe(0);

  await dialog.getByRole("button", { name: /delete space/i }).click();
  await expect
    .poll(async () => (await ipcCalls(page, "delete_folder")).length)
    .toBeGreaterThanOrEqual(1);
});

test("cancelling the confirmation deletes nothing", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /new folder/i }).click();
  const nameField = page.getByRole("textbox", { name: /new folder name/i });
  await nameField.fill("Throwaway");
  await nameField.press("Enter");

  await page.getByRole("button", { name: /delete folder throwaway/i }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: /^cancel$/i }).click();

  expect((await ipcCalls(page, "delete_folder")).length).toBe(0);
  await expect(
    page.getByRole("button", { name: /delete folder throwaway/i })
  ).toBeVisible();
});
