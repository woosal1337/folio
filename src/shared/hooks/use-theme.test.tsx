import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { applyInitialTheme, useTheme } from "./use-theme";

const STORAGE_KEY = "folio-theme";

beforeEach(() => {
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.classList.remove("dark");
});

afterEach(() => {
  window.localStorage.clear();
});

function mockMatchMedia(prefersDark: boolean) {
  const mql = {
    matches: prefersDark,
    media: "(prefers-color-scheme: dark)",
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    onchange: null,
    dispatchEvent: () => false,
  };
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: () => mql,
  });
}

afterEach(() => {
  // @ts-expect-error reset the matchMedia mock between tests
  delete window.matchMedia;
});

describe("useTheme", () => {
  it("defaults to the system theme (light when system is unknown)", () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("defaults to dark when the system prefers dark and nothing is stored", () => {
    mockMatchMedia(true);
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("an explicit stored choice overrides the system theme", () => {
    mockMatchMedia(true);
    window.localStorage.setItem(STORAGE_KEY, "light");
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");
  });

  it("toggle flips light <-> dark", () => {
    const { result } = renderHook(() => useTheme());
    act(() => result.current.toggle());
    expect(result.current.theme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    act(() => result.current.toggle());
    expect(result.current.theme).toBe("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("persists to localStorage", () => {
    const { result } = renderHook(() => useTheme());
    act(() => result.current.setTheme("dark"));
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe("dark");
  });

  it("reads from localStorage on next mount", () => {
    window.localStorage.setItem(STORAGE_KEY, "dark");
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("dark");
  });

  it("falls back to light for unknown stored values", () => {
    window.localStorage.setItem(STORAGE_KEY, "purple");
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");
  });
});

describe("applyInitialTheme", () => {
  it("applies the stored theme to the document root", () => {
    window.localStorage.setItem(STORAGE_KEY, "dark");
    applyInitialTheme();
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("defaults to the system theme (light when system is unknown)", () => {
    applyInitialTheme();
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("applies dark when the system prefers dark and nothing is stored", () => {
    mockMatchMedia(true);
    applyInitialTheme();
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
});
