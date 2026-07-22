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

test("Remote server — selectable, endpoint test reports the GPU + model", async ({
  page,
}) => {
  const tile = page.getByRole("button", { name: /remote server/i });
  await tile.click();
  await expect(tile).toHaveAttribute("aria-pressed", "true");

  const endpoint = page.getByPlaceholder("https://folio-api.example.com");
  await expect(endpoint).toBeVisible();
  await endpoint.fill("https://folio-api.chele.bi");

  await page.getByRole("button", { name: /^test$/i }).click();
  await expect(page.getByText(/Connected to Folio Server/i)).toBeVisible();
  await expect(page.getByText(/large-v3/i)).toBeVisible();

  await expect(page.getByRole("button", { name: /sign in/i })).toBeVisible();
  await expect(page.getByRole("button", { name: /create account/i })).toBeVisible();
  await expect(page.getByRole("switch", { name: /auto-upload/i })).toBeVisible();

  await page.screenshot({ path: "e2e/remote-settings.png", fullPage: false });
});

test("Remote server — creating an account flips to the signed-in state", async ({
  page,
}) => {
  await page.getByRole("button", { name: /remote server/i }).click();
  await page
    .getByPlaceholder("https://folio-api.example.com")
    .fill("https://folio-api.chele.bi");
  await page.getByPlaceholder("you@example.com").fill("me@example.com");
  await page.getByPlaceholder("Password").fill("supersecret123");
  await page.getByRole("button", { name: /create account/i }).click();
  await expect(page.getByRole("button", { name: /sign out/i })).toBeVisible();
});
