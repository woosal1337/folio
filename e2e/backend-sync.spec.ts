import { expect, test } from "@playwright/test";

import { ipcCalls, ipcLog, setupScenario } from "./fixtures/scenario";

test("save_settings is fired on the global Save button", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("button", { name: /^save$/i }).click();
  const calls = await ipcCalls(page, "save_settings");
  expect(calls.length).toBeGreaterThanOrEqual(1);
});

test("save_settings carries the patched payload, not an arbitrary blob", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page
    .getByRole("button", { name: /^general$/i })
    .first()
    .click();

  const toggle = page.getByRole("switch", { name: /capture system audio/i });
  await toggle.click();
  await page.getByRole("button", { name: /^save$/i }).click();

  const calls = await ipcCalls(page, "save_settings");
  expect(calls.length).toBeGreaterThanOrEqual(1);
  const last = calls.at(-1)!.args as { settings: { system_audio_enabled: boolean } };
  expect(last.settings.system_audio_enabled).toBe(false);
});

test("auth_logout clears identity + flips the gate back to signup", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^profile$/i }).click();
  await page
    .getByRole("button", { name: /^sign out$/i })
    .first()
    .click();

  const logoutCalls = await ipcCalls(page, "auth_logout");
  expect(logoutCalls).toHaveLength(1);
  await expect(page.getByRole("heading", { name: /welcome to folio/i })).toBeVisible();
});

test("the IPC trail contains the boot-time probes (auth_status, get_settings)", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();

  const log = await ipcLog(page);
  const cmds = log.map((e) => e.cmd);
  expect(cmds).toContain("auth_status");
  expect(cmds).toContain("get_settings");
});
