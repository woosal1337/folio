import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  applyInitialReadingControls,
  useReadingControls,
} from "./use-reading-controls";

const KEYS = {
  font: "attune.reading.font",
  size: "attune.reading.size",
  spacing: "attune.reading.spacing",
};

beforeEach(() => {
  window.localStorage.clear();
  const root = document.documentElement;
  delete root.dataset.readingFont;
  delete root.dataset.readingSize;
  delete root.dataset.readingSpacing;
});

afterEach(() => {
  window.localStorage.clear();
});

describe("useReadingControls", () => {
  it("defaults to system / m / normal when nothing is stored", () => {
    const { result } = renderHook(() => useReadingControls());
    expect(result.current.font).toBe("system");
    expect(result.current.size).toBe("m");
    expect(result.current.spacing).toBe("normal");
    expect(document.documentElement.dataset.readingFont).toBe("system");
    expect(document.documentElement.dataset.readingSize).toBe("m");
    expect(document.documentElement.dataset.readingSpacing).toBe("normal");
  });

  it("persists each setter to localStorage and applies to the root", () => {
    const { result } = renderHook(() => useReadingControls());
    act(() => result.current.setFont("fraunces"));
    act(() => result.current.setSize("xl"));
    act(() => result.current.setSpacing("wide"));
    expect(window.localStorage.getItem(KEYS.font)).toBe("fraunces");
    expect(window.localStorage.getItem(KEYS.size)).toBe("xl");
    expect(window.localStorage.getItem(KEYS.spacing)).toBe("wide");
    expect(document.documentElement.dataset.readingFont).toBe("fraunces");
    expect(document.documentElement.dataset.readingSize).toBe("xl");
    expect(document.documentElement.dataset.readingSpacing).toBe("wide");
  });

  it("reads back stored values on next mount", () => {
    window.localStorage.setItem(KEYS.font, "atkinson-hyperlegible");
    window.localStorage.setItem(KEYS.size, "l");
    window.localStorage.setItem(KEYS.spacing, "wider");
    const { result } = renderHook(() => useReadingControls());
    expect(result.current.font).toBe("atkinson-hyperlegible");
    expect(result.current.size).toBe("l");
    expect(result.current.spacing).toBe("wider");
  });

  it("falls back to defaults for unknown stored values", () => {
    window.localStorage.setItem(KEYS.font, "comic-sans");
    window.localStorage.setItem(KEYS.size, "huge");
    window.localStorage.setItem(KEYS.spacing, "loose");
    const { result } = renderHook(() => useReadingControls());
    expect(result.current.font).toBe("system");
    expect(result.current.size).toBe("m");
    expect(result.current.spacing).toBe("normal");
  });
});

describe("applyInitialReadingControls", () => {
  it("applies stored values to the document root", () => {
    window.localStorage.setItem(KEYS.font, "opendyslexic");
    window.localStorage.setItem(KEYS.size, "s");
    window.localStorage.setItem(KEYS.spacing, "tight");
    applyInitialReadingControls();
    expect(document.documentElement.dataset.readingFont).toBe("opendyslexic");
    expect(document.documentElement.dataset.readingSize).toBe("s");
    expect(document.documentElement.dataset.readingSpacing).toBe("tight");
  });

  it("falls back to defaults when nothing is stored", () => {
    applyInitialReadingControls();
    expect(document.documentElement.dataset.readingFont).toBe("system");
    expect(document.documentElement.dataset.readingSize).toBe("m");
    expect(document.documentElement.dataset.readingSpacing).toBe("normal");
  });
});
