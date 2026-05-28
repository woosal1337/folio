/**
 * Sidebar navigation — every route renders without crashing.
 *
 * Each test clicks a sidebar entry and asserts the route heading
 * shows up. If any route throws at mount this fails loud.
 */

import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
});

test("Record route loads", async ({ page }) => {
  await page.getByRole("link", { name: /record/i }).click();
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
});

test("Inbox route loads", async ({ page }) => {
  await page.getByRole("link", { name: /inbox/i }).click();
  await expect(page.getByRole("heading", { name: /^inbox$/i })).toBeVisible();
});

test("Library route loads", async ({ page }) => {
  await page.getByRole("link", { name: /library/i }).click();
  await expect(page.getByRole("heading", { name: /^library$/i })).toBeVisible();
});

test("Tasks route loads", async ({ page }) => {
  await page.getByRole("link", { name: /tasks/i }).click();
  await expect(page.getByRole("heading", { name: /^tasks$/i })).toBeVisible();
});

test("Memory route loads", async ({ page }) => {
  await page.getByRole("link", { name: /memory/i }).click();
  await expect(page.getByRole("heading", { name: /^memory$/i })).toBeVisible();
});
