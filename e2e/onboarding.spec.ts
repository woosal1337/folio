/**
 * End-to-end test for the Attune onboarding flow.
 *
 * Drives the real React UI (the same code Tauri ships) through a
 * Chromium browser pointed at the Vite dev server. The Tauri IPC
 * bridge is stubbed via `window.__TAURI_INTERNALS__.invoke`, so
 * every backend call (settings load, OTP request/verify, attendee
 * suggestions, etc.) lands on an in-page handler we control. This
 * gives us the closest thing to running the real binary without
 * needing `tauri-driver` (which isn't supported on macOS yet).
 *
 * Scenarios covered:
 *   1. Fresh install → permissions → signup (email) → code-entry
 *      (paste 6 digits) → eventkit rationale → workspace name
 *      (auto-populated from email) → workspace bucket → transcriber
 *      → "I'm ready" → main app renders with the sidebar visible.
 *   2. Signed-out + onboarding_completed=true returning user → lands
 *      on signup, verifies OTP, goes straight to the main app
 *      (no workspace setup repeat).
 *   3. Signed-out gate hides the sidebar; signing in shows it.
 */

import { expect, test } from "@playwright/test";

import { installTauriStub, ipcLog } from "./fixtures/tauri-ipc";

interface MockSettings {
  mic_device: string | null;
  system_audio_enabled: boolean;
  output_dir: string;
  notes_dir: string;
  tasks_path: string;
  transcripts_dir: string;
  theme: string;
  transcriber: string;
  transcription_language: string;
  briefing_language: string;
  dictionary_terms: string[];
  local_whisper_model: string;
  voice_processing_enabled: boolean;
  auto_transcribe_enabled: boolean;
  auto_vad_enabled: boolean;
  memory_dir: string;
  auto_extract_memories_enabled: boolean;
  feedback_sounds_enabled: boolean;
  auto_summarize_enabled: boolean;
  auto_extract_tasks_enabled: boolean;
  auto_name_enabled: boolean;
  wav_retention_days: number | null;
  share_aggregate_stats: boolean;
  pro_license_key: string;
  pro_trial_started_at: string;
  voice_debrief_enabled: boolean;
  privacy_mode: boolean;
  onboarding_completed: boolean;
  show_upcoming_meetings_in_menubar: boolean;
  show_events_without_participants: boolean;
  live_meeting_indicator: boolean;
  open_at_login: boolean;
  move_aside_in_meetings: boolean;
  default_link_sharing: string;
  always_open_shared_links: boolean;
  privacy_tier_band_enabled: boolean;
  auto_delete_period_days: number | null;
  notify_scheduled_meetings: boolean;
  notify_auto_detected_meetings: boolean;
  notification_muted_apps: string[];
  note_shared_notification: string;
  signin_mode: string;
  workspace_name: string;
  workspace_bucket: string;
  onboarding_calendar_deferred: boolean;
  workspace_discoverable: boolean;
  workspace_auto_join: boolean;
  workspace_logo_path: string;
}

function freshSettings(overrides: Partial<MockSettings> = {}): MockSettings {
  return {
    mic_device: null,
    system_audio_enabled: true,
    output_dir: "/tmp/Attune",
    notes_dir: "/tmp/Attune/Notes",
    tasks_path: "/tmp/Attune/Tasks.json",
    transcripts_dir: "/tmp/Attune/Transcripts",
    theme: "dark",
    transcriber: "local_whisper",
    transcription_language: "auto",
    briefing_language: "en",
    dictionary_terms: [],
    local_whisper_model: "large-v3",
    voice_processing_enabled: true,
    auto_transcribe_enabled: true,
    auto_vad_enabled: true,
    memory_dir: "/tmp/Attune/Memory",
    auto_extract_memories_enabled: false,
    feedback_sounds_enabled: false,
    auto_summarize_enabled: false,
    auto_extract_tasks_enabled: false,
    auto_name_enabled: false,
    wav_retention_days: null,
    share_aggregate_stats: false,
    pro_license_key: "",
    pro_trial_started_at: "",
    voice_debrief_enabled: false,
    privacy_mode: false,
    onboarding_completed: false,
    show_upcoming_meetings_in_menubar: true,
    show_events_without_participants: false,
    live_meeting_indicator: true,
    open_at_login: false,
    move_aside_in_meetings: false,
    default_link_sharing: "workspace_only",
    always_open_shared_links: true,
    privacy_tier_band_enabled: false,
    auto_delete_period_days: 90,
    notify_scheduled_meetings: true,
    notify_auto_detected_meetings: true,
    notification_muted_apps: [],
    note_shared_notification: "activity_and_email",
    signin_mode: "",
    workspace_name: "",
    workspace_bucket: "",
    onboarding_calendar_deferred: false,
    workspace_discoverable: true,
    workspace_auto_join: true,
    workspace_logo_path: "",
    ...overrides,
  };
}

