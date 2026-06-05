import * as React from "react";

import { setMessages } from "@/shared/lib/i18n";
import { messages } from "@/shared/lib/messages.en";
import { messagesTr } from "@/shared/lib/messages.tr";

export type Locale = "en" | "tr" | "ar" | "he";
export const LOCALES: Locale[] = ["en", "tr", "ar", "he"];

export function dirFor(locale: Locale): "ltr" | "rtl" {
  return locale === "ar" || locale === "he" ? "rtl" : "ltr";
}

const STORAGE_KEY = "folio.locale";

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

export function applyInitialLocale() {
  const locale = readStored();
  setMessages(tableFor(locale));
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
    document.documentElement.dir = dirFor(locale);
  }
}
