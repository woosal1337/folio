import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
});

test("Home is the landing surface and carries the note list controls", async ({
  page,
}) => {
  await expect(page.getByRole("button", { name: /take notes/i })).toBeVisible();
  await expect(page.getByRole("textbox", { name: /search recordings/i })).toBeVisible();
});

test("retired surfaces have no sidebar entry", async ({ page }) => {
  const nav = page.getByRole("navigation");
  await expect(nav.getByRole("link", { name: /^record$/i })).toHaveCount(0);
  await expect(nav.getByRole("link", { name: /^inbox$/i })).toHaveCount(0);
  await expect(nav.getByRole("link", { name: /^chat$/i })).toHaveCount(0);
  await expect(nav.getByRole("link", { name: /^my notes$/i })).toHaveCount(0);
  await expect(nav.getByRole("link", { name: /^tasks$/i })).toHaveCount(0);
  await expect(nav.getByRole("link", { name: /^memory$/i })).toHaveCount(0);
});

test("retired routes redirect home", async ({ page }) => {
  for (const route of [
    "/#/record",
    "/#/inbox",
    "/#/library",
    "/#/tasks",
    "/#/memory",
  ]) {
    await page.goto(route);
    await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  }
});
