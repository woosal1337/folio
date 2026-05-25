import * as React from "react";

import { setMessages } from "@/shared/lib/i18n";
import { messages } from "@/shared/lib/messages.en";
import { messagesTr } from "@/shared/lib/messages.tr";

export type Locale = "en" | "tr" | "ar" | "he";
export const LOCALES: Locale[] = ["en", "tr", "ar", "he"];

/** Reading direction for a locale. Hebrew + Arabic are right-to-left;
 *  every other locale we currently ship is left-to-right. Used by
 *  applyInitialLocale to set document.documentElement.dir so flexbox
 *  / grid layouts mirror automatically (the chrome already uses
 *  logical properties via Tailwind). v2 finding 099 / GET-112. */
export function dirFor(locale: Locale): "ltr" | "rtl" {
  return locale === "ar" || locale === "he" ? "rtl" : "ltr";
}

const STORAGE_KEY = "attune.locale";

function tableFor(locale: Locale): Record<string, string> {
  switch (locale) {
    case "tr":
      return messagesTr as unknown as Record<string, string>;
    case "en":
    default:
      return messages as unknown as Record<string, string>;
  }
}

function readStored(): Locale {
  if (typeof window === "undefined") return "en";
  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (raw === "tr" || raw === "ar" || raw === "he" || raw === "en") {
    return raw;
  }
  return "en";
}

/**
 * Subscribe + setter for the active locale. Swaps the active message
 * table in the i18n module so `t(...)` call sites at the existing
 * frontends pick up the new strings without prop drilling. Persists
 * to localStorage so the next launch keeps the choice.
 *
 * v2 roadmap finding 085 / GET-105.
 */
export function useLocale() {
  const [locale, setLocaleState] = React.useState<Locale>(() => readStored());

  React.useEffect(() => {
    setMessages(tableFor(locale));
    window.localStorage.setItem(STORAGE_KEY, locale);
    document.documentElement.lang = locale;
    document.documentElement.dir = dirFor(locale);
  }, [locale]);

  return { locale, setLocale: setLocaleState };
}

/** Apply the saved locale before React mounts so cold-start UI shows
 *  the right language without a flash of English. */
export function applyInitialLocale() {
  const locale = readStored();
  setMessages(tableFor(locale));
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
    document.documentElement.dir = dirFor(locale);
  }
}
