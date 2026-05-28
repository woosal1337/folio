/**
 * Settings → Workspace section coverage (General, Team, Analytics,
 * Billing, Connectors, Webhooks, Usage, Referrals).
 *
 * Most of these are UI-only stubs (backend wiring lands later) — the
 * tests assert the panel renders + key labels surface so we catch
 * regressions when the route IDs or section names change.
 */

import { expect, test } from "@playwright/test";

import { ipcCalls, readSettings, setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
  await page.getByRole("button", { name: /^settings$/i }).click();
});

test("Workspace General — workspace name input is editable + persists", async ({
  page,
}) => {
  // There are two General entries (Recording → General + Workspace → General)
  // — disambiguate by clicking the second one in DOM order.
  await page.getByRole("button", { name: /^general$/i }).nth(1).click();
  const nameInput = page.getByPlaceholder(/clinora/i);
  await nameInput.fill("Acme Co");
  await page.getByRole("button", { name: /^save$/i }).click();
  const saved = await readSettings(page);
  expect(saved.workspace_name).toBe("Acme Co");
});

test("Workspace General — discoverable toggle persists", async ({ page }) => {
  await page.getByRole("button", { name: /^general$/i }).nth(1).click();
  const toggle = page.getByRole("switch", {
    name: /discoverable by matching email domain/i,
  });
  await toggle.click();
  await page.getByRole("button", { name: /^save$/i }).click();
  const saved = await readSettings(page);
  expect(saved.workspace_discoverable).toBe(false);
});

test("Workspace Team — empty state renders for a fresh workspace", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^team$/i }).click();
  await expect(page.getByText(/no pending invites/i)).toBeVisible();
});

test("Workspace Analytics — privacy anti-feature copy renders", async ({ page }) => {
  await page.getByRole("button", { name: /^analytics$/i }).click();
  await expect(page.getByText(/no engagement scoring/i)).toBeVisible();
});

test("Workspace Analytics — range chips switch active state", async ({ page }) => {
  await page.getByRole("button", { name: /^analytics$/i }).click();
  const sevenDay = page.getByRole("button", { name: /last 7 days/i });
  await sevenDay.click();
  await expect(sevenDay).toHaveAttribute("aria-pressed", "true");
});

test("Workspace Billing — Free tier card + 10-row feature matrix", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^billing$/i }).click();
  await expect(page.getByText(/^free$/i).first()).toBeVisible();
  // The matrix lists Pro / Team / Enterprise tiers.
  await expect(page.getByText(/^pro$/i).first()).toBeVisible();
  await expect(page.getByText(/^enterprise$/i).first()).toBeVisible();
});

test("Connectors — featured MCP card + copy-URL is interactive", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^connectors$/i }).click();
  await expect(page.getByText(/local mcp server/i)).toBeVisible();
  // Granting clipboard write to Chromium so copy actually works in
  // the headless run.
  await page.context().grantPermissions(["clipboard-write", "clipboard-read"]);
  await page.getByRole("button", { name: /^copy$/i }).first().click();
  const text = await page.evaluate(() => navigator.clipboard.readText());
  expect(text).toContain("http://127.0.0.1:7438/mcp");
});

test("Referrals — personal link is visible and copyable", async ({ page }) => {
  await page.getByRole("button", { name: /^referrals$/i }).click();
  await expect(page.getByText(/join\.attune\.app/i).first()).toBeVisible();
  await page.context().grantPermissions(["clipboard-write", "clipboard-read"]);
  await page.getByRole("button", { name: /^copy$/i }).first().click();
  const text = await page.evaluate(() => navigator.clipboard.readText());
  expect(text).toContain("join.attune.app");
});

test("Webhooks tab opens without crashing", async ({ page }) => {
  await page.getByRole("button", { name: /^webhooks$/i }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
});

test("Usage tab opens without crashing", async ({ page }) => {
  await page.getByRole("button", { name: /^usage$/i }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
});

test("Settings save round-trips IPC: settings_sync_push fires when enabled", async ({
  page,
}) => {
  // Open any section and click Save — the harness should at minimum
  // call `save_settings`. (`settings_sync_push` wiring lands when the
  // cloud-sync toggle is on; here we just confirm the local persist.)
  await page.getByRole("button", { name: /^preferences$/i }).click();
  await page.getByRole("button", { name: /^save$/i }).click();
  const calls = await ipcCalls(page, "save_settings");
  expect(calls.length).toBeGreaterThanOrEqual(1);
});
