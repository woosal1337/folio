import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

// The auth store transitively imports the IPC layer; stub the Tauri
// bridge so nothing tries to reach a real backend.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useAuthStore } from "@/shared/stores/auth-store";
import { currentDisplayName, useDisplayName } from "./use-current-user";

const identity = {
  user_id: "u1",
  email: "ege@clinora.ai",
  display_name: "Ege Çelebi",
  privacy_tier: "tier1",
};

afterEach(() => {
  useAuthStore.setState({ signedIn: false, identity: null, hydrated: false });
});

describe("useDisplayName", () => {
  it("returns the signed-in user's display name", () => {
    useAuthStore.setState({ signedIn: true, identity });
    const { result } = renderHook(() => useDisplayName());
    expect(result.current).toBe("Ege Çelebi");
  });

  it("returns an empty string when signed out", () => {
    useAuthStore.setState({ signedIn: false, identity: null });
    const { result } = renderHook(() => useDisplayName());
    expect(result.current).toBe("");
  });

  it("returns an empty string when the name is unset", () => {
    useAuthStore.setState({
      signedIn: true,
      identity: { ...identity, display_name: null },
    });
    const { result } = renderHook(() => useDisplayName());
    expect(result.current).toBe("");
  });

  it("updates reactively when the cached name changes", () => {
    useAuthStore.setState({ signedIn: true, identity });
    const { result, rerender } = renderHook(() => useDisplayName());
    expect(result.current).toBe("Ege Çelebi");
    useAuthStore.getState().setDisplayName("Renamed");
    rerender();
    expect(result.current).toBe("Renamed");
  });
});

describe("currentDisplayName (non-reactive)", () => {
  it("reads the store outside React", () => {
    useAuthStore.setState({ signedIn: true, identity });
    expect(currentDisplayName()).toBe("Ege Çelebi");
  });

  it("is empty when signed out", () => {
    useAuthStore.setState({ signedIn: false, identity: null });
    expect(currentDisplayName()).toBe("");
  });
});
