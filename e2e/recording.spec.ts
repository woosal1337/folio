/**
 * Note-first recording (GET-155). There is no Record screen: a note is
 * created and opened, and capture attaches to it from the in-note dock.
 * Drives the real `useTakeNotes` / `useQuickNote` hooks + recording
 * store against the mocked `create_note` / `start_recording` IPC.
 */

import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("Quick note creates a note and opens it (no capture)", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /quick note/i }).click();

  await expect.poll(async () => (await ipcCalls(page, "create_note")).length).toBe(1);
  // Lands in the editor; no recording was started.
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
  // The editable title shows the Draft placeholder, not "2026-05-28-note".
  await expect(page.getByRole("textbox", { name: /note title/i })).toHaveValue(
    "Draft 1"
  );
});

test("editing the note title persists it (GET-163)", async ({ page }) => {
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
