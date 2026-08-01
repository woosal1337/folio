import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
});

test("Cmd-K opens the command palette", async ({ page }) => {
  await page.keyboard.press("Meta+K");

  await expect(page.getByPlaceholder(/search|command/i).first()).toBeVisible();
});

test("Cmd-Shift-/ opens the cheatsheet overlay", async ({ page }) => {
  await page.keyboard.press("Meta+Shift+/");
  await expect(page.getByText(/keyboard shortcuts|cheatsheet/i).first()).toBeVisible({
    timeout: 4000,
  });
});

test("Escape closes the command palette", async ({ page }) => {
  await page.keyboard.press("Meta+K");
  const palette = page.getByRole("dialog");
  await expect(palette.getByPlaceholder(/search|command/i)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
