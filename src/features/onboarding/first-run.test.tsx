import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type * as IpcModule from "@/shared/lib/ipc";

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
    setProviderKey: vi.fn(async () => {}),
  };
});

interface MockSettings {
  onboarding_completed: boolean;
  transcriber: "local_whisper" | "openai";
}

function makeSettings(overrides: Partial<MockSettings> = {}): MockSettings {
  return {
    onboarding_completed: false,
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

import * as React from "react";
import { FirstRunConductor } from "./first-run";
import { useSettingsStore } from "@/shared/stores/settings-store";

beforeEach(() => {
  (useSettingsStore as unknown as { __reset: () => void }).__reset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("FirstRunConductor — local-only setup", () => {
  it("permissions → transcriber → onFinish", async () => {
    const user = userEvent.setup();
    const onFinish = vi.fn();
    render(<FirstRunConductor onFinish={onFinish} />);

    await user.click(await screen.findByRole("button", { name: /continue/i }));

    expect(
      await screen.findByRole("heading", { name: /welcome to folio/i })
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /i.?m ready/i }));

    await waitFor(() => expect(onFinish).toHaveBeenCalled());
  }, 20000);

  it("does not call onFinish before reaching the transcriber step", async () => {
    const onFinish = vi.fn();
    render(<FirstRunConductor onFinish={onFinish} />);

    expect(onFinish).not.toHaveBeenCalled();
  });
});
