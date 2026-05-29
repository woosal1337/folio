/**
 * Chat history + Recents (GET-167). A cross-library conversation is
 * persisted; after navigating away and back it appears in Recents and
 * reopens with its history.
 */

import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test("a conversation persists to Recents and reopens with history", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/#/chat");

  // Ask something; the mocked ask_library answers and the thread saves.
  const input = page.getByRole("textbox", { name: /ask across your library/i });
  await input.fill("what are my open todos");
  await input.press("Enter");
  await expect(page.getByText("No open action items found.")).toBeVisible();

  // Leave to Home, come back to Chat.
  await page.goto("/#/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.goto("/#/chat");

  // The conversation is in Recents…
  await page.getByRole("button", { name: /^recents$/i }).click();
  const recent = page.getByRole("menu").getByText("what are my open todos");
  await expect(recent).toBeVisible();

  // …and reopens with its history (the prior answer is back on screen).
  await recent.click();
  await expect(page.getByText("No open action items found.")).toBeVisible();
});

test("deleting a conversation removes it from Recents", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/#/chat");
  const input = page.getByRole("textbox", { name: /ask across your library/i });
  await input.fill("delete me later");
  await input.press("Enter");
  await expect(page.getByText("No open action items found.")).toBeVisible();

  await page.getByRole("button", { name: /^recents$/i }).click();
  await expect(page.getByRole("menu").getByText("delete me later")).toBeVisible();
  await page
    .getByRole("button", { name: /delete conversation delete me later/i })
    .click();
  await expect(page.getByRole("menu").getByText("delete me later")).toHaveCount(0);
});
