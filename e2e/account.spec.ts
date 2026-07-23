import { expect, test } from "@playwright/test";

import { readSettings, setupScenario } from "./fixtures/scenario";

test("Account tab in the sidebar opens the Account page", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();

  await page.getByRole("link", { name: /^account$/i }).click();
  await expect(page).toHaveURL(/#\/account/);
  await expect(page.getByRole("heading", { name: /^account$/i })).toBeVisible();
  await expect(page.getByText(/your folio server/i)).toBeVisible();
});

test("endpoint saves on blur and Test reports engine, model, and GPU", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/#/account");

  const endpoint = page.getByPlaceholder("https://folio-api.example.com");
  await endpoint.fill("https://folio-api.example.com");
  await endpoint.blur();
  await expect
    .poll(async () => (await readSettings(page)).remote_endpoint)
    .toBe("https://folio-api.example.com");

  await page.getByRole("button", { name: /^test$/i }).click();
  await expect(page.getByText(/connected to folio server/i)).toBeVisible();
  await expect(page.getByText(/large-v3/i)).toBeVisible();
  await expect(page.getByText(/^GPU$/)).toBeVisible();
});

test("creating an account flips to the signed-in state and back", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    initialSettings: { remote_endpoint: "https://folio-api.example.com" },
  });
  await page.goto("/#/account");

  await page.getByRole("tab", { name: /create account/i }).click();
  await page.getByPlaceholder("you@example.com").fill("me@example.com");
  await page.getByLabel("Password").fill("supersecret123");
  await page.getByRole("button", { name: /^create account$/i }).click();

  await expect(page.getByRole("main").getByText("you@example.com")).toBeVisible();
  await expect(page.getByText(/^connected$/i)).toBeVisible();

  await page.getByRole("button", { name: /sign out/i }).click();
  await expect(page.getByRole("tab", { name: /^sign in$/i })).toBeVisible();
});

test("sync preferences — auto-upload toggle and Make default persist", async ({
  page,
}) => {
  await setupScenario(page, {
    startSignedIn: true,
    initialSettings: { remote_endpoint: "https://folio-api.example.com" },
  });
  await page.goto("/#/account");

  await page.getByRole("switch", { name: /auto-upload/i }).click();
  await expect
    .poll(async () => (await readSettings(page)).remote_auto_upload)
    .toBe(true);

  await page.getByRole("button", { name: /make default/i }).click();
  await expect
    .poll(async () => (await readSettings(page)).transcriber)
    .toBe("remote_server");
  await expect(page.getByText(/^active$/i)).toBeVisible();
});

test("signed-in account shows in the sidebar with a status dot", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    initialSettings: { remote_endpoint: "https://folio-api.example.com" },
  });
  await page.goto("/#/account");

  await page.getByRole("tab", { name: /create account/i }).click();
  await page.getByPlaceholder("you@example.com").fill("me@example.com");
  await page.getByLabel("Password").fill("supersecret123");
  await page.getByRole("button", { name: /^create account$/i }).click();
  await expect(page.getByText(/^connected$/i)).toBeVisible();

  const accountLink = page.getByRole("link", { name: /^account$/i });
  await expect(accountLink).toContainText("you@example.com");
  await expect(accountLink.getByLabel(/signed in to your server/i)).toBeVisible();
});
