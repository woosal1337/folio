import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  DEFAULT_LOCALE,
  LOCALES,
  LOCALE_SPECS,
  RTL_LOCALES,
  applyDocumentDirection,
  resolveLocale,
  setMessages,
  t,
} from "./i18n";
import { messages } from "./messages.en";

beforeEach(() => {
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

describe("locale catalogue", () => {
  it("ships eight launch languages", () => {
    expect(LOCALES.length).toBe(8);
    expect(new Set(LOCALES).size).toBe(8);
  });

  it("marks Arabic as the only RTL locale", () => {
    expect(RTL_LOCALES.has("ar")).toBe(true);
    for (const id of LOCALES) {
      if (id === "ar") continue;
      expect(RTL_LOCALES.has(id)).toBe(false);
    }
  });

  it("flags direction consistently with RTL set", () => {
    for (const id of LOCALES) {
      const expected = RTL_LOCALES.has(id) ? "rtl" : "ltr";
      expect(LOCALE_SPECS[id].direction).toBe(expected);
    }
  });
});

describe("resolveLocale", () => {
  it("picks the first preferred tag we ship", () => {
    expect(resolveLocale(["tr-TR", "en"])).toBe("tr");
    expect(resolveLocale(["fr-CA"])).toBe("fr");
  });

  it("falls back to default when nothing matches", () => {
    expect(resolveLocale(["xx-YY", "und"])).toBe(DEFAULT_LOCALE);
    expect(resolveLocale([])).toBe(DEFAULT_LOCALE);
  });

  it("strips region subtags", () => {
    expect(resolveLocale(["zh-Hant"])).toBe("zh");
    expect(resolveLocale(["EN-US"])).toBe("en");
  });
});

describe("applyDocumentDirection", () => {
  it("writes the lang and dir attributes onto <html>", () => {
    const doc = {
      documentElement: { lang: "", dir: "" } as unknown as HTMLElement,
    } as Document;
    applyDocumentDirection("ar", doc);
    expect(doc.documentElement.lang).toBe("ar");
    expect(doc.documentElement.dir).toBe("rtl");
    applyDocumentDirection("en", doc);
    expect(doc.documentElement.dir).toBe("ltr");
  });
});