interface ScenarioOptions {
  initialSettings?: Partial<MockSettings>;
  startSignedIn?: boolean;
}

async function setupScenario(
  page: import("@playwright/test").Page,
  options: ScenarioOptions = {},
) {
  const baseSettings = freshSettings(options.initialSettings ?? {});
  const startSignedIn = options.startSignedIn ?? false;

  // Each handler is stringified and shipped into the page; closures
  // can't capture `baseSettings` directly. We embed the seed as a
  // window-scoped global the handlers can read + mutate.
  await page.addInitScript(
    ([seed, signedIn]) => {
      (window as unknown as Record<string, unknown>).__ATTUNE_SETTINGS__ = JSON.parse(
        seed as string,
      );
      (window as unknown as Record<string, unknown>).__ATTUNE_SIGNED_IN__ =
        signedIn as boolean;
      (window as unknown as Record<string, unknown>).__ATTUNE_INPUT_DEVICES__ = [
        { id: "default", name: "MacBook Pro Microphone" },
      ];
    },
    [JSON.stringify(baseSettings), startSignedIn] as const,
  );

  await installTauriStub(page, {
    passthroughUnknown: true,
    handlers: {
      get_settings: () => {
        return (window as unknown as Record<string, unknown>).__ATTUNE_SETTINGS__;
      },
      save_settings: (args) => {
        const a = args as { settings: unknown };
        (window as unknown as Record<string, unknown>).__ATTUNE_SETTINGS__ =
          a.settings;
        return null;
      },
      list_permissions: () => [
        {
          permission: "microphone",
          status: "granted",
          rationale: "We record what you say.",
          settings_url: "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
        },
        {
          permission: "screen_recording",
          status: "granted",
          rationale: "We record what the other side says.",
          settings_url: "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
        },
        {
          permission: "calendar",
          status: "unknown",
          rationale: "Pre-fills meeting titles.",
          settings_url: "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars",
        },
        {
          permission: "notifications",
          status: "unknown",
          rationale: "Used only for 'recording started' alerts.",
          settings_url: "x-apple.systempreferences:com.apple.preference.notifications",
        },
      ],
      open_permission_settings: () => null,
      request_calendar_access: () => null,
      list_input_devices: () => {
        return (window as unknown as Record<string, unknown>).__ATTUNE_INPUT_DEVICES__;
      },
      list_attendee_suggestions: () => [],
      ping: () => "pong",
      auth_status: () => {
        if ((window as unknown as Record<string, unknown>).__ATTUNE_SIGNED_IN__) {
          return {
            signed_in: true,
            identity: {
              user_id: "user-1",
              email: "ege@clinora.ai",
              display_name: null,
              privacy_tier: null,
            },
          };
        }
        return { signed_in: false, identity: null };
      },
      auth_request_signin_code: () => null,
      auth_verify_signin_code: () => {
        (window as unknown as Record<string, unknown>).__ATTUNE_SIGNED_IN__ = true;
        return {
          user_id: "user-1",
          email: "ege@clinora.ai",
          display_name: null,
          privacy_tier: null,
        };
      },
      auth_logout: () => {
        (window as unknown as Record<string, unknown>).__ATTUNE_SIGNED_IN__ = false;
        return null;
      },
      list_recordings: () => [],
      recording_status: () => ({
        active: false,
        elapsed_seconds: 0,
        session_dir: null,
      }),
      list_providers: () => [],
      list_agents: () => [],
      list_tasks: () => [],
      list_memories: () => [],
      list_webhooks: () => [],
    },
  });
}

