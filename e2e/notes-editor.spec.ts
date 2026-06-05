import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("typing markdown converts to styled formatting and autosaves", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page.getByRole("button", { name: /quick note/i }).click();
  await expect(page).toHaveURL(/#\/editor\//);

  const editor = page.locator(".ProseMirror").first();
  await editor.click();

  await page.keyboard.type("# Kickoff agenda");
  await expect(editor.locator("h1")).toHaveText("Kickoff agenda");

  await page.keyboard.press("Enter");
  await page.keyboard.type("- ship the editor");
  await expect(editor.locator("ul li")).toContainText("ship the editor");

  await expect
    .poll(async () => (await ipcCalls(page, "save_live_notes")).length)
    .toBeGreaterThanOrEqual(1);
});
