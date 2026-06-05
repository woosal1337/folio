/**
 * Render test for the local-only first-run conductor.
 *
 * Simulates a fresh user walking the local flow:
 *   permissions (granted) → eventkit (skip) → transcriber → onFinish.
 *
 * Attune is fully local — there is no sign-in, OTP, or workspace setup —
 * so the conductor only primes permissions, offers calendar access, and
 * picks a transcriber before flipping `onboarding_completed`.
 *
 * Mocks `@/shared/lib/ipc` so no real OS calls happen, and mocks the
 * settings store so `save` flips `onboarding_completed` in memory.
 */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type * as IpcModule from "@/shared/lib/ipc";

// ---- IPC mock ------------------------------------------------------

vi.mock("@/shared/lib/ipc", async () => {
  const actual = await vi.importActual<typeof IpcModule>("@/shared/lib/ipc");
  return {
    ...actual,
    listPermissions: vi.fn(async () => [
      {
        permission: "microphone",
        status: "granted",
        rationale: "",
        settings_url: "",
      },
      {
        permission: "screen_recording",
        status: "granted",
        rationale: "",
        settings_url: "",
      },
    ]),
    openPermissionSettings: vi.fn(async () => {}),
    requestCalendarAccess: vi.fn(async () => {}),
    setProviderKey: vi.fn(async () => {}),
  };
});

// ---- Settings store mock -------------------------------------------

interface MockSettings {
  onboarding_completed: boolean;
  onboarding_calendar_deferred: boolean;
  transcriber: "local_whisper" | "openai";
}

function makeSettings(overrides: Partial<MockSettings> = {}): MockSettings {
  return {
    onboarding_completed: false,
    onboarding_calendar_deferred: false,
    transcriber: "local_whisper",
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
import { useSettingsStore } from "@/shared/stores/settings-store";

beforeEach(() => {
  (useSettingsStore as unknown as { __reset: () => void }).__reset();
});

afterEach(() => {
  vi.clearAllMocks();
});

// ---- Tests ---------------------------------------------------------

describe("FirstRunConductor — local-only setup", () => {
  it("permissions → eventkit (skip) → transcriber → onFinish", async () => {
    const user = userEvent.setup();
    const onFinish = vi.fn();
    render(<FirstRunConductor onFinish={onFinish} />);

    // 1. Permissions — mock reports both granted, so Continue is enabled.
    await user.click(await screen.findByRole("button", { name: /continue/i }));

    // 2. EventKit rationale — defer calendar.
    expect(
      await screen.findByRole("heading", {
        name: /read your mac.s calendar locally/i,
      })
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /skip for now/i }));

    // 3. Transcriber — finish.
    expect(
      await screen.findByRole("heading", { name: /welcome to attune/i })
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /i.?m ready/i }));

    await waitFor(() => expect(onFinish).toHaveBeenCalled());
  }, 20000);

  it("does not call onFinish before reaching the transcriber step", async () => {
    const onFinish = vi.fn();
    render(<FirstRunConductor onFinish={onFinish} />);
    // Still on the permissions screen — nothing finished yet.
    expect(onFinish).not.toHaveBeenCalled();
  });
});
