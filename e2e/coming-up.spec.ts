import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("shows the next meeting when calendar access is granted", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  const startsAt = new Date(Date.now() + 12 * 60 * 1000).toISOString();
  const endsAt = new Date(Date.now() + 42 * 60 * 1000).toISOString();
  await page.addInitScript(
    ([s, e]) => {
      const w = window as unknown as Record<string, unknown>;
      w.__ATTUNE_CAL_ACCESS__ = "authorized";
      w.__ATTUNE_NEXT_EVENT__ = {
        id: "ev-1",
        title: "Pricing sync with Lila",
        starts_at: s,
        ends_at: e,
        attendees: ["lila@acme.com", "me@acme.com"],
      };
    },
    [startsAt, endsAt]
  );

  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await expect(page.getByText("Pricing sync with Lila")).toBeVisible();
  await expect(page.getByText(/in \d+ min/)).toBeVisible();
});

test("take notes on the coming-up card pre-names the note", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.addInitScript(() => {
    const w = window as unknown as Record<string, unknown>;
    w.__ATTUNE_CAL_ACCESS__ = "authorized";
    w.__ATTUNE_NEXT_EVENT__ = {
      id: "ev-2",
      title: "Design review",
      starts_at: new Date(Date.now() + 5 * 60 * 1000).toISOString(),
      ends_at: new Date(Date.now() + 35 * 60 * 1000).toISOString(),
      attendees: [],
    };
  });

  await page.goto("/");
  await page.getByRole("button", { name: /take notes/i }).click();

  await expect.poll(async () => (await ipcCalls(page, "create_note")).length).toBe(1);
  await expect
    .poll(async () => (await ipcCalls(page, "rename_note")).length)
    .toBeGreaterThanOrEqual(1);
  const renames = await ipcCalls(page, "rename_note");
  expect((renames[0]?.args as { title?: string })?.title).toBe("Design review");
});

test("offers to enable calendar when access is denied", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.addInitScript(() => {
    (window as unknown as Record<string, unknown>).__ATTUNE_CAL_ACCESS__ = "denied";
  });

  await page.goto("/");
  await expect(page.getByText(/calendar access is off/i)).toBeVisible();
  await expect(page.getByRole("button", { name: /enable calendar/i })).toBeVisible();
});
