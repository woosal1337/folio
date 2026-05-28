/**
 * Onboarding & auth-gate scenarios.
 *
 * Drives the full conductor walk-through through a real Chromium
 * against the Vite dev URL, with the Tauri IPC bridge stubbed via
 * `window.__TAURI_INTERNALS__.invoke`.
 */

import { expect, test } from "@playwright/test";

import { ipcCalls, readSettings, setupScenario } from "./fixtures/scenario";

test.describe("Onboarding — fresh signup", () => {
  test("walks permissions → signup → OTP → workspace setup → main app", async ({
    page,
  }) => {
    await setupScenario(page, {
      initialSettings: {
        onboarding_completed: false,
        signin_mode: "",
        workspace_name: "",
        workspace_bucket: "",
      },
      startSignedIn: false,
    });
    await page.goto("/");

    // 1. Permissions screen renders first.
    await expect(
      page.getByRole("heading", { name: /allow attune to transcribe/i }),
    ).toBeVisible();
    await page.getByRole("button", { name: /^continue$/i }).click();

    // 2. Signup.
    await expect(page.getByRole("heading", { name: /welcome to attune/i })).toBeVisible();
    await page.getByPlaceholder(/you@company\.com/i).fill("ege@clinora.ai");
    await page.getByRole("button", { name: /^continue$/i }).first().click();

    // 3. Code entry.
    await expect(
      page.getByRole("heading", { name: /check your email/i }),
    ).toBeVisible();
    await page.locator('input[id="code-0"]').fill("123456");
    await page.getByRole("button", { name: /verify and continue/i }).click();

    // 4. EventKit rationale (PR #215 fix — must appear post-signup).
    await expect(
      page.getByRole("heading", { name: /read your mac.s calendar locally/i }),
    ).toBeVisible();
    await page.getByRole("button", { name: /skip for now/i }).click();

    // 5. Workspace name — auto-populated from the email domain.
    await expect(
      page.getByRole("heading", { name: /name your workspace/i }),
    ).toBeVisible();
    await expect(page.getByLabel(/workspace name/i)).toHaveValue("Clinora");
    await page.getByRole("button", { name: /^continue$/i }).click();

    // 6. Bucket — calendar deferred so this routes to transcriber.
    await expect(
      page.getByRole("heading", { name: /what do you do\?/i }),
    ).toBeVisible();
    await page.getByRole("radio", { name: /founder/i }).click();
    await page.getByRole("button", { name: /^continue$/i }).click();

    // 7. Transcriber.
    await expect(
      page.getByRole("heading", { name: /welcome to attune/i }),
    ).toBeVisible();
    await page.getByRole("button", { name: /i.?m ready/i }).click();

    // 8. Main app renders with the sidebar.
    await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
    await expect(page.getByRole("navigation").getByText(/inbox/i)).toBeVisible();

    // 9. Backend received the right IPC calls.
    const signupCalls = await ipcCalls(page, "auth_request_signin_code");
    expect(signupCalls).toHaveLength(1);
    const verify = await ipcCalls(page, "auth_verify_signin_code");
    expect(verify).toHaveLength(1);

    // 10. The persisted settings reflect the workspace setup.
    const saved = await readSettings(page);
    expect(saved.onboarding_completed).toBe(true);
    expect(saved.signin_mode).toBe("email");
    expect(saved.workspace_name).toBe("Clinora");
    expect(saved.workspace_bucket).toBe("founder");
    expect(saved.transcriber).toBe("local_whisper");
  });

  test("hides the sidebar entirely when signed out", async ({ page }) => {
    await setupScenario(page, {
      initialSettings: { onboarding_completed: false },
      startSignedIn: false,
    });
    await page.goto("/");
    await expect(
      page.getByRole("heading", { name: /allow attune to transcribe/i }),
    ).toBeVisible();
    await expect(page.getByRole("navigation")).toHaveCount(0);
  });
});

test.describe("Onboarding — returning user", () => {
  test("signed-out + onboarded → signup → OTP → main app (no workspace setup)", async ({
    page,
  }) => {
    await setupScenario(page, {
      initialSettings: { onboarding_completed: true },
      startSignedIn: false,
    });
    await page.goto("/");

    await expect(page.getByRole("heading", { name: /welcome to attune/i })).toBeVisible();
    await page.getByPlaceholder(/you@company\.com/i).fill("ege@clinora.ai");
    await page.getByRole("button", { name: /^continue$/i }).first().click();
    await expect(
      page.getByRole("heading", { name: /check your email/i }),
    ).toBeVisible();
    await page.locator('input[id="code-0"]').fill("987654");
    await page.getByRole("button", { name: /verify and continue/i }).click();

    // Straight to main app — no EventKit / workspace screens.
    await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
  });
});
