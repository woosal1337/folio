import { describe, expect, it } from "vitest";

import { classifyDeepLink } from "./deep-link-allowlist";

describe("classifyDeepLink", () => {
  it("classifies an allowed attune:// route", () => {
    const verdict = classifyDeepLink("attune://library");
    expect(verdict.kind).toBe("allowed-attune-route");
    if (verdict.kind === "allowed-attune-route") {
      expect(verdict.route).toBe("library");
    }
  });

  it("preserves the allowlisted autoStart query param", () => {
    const verdict = classifyDeepLink("attune://library?autoStart=1");
    expect(verdict.kind).toBe("allowed-attune-route");
    if (verdict.kind === "allowed-attune-route") {
      expect(verdict.params.autoStart).toBe("1");
    }
  });

  it("rejects routes retired by the Granola overhaul (record, inbox)", () => {
    for (const dead of ["attune://record", "attune://inbox"]) {
      const verdict = classifyDeepLink(dead);
      expect(verdict.kind).toBe("rejected");
    }
  });

  it("treats trailing path as the label param for editor", () => {
    const verdict = classifyDeepLink("attune://editor/2026-05-26-meeting");
    expect(verdict.kind).toBe("allowed-attune-route");
    if (verdict.kind === "allowed-attune-route") {
      expect(verdict.route).toBe("editor");
      expect(verdict.params.label).toBe("2026-05-26-meeting");
    }
  });

  it("rejects an unknown route", () => {
    const verdict = classifyDeepLink("attune://hijack");
    expect(verdict.kind).toBe("rejected");
    if (verdict.kind === "rejected") {
      expect(verdict.reason).toContain("hijack");
    }
  });

  it("rejects an unknown query parameter", () => {
    const verdict = classifyDeepLink("attune://library?evil=1");
    expect(verdict.kind).toBe("rejected");
    if (verdict.kind === "rejected") {
      expect(verdict.reason).toContain("evil");
    }
  });

  it("rejects non-attune schemes that are not audio files", () => {
    const verdict = classifyDeepLink("javascript:alert(1)");
    expect(verdict.kind).toBe("rejected");
    if (verdict.kind === "rejected") {
      expect(verdict.reason).toBe("unsupported scheme");
    }
  });

  it("classifies an audio file URL as allowed-audio-file", () => {
    const verdict = classifyDeepLink("file:///Users/ege/Music/test.wav");
    expect(verdict.kind).toBe("allowed-audio-file");
    if (verdict.kind === "allowed-audio-file") {
      expect(verdict.path).toBe("/Users/ege/Music/test.wav");
    }
  });

  it("rejects an empty attune:// (no route)", () => {
    const verdict = classifyDeepLink("attune://");
    expect(verdict.kind).toBe("rejected");
    if (verdict.kind === "rejected") {
      expect(verdict.reason).toBe("missing route");
    }
  });

  it("URL-decodes allowed param values", () => {
    const verdict = classifyDeepLink("attune://editor?label=2026-05-26%2010%3A00");
    expect(verdict.kind).toBe("allowed-attune-route");
    if (verdict.kind === "allowed-attune-route") {
      expect(verdict.params.label).toBe("2026-05-26 10:00");
    }
  });
});
