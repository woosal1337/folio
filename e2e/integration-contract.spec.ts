import { expect, test } from "@playwright/test";

import { ipcLog, setupScenario } from "./fixtures/scenario";

const BOOT_COMMANDS = ["auth_status", "get_settings"];

test("the React app emits the documented boot probes on cold load", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  const log = await ipcLog(page);
  const cmds = new Set(log.map((e) => e.cmd));
  for (const expected of BOOT_COMMANDS) {
    expect(cmds.has(expected), `missing boot IPC: ${expected}`).toBe(true);
  }
});

test("recording_status probe fires on app mount (re-adopts in-progress capture)", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await expect
    .poll(async () => (await ipcLog(page)).map((e) => e.cmd))
    .toContain("recording_status");
});

test("list_recordings fires on Home mount (recent notes)", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await expect
    .poll(async () => (await ipcLog(page)).map((e) => e.cmd))
    .toContain("list_recordings");
});

test("save_settings is the canonical persist call (not settings_sync_push)", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page
    .getByRole("button", { name: /^save$/i })
    .last()
    .click();
  const log = await ipcLog(page);
  const save = log.filter((e) => e.cmd === "save_settings");
  expect(save.length).toBeGreaterThanOrEqual(1);

  const cloudPush = log.filter((e) => e.cmd === "settings_sync_push");
  expect(cloudPush.length).toBe(0);
});

test("auth_logout fires exactly once per sign-out click", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^profile$/i }).click();
  await page
    .getByRole("button", { name: /^sign out$/i })
    .first()
    .click();
  const log = await ipcLog(page);
  expect(log.filter((e) => e.cmd === "auth_logout")).toHaveLength(1);
});
