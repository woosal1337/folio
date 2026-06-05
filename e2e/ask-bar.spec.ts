import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("typing in the Home Ask bar opens Chat and asks the library", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");

  const input = page.getByPlaceholder(/ask anything across your notes/i);
  await input.fill("what did we decide about pricing?");
  await input.press("Enter");

  await expect(page).toHaveURL(/#\/chat/);
  await expect
    .poll(async () => (await ipcCalls(page, "ask_library")).length)
    .toBeGreaterThanOrEqual(1);
});

test('the "List recent todos" chip opens Chat and asks', async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");

  await page.getByRole("button", { name: /list recent todos/i }).click();
  await expect(page).toHaveURL(/#\/chat/);
  await expect
    .poll(async () => (await ipcCalls(page, "ask_library")).length)
    .toBeGreaterThanOrEqual(1);
});
