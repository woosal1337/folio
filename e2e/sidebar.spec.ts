import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
});

test("Collapse button toggles the sidebar width", async ({ page }) => {
  const nav = page.getByRole("navigation");
  const beforeWidth = (await nav.boundingBox())?.width ?? 0;
  await page.getByRole("button", { name: /collapse sidebar/i }).click();

  await expect
    .poll(async () => (await nav.boundingBox())?.width ?? 0)
    .toBeLessThan(beforeWidth);
});

test("Home stays the active sidebar entry while an editor route is open", async ({
  page,
}) => {
  await expect(
    page.getByRole("link", { name: /^home$/i, includeHidden: false })
  ).toHaveAttribute("aria-current", "page");
});

test("Settings button at the bottom of the sidebar opens the Settings modal", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^settings$/i }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
});

test("Theme toggle button appears in the sidebar footer", async ({ page }) => {
  await expect(
    page.getByRole("button", { name: /(light|dark|system) mode/i }).first()
  ).toBeVisible();
});
