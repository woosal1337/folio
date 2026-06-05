import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("Library — recording row renders from list_recordings IPC", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    recordings: [
      {
        session_dir: "/tmp/Folio/2026-05-28-product-review",

        label: "2026-05-28-product-review",
        duration_seconds: 1800,
        mic_bytes: 1_000_000,
        system_bytes: 2_000_000,
        mic_sample_rate: 16_000,
        system_sample_rate: 16_000,
        created_at: "2026-05-28T14:00:00Z",
        has_transcript: true,
        suggested_title: "Product review",
        suggested_subtitle: "Q2 launch checkpoint",
        suggested_tags: ["product", "launch"],
        language_override: "en",
      },
    ],
  });
  await page.goto("/");
  await page.getByRole("link", { name: /my notes/i }).click();
  await expect(page.getByRole("heading", { name: /^my notes$/i })).toBeVisible();
  await expect(page.getByText("Product review").first()).toBeVisible();
});

test("Tasks — empty state, add a task via the UI, see it appear", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true, tasks: [] });
  await page.goto("/");
  await page.getByRole("link", { name: /tasks/i }).click();
  await expect(page.getByRole("heading", { name: /^tasks$/i })).toBeVisible();

  await page
    .getByRole("button", { name: /^add task$/i })
    .first()
    .click();
  const input = page.getByPlaceholder(/what needs doing/i);
  await input.fill("Confirm the email domain at example.com");
  await input.press("Enter");

  await expect
    .poll(async () => (await ipcCalls(page, "create_task")).length)
    .toBeGreaterThanOrEqual(1);
  const calls = await ipcCalls(page, "create_task");
  const payload = calls[0]!.args as { task: { title: string } };
  expect(payload.task.title).toContain("email domain");
});

test("Memory — seeded entries render in the list", async ({ page }) => {
  await setupScenario(page, {
    startSignedIn: true,
    memories: [
      {
        id: "0192f9d4-0000-7000-8000-000000000001",
        kind: "observe",
        key: null,
        content: "Founder prefers OTP email over OAuth for v1.",
        evidence: null,
        confidence: 1.0,
        tags: ["auth", "decision"],
        source_session_dir: null,
        source_session_label: null,
        valid_from: "2026-05-28T10:00:00Z",
        valid_until: null,
        supersedes_id: null,
        pinned: true,
        created_at: "2026-05-28T10:00:00Z",
        updated_at: "2026-05-28T10:00:00Z",
      },
    ],
  });
  await page.goto("/");
  await page.getByRole("link", { name: /memory/i }).click();
  await expect(page.getByRole("heading", { name: /^memory$/i })).toBeVisible();
  await expect(page.getByText(/founder prefers otp/i)).toBeVisible();
});
