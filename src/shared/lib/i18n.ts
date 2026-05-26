/**
 * Tiny i18n surface for user-facing strings.
 *
 * v2 roadmap finding R13 / GET-118: lays the plumbing so the next
 * pass (#096) can drop a TR / DE / FR translation in as a single
 * additional dictionary. We don't ship a third-party i18n library
 * — the message space is small (~150 strings worst-case), the keys
 * are static at call sites, and the static lookup is faster + has
 * fewer moving parts than i18next-style runtime resolution.
 *
 * Contract:
 *   t("errors.recordings.load")
 *     → "Could not load recordings"
 *   t("errors.recording.delete", { label: "2026-05-25-meeting" })
 *     → "Could not delete recording 2026-05-25-meeting"
 *
 * Missing keys fall back to the key itself + the params object — so
 * a typo doesn't crash the UI; it surfaces as a visible string the
 * dev can grep for.
 */

import { messages, type MessageKey, type MessageParams } from "./messages.en";

export type { MessageKey } from "./messages.en";

const cache: Record<string, string> = { ...messages };

interface TOptions {
  /** Override the active locale's messages with a fallback string for
   *  this one call. Useful when a back-end error already carries a
   *  human-readable message we want to forward verbatim. */
  defaultValue?: string;
}

/**
 * Resolve a translation key against the active locale (English for
 * now), interpolating `{name}` placeholders with the provided params.
 */
export function t(key: MessageKey, params?: MessageParams, options?: TOptions): string {
  const template = cache[key] ?? options?.defaultValue ?? key;
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_, name: string) => {
    const value = params[name as keyof MessageParams];
    return value === undefined || value === null ? `{${name}}` : String(value);
  });
}

/** Replace the active message table. Locale switching is a follow-up;
 *  this function exists so the eventual TR / DE bundle can swap the
 *  cache without touching every call site. */
export function setMessages(next: Record<string, string>): void {
  for (const k of Object.keys(cache)) delete cache[k];
  Object.assign(cache, next);
}

/**
 * Locale catalogue for the eight launch languages (v2 finding 096 /
 * GET-85). Turkish ships first because the maintainer is Turkish and
 * catches breakage early. Arabic forces RTL plumbing. Japanese and
 * Chinese force CJK fallback in the font stack.
 */
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

/**
 * Best-effort match: pick the user's preferred locale that we ship.
 * Falls through to DEFAULT_LOCALE when no match. Accepts BCP-47-style
 * tags ("en-GB", "zh-Hant") by taking the language subtag.
 */
export function resolveLocale(preferred: readonly string[]): Locale {
  for (const candidate of preferred) {
    const tag = candidate.toLowerCase().split("-")[0] ?? "";
    if ((LOCALES as readonly string[]).includes(tag)) return tag as Locale;
  }
  return DEFAULT_LOCALE;
}

/**
 * Apply the locale's reading direction + lang attribute to the
 * document. RTL locales flip every Tailwind `start-*` / `end-*`
 * direction-aware utility automatically; absolute `left-` /
 * `right-` utilities must be migrated to logical equivalents.
 */
export function applyDocumentDirection(locale: Locale, doc: Document = document): void {
  doc.documentElement.lang = locale;
  doc.documentElement.dir = LOCALE_SPECS[locale].direction;
}
