/**
 * End-to-end render test for the post-signup conductor flow.
 *
 * Simulates a fresh user: permissions granted → email entered → OTP
 * verified → conductor advances to EventKit / workspace name /
 * workspace bucket / invite-teammates / transcriber. Asserts that
 * the conductor stays mounted across the auth-store `signedIn`
 * flip (the bug that PR #215 fixed) and that returning users skip
 * straight to the app.
 *
 * Mocks the `@tauri-apps/api/core::invoke` so no real backend is
 * required. Mocks `setProviderKey` etc through `@/shared/lib/ipc`.
 */

import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// ---- IPC mock ------------------------------------------------------

vi.mock("@/shared/lib/ipc", async () => {
  const actual = await vi.importActual<typeof import("@/shared/lib/ipc")>(
    "@/shared/lib/ipc",
  );
  return {
    ...actual,
    listPermissions: vi.fn(async () => [
      {
        permission: "Microphone",
        status: "Granted",
        rationale: "",
        settings_url: "",
      },
      {
        permission: "ScreenRecording",
        status: "Granted",
        rationale: "",
        settings_url: "",
      },
    ]),
    openPermissionSettings: vi.fn(async () => {}),
    requestCalendarAccess: vi.fn(async () => {}),
    listAttendeeSuggestions: vi.fn(async () => []),
    setProviderKey: vi.fn(async () => {}),
    authRequestSigninCode: vi.fn(async () => {}),
    authVerifySigninCode: vi.fn(async () => ({
      user_id: "user-1",
      email: "ege@clinora.ai",
      display_name: null,
      privacy_tier: null,
    })),
  };
});

// ---- Settings store mock -------------------------------------------

interface MockSettings {
  onboarding_completed: boolean;
  workspace_name: string;
  workspace_bucket: string;
  onboarding_calendar_deferred: boolean;
  transcriber: "local_whisper" | "openai";
  signin_mode: string;
  briefing_language: string;
}

function makeSettings(overrides: Partial<MockSettings> = {}): MockSettings {
  return {
    onboarding_completed: false,
    workspace_name: "",
    workspace_bucket: "",
    onboarding_calendar_deferred: false,
    transcriber: "local_whisper",
    signin_mode: "",
    briefing_language: "en",
    ...overrides,
  };
}

vi.mock("@/shared/stores/settings-store", () => {
  let settings: MockSettings = makeSettings();
  const subscribers = new Set<() => void>();
  const notify = () => subscribers.forEach((s) => s());

  function useStore<T>(selector: (s: ReturnType<typeof state>) => T): T {
    const [, setTick] = React.useState(0);
    React.useEffect(() => {
      const sub = () => setTick((t) => t + 1);
      subscribers.add(sub);
      return () => {
        subscribers.delete(sub);
      };
    }, []);
    return selector(state());
  }

  const state = () => ({
    settings: settings as unknown as Record<string, unknown>,
    load: async () => {},
    save: async (next: MockSettings) => {
      settings = { ...settings, ...next };
      notify();
    },
  });

  return {
    useSettingsStore: Object.assign(useStore, {
      getState: state,
      setState: (patch: Partial<MockSettings>) => {
        settings = { ...settings, ...patch };
        notify();
      },
      __reset: () => {
        settings = makeSettings();
        notify();
      },
    }),
  };
});

// ---- React import after mocks --------------------------------------

import * as React from "react";
import { FirstRunConductor } from "./first-run";
import { useAuthStore } from "@/shared/stores/auth-store";
import { useSettingsStore } from "@/shared/stores/settings-store";

