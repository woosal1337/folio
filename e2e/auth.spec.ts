/**
 * Auth-flow scenarios — sign in, sign out, re-sign in.
 */

import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("sign out from Settings → Profile routes back to signup", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");

  // Main app loaded.
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();

  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^profile$/i }).click();
  await expect(page.getByText("ege@clinora.ai")).toBeVisible();

  await page.getByRole("button", { name: /^sign out$/i }).first().click();

  // App.tsx auth-gate flips → conductor takes the screen, sidebar
  // disappears.
  await expect(page.getByRole("heading", { name: /welcome to attune/i })).toBeVisible();
  await expect(page.getByRole("navigation")).toHaveCount(0);

  // IPC saw the logout call.
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
  await page.getByRole("button", { name: /^sign out$/i }).first().click();

  // We're back on signup. Walk through OTP again.
  await expect(page.getByRole("heading", { name: /welcome to attune/i })).toBeVisible();
  await page.getByPlaceholder(/you@company\.com/i).fill("ege@clinora.ai");
  await page.getByRole("button", { name: /^continue$/i }).first().click();
  await page.locator('input[id="code-0"]').fill("000000");
  await page.getByRole("button", { name: /verify and continue/i }).click();

  // No workspace setup — onboarding was already complete.
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
});

test("auth_status hydrates at boot — signed-in user skips signup", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
  // The auth_status probe ran on boot.
  const calls = await ipcCalls(page, "auth_status");
  expect(calls.length).toBeGreaterThanOrEqual(1);
});
