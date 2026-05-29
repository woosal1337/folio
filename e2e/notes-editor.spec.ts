/**
 * Markdown notes editor. The "Your notes" pane is a live WYSIWYG markdown
 * editor: typing markdown syntax converts + styles it inline, and edits
 * autosave to the note via save_live_notes.
 */

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
  // The "# " input rule turns the line into a heading.
  await page.keyboard.type("# Kickoff agenda");
  await expect(editor.locator("h1")).toHaveText("Kickoff agenda");

  // A "- " input rule starts a bullet list on a fresh line.
  await page.keyboard.press("Enter");
  await page.keyboard.type("- ship the editor");
  await expect(editor.locator("ul li")).toContainText("ship the editor");

  // Edits autosave to the note.
  await expect
    .poll(async () => (await ipcCalls(page, "save_live_notes")).length)
    .toBeGreaterThanOrEqual(1);
});
