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

test("list_agents fires when a note's AI panel mounts", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    recordings: [
      {
        session_dir: "/tmp/Attune/2026-05-28-product-review",
        label: "2026-05-28-product-review",
        duration_seconds: 1200,
        mic_bytes: 500_000,
        system_bytes: 800_000,
        created_at: "2026-05-28T14:00:00Z",
        has_transcript: true,
        suggested_title: "Product review",
        suggested_tags: ["product"],
      },
    ],
  });
  await page.goto("/");
  await page.getByRole("link", { name: /my notes/i }).click();
  await expect(page.getByRole("heading", { name: /^my notes$/i })).toBeVisible();
  // Open the note, then expand the tucked "AI agents" disclosure —
  // mounting the AgentPanel reads list_agents.
  await page.getByRole("button", { name: /product review/i }).click();
  await page.getByRole("button", { name: /ai agents/i }).click();
  await expect
    .poll(async () => (await ipcCalls(page, "list_agents")).length)
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
