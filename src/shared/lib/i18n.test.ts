import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { setMessages, t } from "./i18n";
import { messages } from "./messages.en";

beforeEach(() => {
  // Reset to the canonical English table so tests don't leak state
  // into each other.
  setMessages(messages as unknown as Record<string, string>);
});

afterEach(() => {
  setMessages(messages as unknown as Record<string, string>);
});

describe("t", () => {
  it("resolves a known key", () => {
    expect(t("errors.recordings.load")).toBe("Could not load recordings");
  });

  it("interpolates {name} placeholders", () => {
    expect(t("errors.agents.run", { name: "Summarize" })).toBe("Summarize failed");
  });

  it("falls back to the key when no message is found", () => {
    // Cast through unknown so the test can drive a missing-key edge
    // case without weakening the public type.
    expect(t("errors.nope" as unknown as Parameters<typeof t>[0])).toBe("errors.nope");
  });

  it("supports a defaultValue override for unknown keys", () => {
    expect(
      t("errors.nope" as unknown as Parameters<typeof t>[0], undefined, {
        defaultValue: "Fallback prose",
      })
    ).toBe("Fallback prose");
  });

  it("preserves unknown placeholders intact", () => {
    expect(t("errors.agents.run", {})).toBe("{name} failed");
  });

  it("setMessages swaps the locale table", () => {
    setMessages({ "errors.recordings.load": "Kayıtlar yüklenemedi" });
    expect(t("errors.recordings.load")).toBe("Kayıtlar yüklenemedi");
  });
});
