/**
 * Command palette (Cmd-K) + cheatsheet (Cmd-?) overlays.
 *
 * Both are global keyboard-shortcut surfaces wired in
 * `chrome/global-shortcuts.tsx` + `chrome/cheatsheet-overlay.tsx` +
 * `chrome/command-palette.tsx`. The shortcuts route through React's
 * `keydown` listeners — Playwright drives them with `page.keyboard.press`.
 */

import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test.beforeEach(async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
});

test("Cmd-K opens the command palette", async ({ page }) => {
  await page.keyboard.press("Meta+K");
  // Palette renders as a dialog with a search input.
  await expect(page.getByPlaceholder(/search|command/i).first()).toBeVisible();
});

test("Cmd-Shift-/ opens the cheatsheet overlay", async ({ page }) => {
  // The keymap binds Cmd-Shift-/ (a.k.a. Cmd-?) as the cheatsheet
  // trigger — see `openCheatsheet` in src/shared/lib/shortcuts.ts.
  await page.keyboard.press("Meta+Shift+/");
  await expect(page.getByText(/keyboard shortcuts|cheatsheet/i).first()).toBeVisible({
    timeout: 4000,
  });
});

test("Escape closes the command palette", async ({ page }) => {
  await page.keyboard.press("Meta+K");
  await expect(page.getByPlaceholder(/search|command/i).first()).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByPlaceholder(/search|command/i)).toHaveCount(0);
});
