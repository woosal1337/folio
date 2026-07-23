import { expect, test } from "@playwright/test";

import { readSettings, setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^transcription$/i }).click();
});

test("Local Whisper stays selectable alongside the new remote option", async ({
  page,
}) => {
  const local = page.getByRole("button", { name: /local whisper/i });
  await local.click();
  await expect(local).toHaveAttribute("aria-pressed", "true");
  expect((await readSettings(page)).transcriber).toBeDefined();
});

test("Remote server — selectable, shows the server summary and auto-upload", async ({
  page,
}) => {
  const tile = page.getByRole("button", { name: /remote server/i });
  await tile.click();
  await expect(tile).toHaveAttribute("aria-pressed", "true");

  await expect(page.getByText(/folio server/i).first()).toBeVisible();
  await expect(page.getByText(/no endpoint configured/i)).toBeVisible();
  await expect(page.getByText(/not signed in/i)).toBeVisible();
  await expect(page.getByRole("switch", { name: /auto-upload/i })).toBeVisible();
});

test("Remote server — Manage in Account closes settings and opens the Account tab", async ({
  page,
}) => {
  await page.getByRole("button", { name: /remote server/i }).click();
  await page.getByRole("button", { name: /manage in account/i }).click();

  await expect(page).toHaveURL(/#\/account/);
  await expect(page.getByRole("heading", { name: /^account$/i })).toBeVisible();
});
