import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
});

test("Home is the landing surface", async ({ page }) => {
  await expect(page.getByRole("heading", { name: "Coming up" })).toBeVisible();
});

test("My Notes route loads", async ({ page }) => {
  await page.getByRole("link", { name: /my notes/i }).click();
  await expect(page.getByRole("heading", { name: /^my notes$/i })).toBeVisible();
});

test("Tasks route loads", async ({ page }) => {
  await page.getByRole("link", { name: /tasks/i }).click();
  await expect(page.getByRole("heading", { name: /^tasks$/i })).toBeVisible();
});

test("Memory route loads", async ({ page }) => {
  await page.getByRole("link", { name: /memory/i }).click();
  await expect(page.getByRole("heading", { name: /^memory$/i })).toBeVisible();
});

test("retired surfaces have no sidebar entry", async ({ page }) => {
  const nav = page.getByRole("navigation");
  await expect(nav.getByRole("link", { name: /^record$/i })).toHaveCount(0);
  await expect(nav.getByRole("link", { name: /^inbox$/i })).toHaveCount(0);
  await expect(nav.getByRole("link", { name: /^chat$/i })).toHaveCount(0);
});

test("retired routes redirect home", async ({ page }) => {
  await page.goto("/#/record");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.goto("/#/inbox");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
});
