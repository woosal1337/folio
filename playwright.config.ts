/**
 * Playwright config — drives the React UI Tauri ships, in a real
 * Chromium against the Vite dev URL.
 *
 * We don't run inside the Tauri shell because `tauri-driver` isn't
 * supported on macOS yet (Tauri 2 open issue). Instead the test
 * harness injects a stub for `window.__TAURI_INTERNALS__.invoke`
 * via `page.addInitScript`, so every `@tauri-apps/api/core::invoke`
 * call lands on our fake instead of the real Rust runtime. This
 * covers the exact React render tree the user sees, including the
 * auth gate + conductor + Settings — what the Vitest unit tests
 * miss because they don't render in a real browser.
 *
 * Pair with `bun run e2e` (one shot) or `bun run e2e:headed` (watch
 * the run).
 */

import { defineConfig, devices } from "@playwright/test";

const PORT = Number(process.env.E2E_PORT ?? 5173);
const BASE_URL = process.env.E2E_BASE_URL ?? `http://localhost:${PORT}`;

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? "github" : "list",
  timeout: 60_000,
  expect: { timeout: 8_000 },

  use: {
    baseURL: BASE_URL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: process.env.CI ? "retain-on-failure" : "off",
    viewport: { width: 1280, height: 800 },
    locale: "en-US",
    timezoneId: "Europe/Istanbul",
  },

  webServer: {
    command: `bun run dev -- --host 127.0.0.1 --port ${PORT}`,
    url: BASE_URL,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
    stdout: "ignore",
    stderr: "pipe",
  },

  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        // Keep colour-scheme deterministic so visual regressions
        // don't trip on macOS appearance changes.
        colorScheme: "dark",
      },
    },
  ],
});
