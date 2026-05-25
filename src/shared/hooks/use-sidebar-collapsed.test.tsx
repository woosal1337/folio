import { act, fireEvent, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { useSidebarCollapsed } from "./use-sidebar-collapsed";

const STORAGE_KEY = "attune.sidebar.collapsed";

function setViewportWidth(width: number) {
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    writable: true,
    value: width,
  });
  fireEvent(window, new Event("resize"));
}

beforeEach(() => {
  window.localStorage.clear();
  setViewportWidth(1280);
});

afterEach(() => {
  window.localStorage.clear();
});

describe("useSidebarCollapsed", () => {
  it("defaults to expanded on a wide window with nothing stored", () => {
    const { result } = renderHook(() => useSidebarCollapsed());
    expect(result.current.collapsed).toBe(false);
    expect(result.current.forcedByViewport).toBe(false);
  });

  it("toggle persists the user preference", () => {
    const { result } = renderHook(() => useSidebarCollapsed());
    act(() => result.current.toggle());
    expect(result.current.collapsed).toBe(true);
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe("1");
    act(() => result.current.toggle());
    expect(result.current.collapsed).toBe(false);
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe("0");
  });

  it("reads the stored preference on next mount", () => {
    window.localStorage.setItem(STORAGE_KEY, "1");
    const { result } = renderHook(() => useSidebarCollapsed());
    expect(result.current.collapsed).toBe(true);
  });

  it("force-collapses below the 900px breakpoint", () => {
    setViewportWidth(800);
    const { result } = renderHook(() => useSidebarCollapsed());
    expect(result.current.collapsed).toBe(true);
    expect(result.current.forcedByViewport).toBe(true);
  });

  it("restores the user preference when the window grows back", () => {
    setViewportWidth(800);
    const { result } = renderHook(() => useSidebarCollapsed());
    expect(result.current.collapsed).toBe(true);
    act(() => setViewportWidth(1280));
    expect(result.current.collapsed).toBe(false);
    expect(result.current.forcedByViewport).toBe(false);
  });

  it("Cmd+Ctrl+S toggles the user preference", () => {
    const { result } = renderHook(() => useSidebarCollapsed());
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "s",
          metaKey: true,
          ctrlKey: true,
        })
      );
    });
    expect(result.current.collapsed).toBe(true);
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "s",
          metaKey: true,
          ctrlKey: true,
        })
      );
    });
    expect(result.current.collapsed).toBe(false);
  });

  it("ignores unrelated key chords", () => {
    const { result } = renderHook(() => useSidebarCollapsed());
    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true }));
    });
    expect(result.current.collapsed).toBe(false);
  });
});
