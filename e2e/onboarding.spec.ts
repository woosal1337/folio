import { expect, test } from "@playwright/test";

import { ipcCalls, readSettings, setupScenario } from "./fixtures/scenario";

test.describe("Onboarding — fresh signup", () => {
  test("walks permissions → signup → OTP → workspace setup → main app", async ({
    page,
  }) => {
    await setupScenario(page, {
      initialSettings: {
        onboarding_completed: false,
      },
      startSignedIn: false,
    });
    await page.goto("/");

    await expect(
      page.getByRole("heading", { name: /allow folio to transcribe/i })
    ).toBeVisible();
    await page.getByRole("button", { name: /^continue$/i }).click();

    await expect(
      page.getByRole("heading", { name: /welcome to folio/i })
    ).toBeVisible();
    await page.getByPlaceholder(/you@company\.com/i).fill("ege@clinora.ai");
    await page
      .getByRole("button", { name: /^continue$/i })
      .first()
      .click();

    await expect(
      page.getByRole("heading", { name: /check your email/i })
    ).toBeVisible();
    await page.locator('input[id="code-0"]').fill("123456");
    await page.getByRole("button", { name: /verify and continue/i }).click();

    await expect(
      page.getByRole("heading", { name: /read your mac.s calendar locally/i })
    ).toBeVisible();
    await page.getByRole("button", { name: /skip for now/i }).click();

    await expect(
      page.getByRole("heading", { name: /name your workspace/i })
    ).toBeVisible();
    await expect(page.getByLabel(/workspace name/i)).toHaveValue("Clinora");
    await page.getByRole("button", { name: /^continue$/i }).click();

    await expect(
      page.getByRole("heading", { name: /what do you do\?/i })
    ).toBeVisible();
    await page.getByRole("radio", { name: /founder/i }).click();
    await page.getByRole("button", { name: /^continue$/i }).click();

    await expect(
      page.getByRole("heading", { name: /welcome to folio/i })
    ).toBeVisible();
    await page.getByRole("button", { name: /i.?m ready/i }).click();

    await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
    await expect(page.getByRole("navigation").getByText(/^home$/i)).toBeVisible();

    const signupCalls = await ipcCalls(page, "auth_request_signin_code");
    expect(signupCalls).toHaveLength(1);
    const verify = await ipcCalls(page, "auth_verify_signin_code");
    expect(verify).toHaveLength(1);

    const saved = await readSettings(page);
    expect(saved.onboarding_completed).toBe(true);
    expect(saved.transcriber).toBe("local_whisper");
  });

  test("hides the sidebar entirely when signed out", async ({ page }) => {
    await setupScenario(page, {
      initialSettings: { onboarding_completed: false },
      startSignedIn: false,
    });
    await page.goto("/");
    await expect(
      page.getByRole("heading", { name: /allow folio to transcribe/i })
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

    await expect(
      page.getByRole("heading", { name: /welcome to folio/i })
    ).toBeVisible();
    await page.getByPlaceholder(/you@company\.com/i).fill("ege@clinora.ai");
    await page
      .getByRole("button", { name: /^continue$/i })
      .first()
      .click();
    await expect(
      page.getByRole("heading", { name: /check your email/i })
    ).toBeVisible();
    await page.locator('input[id="code-0"]').fill("987654");
    await page.getByRole("button", { name: /verify and continue/i }).click();

    await expect(page.getByRole("heading", { name: /^home$/i })).toBeVisible();
  });
});
