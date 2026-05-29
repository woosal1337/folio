/**
 * Settings → Personal section coverage (Preferences, Profile, Calendar,
 * Notifications). Every test opens Settings, navigates to a tab,
 * toggles or edits something, then verifies that:
 *   1. The UI reflects the change immediately.
 *   2. Clicking Save calls `save_settings` with the patched value.
 */

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
  // The auto-delete <select> is paired with its title via
  // <Label htmlFor={id}>Auto-delete transcripts</Label>, so
  // accessibility name lookup finds it.
  const select = page.getByLabel(/auto-delete transcripts/i);
  await expect(select).toHaveValue("90");
});

test("Profile — display name persists to the backend on blur (account_update)", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^profile$/i }).click();
  const nameInput = page.getByLabel(/display name/i);
  await nameInput.fill("Ege Çelebi (e2e)");
  // Commit on blur — the field now PATCHes /account rather than riding on
  // save_settings (the display name was never part of the Settings struct).
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
  // Switch sections and back — the name should still be there (it was
  // committed to the store, not lost on remount, which was the bug).
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
  // Each muteable app chip renders its brand icon (an inline SVG from the
  // icon library). Spot-check a couple, including an app that isn't
  // installed locally (Zoom) — the icon is library-sourced, not local.
  await expect(
    page.getByRole("button", { name: /^chrome$/i }).locator("svg")
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: /^zoom$/i }).locator("svg")
  ).toBeVisible();
});