test.describe("Onboarding — fresh signup", () => {
  test("walks permissions → signup → OTP → workspace setup → main app", async ({
    page,
  }) => {
    await setupScenario(page);
    await page.goto("/");

    // 1. Permissions screen.
    await expect(
      page.getByRole("heading", { name: /allow attune to transcribe/i }),
    ).toBeVisible();

    // Both rows pre-granted by our `list_permissions` mock; the
    // screen's local state machine resolves to enabled Continue.
    await page.getByRole("button", { name: /^continue$/i }).click();

    // 2. Signup — Granola-style centred layout.
    await expect(page.getByRole("heading", { name: /welcome to attune/i })).toBeVisible();
    await page.getByPlaceholder(/you@company\.com/i).fill("ege@clinora.ai");
    await page.getByRole("button", { name: /^continue$/i }).first().click();

    // 3. Code entry. 6 inputs; userland uses the paste-distribute
    // path — fill the first input with the whole code so the rest
    // auto-populate.
    await expect(
      page.getByRole("heading", { name: /check your email/i }),
    ).toBeVisible();
    await page.locator('input[id="code-0"]').fill("123456");
    await page.getByRole("button", { name: /verify and continue/i }).click();

    // 4. EventKit rationale renders post-signup (PR #215 fix).
    await expect(
      page.getByRole("heading", { name: /read your mac.s calendar locally/i }),
    ).toBeVisible();
    await page.getByRole("button", { name: /skip for now/i }).click();

    // 5. Workspace name — auto-populated from the email domain.
    await expect(
      page.getByRole("heading", { name: /name your workspace/i }),
    ).toBeVisible();
    const nameField = page.getByLabel(/workspace name/i);
    await expect(nameField).toHaveValue("Clinora");
    await page.getByRole("button", { name: /^continue$/i }).click();

    // 6. Bucket — calendar was deferred, so this routes straight
    // to transcriber after the click.
    await expect(
      page.getByRole("heading", { name: /what do you do\?/i }),
    ).toBeVisible();
    await page.getByRole("radio", { name: /founder/i }).click();
    await page.getByRole("button", { name: /^continue$/i }).click();

    // 7. Transcriber (final).
    await expect(
      page.getByRole("heading", { name: /welcome to attune/i }),
    ).toBeVisible();
    await page.getByRole("button", { name: /i.?m ready/i }).click();

    // 8. Main app: sidebar appears, Record is the default route.
    await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
    await expect(page.getByRole("navigation").getByText(/inbox/i)).toBeVisible();

    // Verify the full IPC trail — at minimum we saw signup, verify,
    // save_settings (for onboarding_completed), and ping (Tauri's
    // app-boot probe).
    const log = await ipcLog(page);
    const commands = log.map((e) => e.cmd);
    expect(commands).toContain("auth_request_signin_code");
    expect(commands).toContain("auth_verify_signin_code");
    expect(commands).toContain("save_settings");
  });

  test("hides the sidebar entirely when signed out", async ({ page }) => {
    await setupScenario(page);
    await page.goto("/");

    // Sidebar items shouldn't be reachable when the auth gate is
    // active — only the conductor renders.
    await expect(page.getByRole("heading", { name: /allow attune to transcribe/i })).toBeVisible();
    await expect(page.getByRole("navigation")).toHaveCount(0);
  });
});

test.describe("Onboarding — returning user", () => {
  test("signed-out + onboarded → signup → OTP → main app (no workspace setup)", async ({
    page,
  }) => {
    await setupScenario(page, {
      initialSettings: { onboarding_completed: true },
    });
    await page.goto("/");

    // Lands directly on signup (permissions are skipped because
    // onboarding_completed is true).
    await expect(page.getByRole("heading", { name: /welcome to attune/i })).toBeVisible();
    await page.getByPlaceholder(/you@company\.com/i).fill("ege@clinora.ai");
    await page.getByRole("button", { name: /^continue$/i }).first().click();

    await expect(
      page.getByRole("heading", { name: /check your email/i }),
    ).toBeVisible();
    await page.locator('input[id="code-0"]').fill("987654");
    await page.getByRole("button", { name: /verify and continue/i }).click();

    // Critical: no EventKit / workspace screens. Straight to the
    // main app because onboarding was already done.
    await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
  });
});

test.describe("Settings — Profile reads identity from auth store", () => {
  test("signed-in user sees their email in Settings → Profile", async ({
    page,
  }) => {
    await setupScenario(page, {
      initialSettings: { onboarding_completed: true },
      startSignedIn: true,
    });
    await page.goto("/");

    // Main app loaded — open Settings via the sidebar Settings button.
    await expect(page.getByRole("heading", { name: /^record$/i })).toBeVisible();
    await page.getByRole("button", { name: /^settings$/i }).click();

    // Settings modal — pick Profile.
    await page.getByRole("button", { name: /^profile$/i }).click();
    await expect(page.getByText("ege@clinora.ai")).toBeVisible();
    // Profile section has the active Sign out button; other sections
    // may have disabled "Sign out all devices" affordances elsewhere.
    await expect(
      page.getByRole("button", { name: /^sign out$/i }).first(),
    ).toBeVisible();
  });
});
