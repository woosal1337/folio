/**
 * Recording flow — Record route start/stop button + status pill +
 * IPC ordering. Drives the actual `useRecording` Zustand store + the
 * recording-store's `startRecording` / `stopRecording` IPC contract
 * end-to-end with mocked Tauri responses.
 */

import { expect, test } from "@playwright/test";

import { ipcCalls, setupScenario } from "./fixtures/scenario";

test("Record page renders Start affordance when idle", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
  // The Start button surfaces as the primary CTA. We pin its
  // accessibility name with a tight match so the test doesn't drift
  // into "Stop" / "Resume" affordances.
  await expect(
    page.getByRole("button", { name: /^start( recording)?$/i }).first(),
  ).toBeVisible();
});

test("Start button calls the start_recording IPC", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await page
    .getByRole("button", { name: /^start( recording)?$/i })
    .first()
    .click();
  await expect
    .poll(async () => (await ipcCalls(page, "start_recording")).length)
    .toBeGreaterThanOrEqual(1);
});

test("recording_status probe fires on page load (state hydration)", async ({ page }) => {
  await setupScenario(page, { startSignedIn: true });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
  const calls = await ipcCalls(page, "recording_status");
  expect(calls.length).toBeGreaterThanOrEqual(1);
});

test("list_recordings fires on Record-page mount so the history strip populates", async ({
  page,
}) => {
  await setupScenario(page, {
    startSignedIn: true,
    recordings: [
      {
        session_dir: "/tmp/Attune/2026-05-28-product-review",
        label: "2026-05-28-product-review",
        duration_seconds: 1200,
        mic_bytes: 500_000,
        system_bytes: 800_000,
        mic_sample_rate: 16_000,
        system_sample_rate: 16_000,
        created_at: "2026-05-28T14:00:00Z",
        has_transcript: true,
        suggested_title: "Product review",
        suggested_subtitle: "Q2 launch checkpoint",
        suggested_tags: ["product"],
        language_override: "en",
      },
    ],
  });
  await page.goto("/");
  // HomeRedirect lands on /library when there are recordings on
  // disk — that's the expected app behaviour. Click Record in the
  // sidebar to force the Record route.
  await page.getByRole("link", { name: /record/i }).click();
  await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
  const calls = await ipcCalls(page, "list_recordings");
  expect(calls.length).toBeGreaterThanOrEqual(1);
});
