import * as React from "react";

import { setMessages } from "@/shared/lib/i18n";
import { messages } from "@/shared/lib/messages.en";
import { messagesTr } from "@/shared/lib/messages.tr";

export type Locale = "en" | "tr";
export const LOCALES: Locale[] = ["en", "tr"];

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
  return raw === "tr" ? "tr" : "en";
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
  }
}
