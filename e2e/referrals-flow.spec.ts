import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^referrals$/i }).click();
});

test("Personal referral link renders in monospace", async ({ page }) => {
  await expect(page.getByText(/join\.folio\.app\/t\//i).first()).toBeVisible();
});

test("Copy button writes the link to the clipboard", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-write", "clipboard-read"]);
  await page
    .getByRole("button", { name: /^copy$/i })
    .first()
    .click();
  const text = await page.evaluate(() => navigator.clipboard.readText());
  expect(text).toMatch(/join\.folio\.app\/t\//i);
});

test("Email button generates a mailto: link with the share URL embedded", async ({
  page,
}) => {
  const emailLink = page.getByRole("link", { name: /^email$/i });
  await expect(emailLink).toHaveAttribute("href", /^mailto:.*join\.folio\.app/i);
});

test("Three rules + three-step explainer render", async ({ page }) => {
  await expect(page.getByText(/^share your link/i)).toBeVisible();
  await expect(page.getByText(/work email/i).first()).toBeVisible();
  await expect(page.getByText(/already have an folio workspace/i)).toBeVisible();
});

test("Referrals tab does NOT trigger an unauthorized backend call on first open", async ({
  page,
}) => {
  const calls = await ipcCalls(page, "referrals_me");
  expect(calls.length).toBeLessThanOrEqual(1);
});
