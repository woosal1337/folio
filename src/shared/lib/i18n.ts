import { messages, type MessageKey, type MessageParams } from "./messages.en";

export type { MessageKey } from "./messages.en";

const cache: Record<string, string> = { ...messages };

interface TOptions {
  defaultValue?: string;
}

export function t(key: MessageKey, params?: MessageParams, options?: TOptions): string {
  const template = cache[key] ?? options?.defaultValue ?? key;
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_, name: string) => {
    const value = params[name as keyof MessageParams];
    return value === undefined || value === null ? `{${name}}` : String(value);
  });
}

export function setMessages(next: Record<string, string>): void {
  for (const k of Object.keys(cache)) delete cache[k];
  Object.assign(cache, next);
}

export const LOCALES = ["en", "tr", "ar", "es", "de", "fr", "ja", "zh"] as const;
export type Locale = (typeof LOCALES)[number];

export const DEFAULT_LOCALE: Locale = "en";

export const RTL_LOCALES: ReadonlySet<Locale> = new Set<Locale>(["ar"]);

export interface LocaleSpec {
  id: Locale;
  label: string;
  native: string;
  direction: "ltr" | "rtl";
}

export const LOCALE_SPECS: Record<Locale, LocaleSpec> = {
  en: { id: "en", label: "English", native: "English", direction: "ltr" },
  tr: { id: "tr", label: "Turkish", native: "Türkçe", direction: "ltr" },
  ar: { id: "ar", label: "Arabic", native: "العربية", direction: "rtl" },
  es: { id: "es", label: "Spanish", native: "Español", direction: "ltr" },
  de: { id: "de", label: "German", native: "Deutsch", direction: "ltr" },
  fr: { id: "fr", label: "French", native: "Français", direction: "ltr" },
  ja: { id: "ja", label: "Japanese", native: "日本語", direction: "ltr" },
  zh: { id: "zh", label: "Chinese", native: "中文", direction: "ltr" },
};

export function resolveLocale(preferred: readonly string[]): Locale {
  for (const candidate of preferred) {
    const tag = candidate.toLowerCase().split("-")[0] ?? "";
    if ((LOCALES as readonly string[]).includes(tag)) return tag as Locale;
  }
  return DEFAULT_LOCALE;
}

export function applyDocumentDirection(locale: Locale, doc: Document = document): void {
  doc.documentElement.lang = locale;
  doc.documentElement.dir = LOCALE_SPECS[locale].direction;
}
