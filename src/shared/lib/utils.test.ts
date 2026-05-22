import { describe, expect, it } from "vitest";

import { cn, formatBytes, formatDuration } from "./utils";

describe("cn", () => {
  it("merges class names", () => {
    expect(cn("a", "b")).toBe("a b");
  });

  it("dedupes conflicting tailwind classes (tailwind-merge)", () => {
    expect(cn("p-2", "p-4")).toBe("p-4");
  });

  it("filters falsy values", () => {
    expect(cn("a", false, null, undefined, "b")).toBe("a b");
  });
});

describe("formatDuration", () => {
  it("formats seconds under one minute", () => {
    expect(formatDuration(0)).toBe("0:00");
    expect(formatDuration(5)).toBe("0:05");
    expect(formatDuration(59)).toBe("0:59");
  });

  it("formats minutes:seconds under one hour", () => {
    expect(formatDuration(60)).toBe("1:00");
    expect(formatDuration(125)).toBe("2:05");
    expect(formatDuration(3599)).toBe("59:59");
  });

  it("formats hours:minutes:seconds", () => {
    expect(formatDuration(3600)).toBe("1:00:00");
    expect(formatDuration(3661)).toBe("1:01:01");
  });
});

describe("formatBytes", () => {
  it("formats zero", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("formats kB", () => {
    expect(formatBytes(1024)).toMatch(/1.*kB/);
  });

  it("formats MB", () => {
    expect(formatBytes(1024 * 1024 * 5)).toMatch(/5.*MB/);
  });

  it("formats GB", () => {
    expect(formatBytes(1024 * 1024 * 1024 * 2.5)).toMatch(/2\.5.*GB/);
  });

  it("treats negative as zero", () => {
    expect(formatBytes(-1)).toBe("0 B");
  });
});