beforeEach(() => {
  (
    useSettingsStore as unknown as { __reset: () => void }
  ).__reset();
  useAuthStore.setState({
    hydrated: true,
    signedIn: false,
    identity: null,
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

// ---- Tests ---------------------------------------------------------

describe("FirstRunConductor — post-signup workspace setup", () => {
  it("eventkit → workspace-name → workspace-bucket → transcriber → onFinish", async () => {
    // Simulate "just verified OTP" state: signed in, onboarding not
    // complete yet. The conductor's initialStep logic should resume
    // at EventKit — this is the path PR #215 unblocks.
    useAuthStore.setState({
      hydrated: true,
      signedIn: true,
      identity: {
        user_id: "user-1",
        email: "ege@clinora.ai",
        display_name: null,
        privacy_tier: null,
      },
    });
    const user = userEvent.setup();
    const onFinish = vi.fn();
    render(<FirstRunConductor onFinish={onFinish} />);

    // 1. EventKit rationale.
    expect(
      await screen.findByRole("heading", {
        name: /read your mac.s calendar locally/i,
      }),
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /skip for now/i }));

    // 2. Workspace name — auto-populated from the signed-in email.
    expect(
      await screen.findByRole("heading", { name: /name your workspace/i }),
    ).toBeTruthy();
    const nameInput = await screen.findByLabelText(/workspace name/i);
    expect((nameInput as HTMLInputElement).value).toBe("Clinora");
    await user.click(screen.getByRole("button", { name: /^continue$/i }));

    // 3. Workspace bucket — calendar was deferred, so this routes
    // straight to transcriber (skipping invite-teammates).
    expect(
      await screen.findByRole("heading", { name: /what do you do\?/i }),
    ).toBeTruthy();
    await user.click(screen.getByRole("radio", { name: /founder/i }));
    await user.click(screen.getByRole("button", { name: /^continue$/i }));

    // 4. Transcriber.
    expect(
      await screen.findByRole("heading", { name: /welcome to attune/i }),
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /i.?m ready/i }));

    await waitFor(() => expect(onFinish).toHaveBeenCalled());
  }, 20000);

  it("returning user (onboarding_completed=true, signed out) skips workspace screens after OTP", async () => {
    const user = userEvent.setup();
    (useSettingsStore as unknown as { setState: (p: object) => void }).setState({
      onboarding_completed: true,
    });
    const onFinish = vi.fn();
    render(<FirstRunConductor onFinish={onFinish} />);

    // Lands directly on signup (skips permissions).
    expect(
      await screen.findByRole("heading", { name: /welcome to attune/i }),
    ).toBeTruthy();
    await user.type(
      await screen.findByPlaceholderText(/you@company\.com/i),
      "ege@clinora.ai",
    );
    await user.click(
      screen.getAllByRole("button", { name: /^continue$/i })[0]!,
    );

    // Code entry; verify.
    expect(
      await screen.findByRole("heading", { name: /check your email/i }),
    ).toBeTruthy();
    const digits = screen
      .getAllByRole("textbox")
      .filter((el) => (el as HTMLInputElement).maxLength === 6);
    for (let i = 0; i < 6; i++) {
      await user.type(digits[i]!, String(i));
    }
    await user.click(
      await screen.findByRole("button", { name: /verify and continue/i }),
    );

    // The returning-user shortcut: handleVerified calls onFinish
    // immediately when onboarding_completed is already true.
    await waitFor(() => expect(onFinish).toHaveBeenCalled());
  }, 20000);
});

describe("FirstRunConductor — auth gate", () => {
  it("a fresh signed-in user with onboarding_completed=false resumes at eventkit", async () => {
    useAuthStore.setState({
      hydrated: true,
      signedIn: true,
      identity: {
        user_id: "user-1",
        email: "ege@clinora.ai",
        display_name: null,
        privacy_tier: null,
      },
    });
    const onFinish = vi.fn();
    render(<FirstRunConductor onFinish={onFinish} />);
    expect(
      await screen.findByRole("heading", {
        name: /read your mac.s calendar locally/i,
      }),
    ).toBeTruthy();
    expect(onFinish).not.toHaveBeenCalled();
  });
});

describe("Settings → Profile reads identity from auth store", () => {
  it("renders the signed-in email", async () => {
    useAuthStore.setState({
      hydrated: true,
      signedIn: true,
      identity: {
        user_id: "user-1",
        email: "ege@clinora.ai",
        display_name: null,
        privacy_tier: null,
      },
    });
    const { SectionProfile } = await import(
      "@/features/settings/section-profile"
    );
    const settings = makeSettings({ briefing_language: "en" });
    render(
      <SectionProfile
        settings={settings as unknown as never}
        onChange={() => {}}
      />,
    );
    expect(await screen.findByText("ege@clinora.ai")).toBeTruthy();
  });

  it("falls back to 'Not signed in' when identity is missing", async () => {
    useAuthStore.setState({
      hydrated: true,
      signedIn: false,
      identity: null,
    });
    const { SectionProfile } = await import(
      "@/features/settings/section-profile"
    );
    const settings = makeSettings();
    render(
      <SectionProfile
        settings={settings as unknown as never}
        onChange={() => {}}
      />,
    );
    expect(await screen.findByText(/not signed in/i)).toBeTruthy();
  });
});
