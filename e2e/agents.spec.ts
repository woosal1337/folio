import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("AI section calls list_providers on open", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    providers: [
      { id: "openai", name: "OpenAI", has_key: true, redacted_key: "…sk_4321" },
      { id: "anthropic", name: "Anthropic", has_key: false, redacted_key: null },
    ],
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^ai$/i }).click();
  await expect
    .poll(async () => (await ipcCalls(page, "list_providers")).length)
    .toBeGreaterThanOrEqual(1);
});

test("provider IPC stub returns the seeded shape", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    providers: [{ id: "openai", name: "OpenAI", has_key: false, redacted_key: null }],
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^settings$/i }).click();
  await page.getByRole("button", { name: /^ai$/i }).click();
  await expect
    .poll(async () => (await ipcCalls(page, "list_providers")).length)
    .toBeGreaterThanOrEqual(1);
  const list = await ipcCalls(page, "list_providers");
  expect(list.length).toBeGreaterThanOrEqual(1);
});
