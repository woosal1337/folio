import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

test("a conversation persists to Recents and reopens with history", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/#/chat");

  const input = page.getByRole("textbox", { name: /ask across your library/i });
  await input.fill("what are my open todos");
  await input.press("Enter");
  await expect(page.getByText("No open action items found.")).toBeVisible();

  await page.goto("/#/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.goto("/#/chat");

  await page.getByRole("button", { name: /^recents$/i }).click();
  const recent = page.getByRole("menu").getByText("what are my open todos");
  await expect(recent).toBeVisible();

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

  const dialog = page.getByRole("dialog");
  await expect(dialog.getByText(/delete this conversation/i)).toBeVisible();
  await dialog.getByRole("button", { name: /delete conversation/i }).click();
  await expect(page.getByText("delete me later")).toHaveCount(0);
});
