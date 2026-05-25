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
