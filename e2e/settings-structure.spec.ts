import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.getByRole("button", { name: /^settings$/i }).click();
});

test("the Workspace group and Team section are gone", async ({ page }) => {
  const nav = page.getByRole("navigation", { name: /settings sections/i });
  await expect(nav).toBeVisible();
  await expect(nav.getByText("Workspace", { exact: true })).toHaveCount(0);
  await expect(nav.getByRole("button", { name: /^team$/i })).toHaveCount(0);
});

test("Analytics lives under the Personal group", async ({ page }) => {
  const nav = page.getByRole("navigation", { name: /settings sections/i });
  await expect(nav.getByText("Personal", { exact: true })).toBeVisible();
  await expect(nav.getByRole("button", { name: /^analytics$/i })).toBeVisible();
  await nav.getByRole("button", { name: /^analytics$/i }).click();

  await expect(page.getByRole("heading", { name: /^analytics$/i })).toBeVisible();
});

test("the Account group holds billing + integrations", async ({ page }) => {
  const nav = page.getByRole("navigation", { name: /settings sections/i });
  await expect(nav.getByText("Account", { exact: true })).toBeVisible();
  for (const item of ["Billing", "Usage", "Referrals", "Connectors", "Webhooks"]) {
    await expect(
      nav.getByRole("button", { name: new RegExp(`^${item}$`, "i") })
    ).toBeVisible();
  }
});

test("Billing is a personal Free/Pro matrix — no Team/Enterprise tiers", async ({
  page,
}) => {
  const nav = page.getByRole("navigation", { name: /settings sections/i });
  await nav.getByRole("button", { name: /^billing$/i }).click();
  await expect(page.getByRole("heading", { name: /^billing$/i })).toBeVisible();
  const dialog = page.getByRole("dialog");
  await expect(dialog.getByText("Enterprise", { exact: true })).toHaveCount(0);

  await expect(dialog.getByText("Free", { exact: true }).first()).toBeVisible();
  await expect(dialog.getByText("Pro", { exact: true }).first()).toBeVisible();
});
