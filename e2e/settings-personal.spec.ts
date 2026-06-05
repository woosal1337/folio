import { expect, test } from "@playwright/test";

import { ipcCalls, readSettings, setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.getByRole("button", { name: /^settings$/i }).click();
});

test("Preferences — toggling live meeting indicator persists on Save", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^preferences$/i }).click();
  const liveToggle = page.getByRole("switch", { name: /live meeting indicator/i });
  await liveToggle.click();
  await page.getByRole("button", { name: /^save$/i }).click();

  const saved = await readSettings(page);
  expect(saved.live_meeting_indicator).toBe(false);
  const calls = await ipcCalls(page, "save_settings");
  expect(calls.length).toBeGreaterThanOrEqual(1);
});

test("Preferences — auto-delete period default is 90 days (GDPR red line)", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^preferences$/i }).click();

  const select = page.getByLabel(/auto-delete transcripts/i);
  await expect(select).toHaveValue("90");
});

test("Profile — display name persists to the backend on blur (account_update)", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^profile$/i }).click();
  const nameInput = page.getByLabel(/display name/i);
  await nameInput.fill("Ege Çelebi (e2e)");

  await nameInput.blur();

  await expect
    .poll(async () => (await ipcCalls(page, "account_update")).length)
    .toBeGreaterThanOrEqual(1);
  const calls = await ipcCalls(page, "account_update");
  expect(calls[calls.length - 1].args).toMatchObject({
    displayName: "Ege Çelebi (e2e)",
  });
});

test("Profile — editing then re-opening keeps the saved display name", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^profile$/i }).click();
  const nameInput = page.getByLabel(/display name/i);
  await nameInput.fill("Persisted Name");
  await nameInput.blur();
  await expect
    .poll(async () => (await ipcCalls(page, "account_update")).length)
    .toBeGreaterThanOrEqual(1);

  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("button", { name: /^profile$/i }).click();
  await expect(page.getByLabel(/display name/i)).toHaveValue("Persisted Name");
});

test("Profile — email surfaces from the auth store identity", async ({ page }) => {
  await page.getByRole("button", { name: /^profile$/i }).click();
  await expect(page.getByText("ege@clinora.ai")).toBeVisible();
});

test("Calendar — toggling 'show upcoming meetings' updates settings", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^calendar$/i }).click();
  const toggle = page.getByRole("switch", {
    name: /show upcoming meetings in menu bar/i,
  });
  await toggle.click();
  await page.getByRole("button", { name: /^save$/i }).click();

  const saved = await readSettings(page);
  expect(saved.show_upcoming_meetings_in_menubar).toBe(false);
});

test("Notifications — toggling scheduled-meeting alerts persists", async ({ page }) => {
  await page.getByRole("button", { name: /^notifications$/i }).click();
  const toggle = page.getByRole("switch", { name: /scheduled meetings/i });
  await toggle.click();
  await page.getByRole("button", { name: /^save$/i }).click();

  const saved = await readSettings(page);
  expect(saved.notify_scheduled_meetings).toBe(false);
});

test("Notifications — app chips render their brand icons", async ({ page }) => {
  await page.getByRole("button", { name: /^notifications$/i }).click();

  await expect(
    page.getByRole("button", { name: /^chrome$/i }).locator("svg")
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: /^zoom$/i }).locator("svg")
  ).toBeVisible();
});
