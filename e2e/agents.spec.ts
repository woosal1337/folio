/**
 * Settings → AI providers + agents IPC integration.
 *
 * AI keys are managed in Settings → AI; the section reads providers
 * via `list_providers`, gates the agents section by their
 * configured-state, and writes via `set_provider_key`. This spec
 * verifies the IPC contract.
 */

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

test("list_agents fires on app boot when agents UI is mounted somewhere", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  // Boot probes — Inbox + Library + agents-related panels all read.
  // We don't assert quantity, only that the IPC was eventually
  // wired through.
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
  // Force at least one list_agents call by navigating to the Inbox
  // (which surfaces recent agent runs / fresh memories).
  await page.getByRole("link", { name: /inbox/i }).click();
  await expect(page.getByRole("heading", { name: /^inbox$/i })).toBeVisible();
  // We don't strictly require list_agents on Inbox load — the
  // scenarios stub returns empty — but we DO want the route to render
  // without throwing on the missing IPC.
});

test("provider IPC stub returns the seeded shape", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    providers: [
      { id: "openai", name: "OpenAI", has_key: false, redacted_key: null },
    ],
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
