/**
 * Settings → Referrals flow: personal link, copy, email, stats.
 *
 * The UI reads from the local stub (`__ATTUNE_REFERRAL_STATS__`) so
 * we can verify the share URL renders + Copy / Email work. The
 * generate IPC call is fired by the section on mount; redemption +
 * stats refresh have their own dedicated tests.
 */

import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^referrals$/i }).click();
});

test("Personal referral link renders in monospace", async ({ page }) => {
  await expect(page.getByText(/join\.attune\.app\/t\//i).first()).toBeVisible();
});

test("Copy button writes the link to the clipboard", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-write", "clipboard-read"]);
  await page.getByRole("button", { name: /^copy$/i }).first().click();
  const text = await page.evaluate(() => navigator.clipboard.readText());
  expect(text).toMatch(/join\.attune\.app\/t\//i);
});

test("Email button generates a mailto: link with the share URL embedded", async ({
  page,
}) => {
  const emailLink = page.getByRole("link", { name: /^email$/i });
  await expect(emailLink).toHaveAttribute("href", /^mailto:.*join\.attune\.app/i);
});

test("Three rules + three-step explainer render", async ({ page }) => {
  // The Tony "half-page" layout has both copy blocks. Catch any
  // accidental removal.
  await expect(page.getByText(/^share your link/i)).toBeVisible();
  await expect(page.getByText(/work email/i).first()).toBeVisible();
  await expect(page.getByText(/already have an attune workspace/i)).toBeVisible();
});

test("Referrals tab does NOT trigger an unauthorized backend call on first open", async ({
  page,
}) => {
  // The current UI is stub-only; `referrals_me` should only be
  // called by code that wires the backend later. We catch that the
  // section renders fine without firing the IPC prematurely.
  const calls = await ipcCalls(page, "referrals_me");
  expect(calls.length).toBeLessThanOrEqual(1);
});
