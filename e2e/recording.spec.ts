import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("Quick note creates a note and opens it (no capture)", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /quick note/i }).click();

  await expect.poll(async () => (await ipcCalls(page, "create_note")).length).toBe(1);

  await expect(page).toHaveURL(/#\/editor\//);
  expect((await ipcCalls(page, "start_recording")).length).toBe(0);
});

test('"Take notes" on the Coming-up card records into a fresh note', async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /take notes/i }).click();

  await expect.poll(async () => (await ipcCalls(page, "create_note")).length).toBe(1);
  await expect
    .poll(async () => (await ipcCalls(page, "start_recording")).length)
    .toBeGreaterThanOrEqual(1);
  await expect(page).toHaveURL(/#\/editor\//);
});

test("a fresh note shows a Draft name, not the timestamp", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /quick note/i }).click();
  await expect(page).toHaveURL(/#\/editor\//);

  await expect(page.getByRole("textbox", { name: /note title/i })).toHaveValue(
    "Draft 1"
  );
});

test("stopping a recording refreshes the note in place — never a stale empty page", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /take notes/i }).click();
  await expect(page).toHaveURL(/#\/editor\/2026-05-28-note/);
  await expect(page.getByRole("button", { name: /^stop$/i })).toBeVisible();

  await page.getByRole("button", { name: /^stop$/i }).click();

  await expect
    .poll(async () => (await ipcCalls(page, "stop_recording")).length)
    .toBe(1);
  await expect(page).toHaveURL(/#\/editor\/2026-05-28-note/);
  await expect(page.getByText(/no transcript yet/i)).toBeVisible({ timeout: 10_000 });
  await expect(page.getByRole("button", { name: /transcribe now/i })).toBeVisible();
});

test("editing the note title persists it", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /quick note/i }).click();
  await expect(page).toHaveURL(/#\/editor\//);

  const title = page.getByRole("textbox", { name: /note title/i });
  await title.fill("Strategy sync");
  await title.press("Enter");

  await expect.poll(async () => (await ipcCalls(page, "rename_note")).length).toBe(1);
  await expect(title).toHaveValue("Strategy sync");
});
