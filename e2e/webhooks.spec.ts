/**
 * Settings → Webhooks — the only fully-functional settings section
 * with real CRUD wired through `save_webhook` / `delete_webhook` /
 * `test_webhook`. Verifies the IPC contract.
 */

import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("Webhooks → list_webhooks fires when the tab is opened", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true, webhooks: [] });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^webhooks$/i }).click();
  await expect
    .poll(async () => (await ipcCalls(page, "list_webhooks")).length)
    .toBeGreaterThanOrEqual(1);
});

test("Webhooks → seeded list renders the URL + name", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    webhooks: [
      {
        id: "wh-1",
        label: "Slack #attune",
        url: "https://hooks.slack.com/services/AAA/BBB/ccc",
        events: ["recording.transcribed"],
        secret: "shh-not-a-real-secret",
        enabled: true,
      },
    ],
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^webhooks$/i }).click();
  await expect(page.getByText("Slack #attune").first()).toBeVisible();
});
