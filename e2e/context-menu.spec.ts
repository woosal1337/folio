import { expect, test } from "@playwright/test";

import { setupScenario } from "./fixtures/scenario";

const RECORDING = {
  session_dir: "/tmp/Folio/2026-05-28-x",
  label: "2026-05-28-x",
  duration_seconds: 600,
  mic_bytes: 1_000_000,
  system_bytes: null,
  mic_sample_rate: 16_000,
  system_sample_rate: null,
  created_at: "2026-05-28T14:00:00Z",
  has_transcript: true,
  suggested_title: "Quarterly review",
  suggested_tags: [],
};

test("right-clicking a note row opens a quick-action menu", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/#/");
  const row = page.getByText("Quarterly review").first();
  await row.click({ button: "right" });

  const menu = page.getByRole("menu");
  await expect(menu).toBeVisible();
  await expect(menu.getByRole("menuitem", { name: /^open$/i })).toBeVisible();
  await expect(menu.getByRole("menuitem", { name: /delete note/i })).toBeVisible();
});

test("Delete from the context menu routes through the confirm dialog", async ({
  page,
}) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/#/");
  await page.getByText("Quarterly review").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: /delete note/i }).click();

  await expect(page.getByRole("dialog").getByText(/delete this note/i)).toBeVisible();
});

test("Escape closes the context menu", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/#/");
  await page.getByText("Quarterly review").first().click({ button: "right" });
  await expect(page.getByRole("menu")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toHaveCount(0);
});

test("right-click works on Home's recent notes too", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true, recordings: [RECORDING] });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  await page.getByText("Quarterly review").first().click({ button: "right" });
  const menu = page.getByRole("menu");
  await expect(menu.getByRole("menuitem", { name: /^open$/i })).toBeVisible();
  await expect(menu.getByRole("menuitem", { name: /delete note/i })).toBeVisible();
});
