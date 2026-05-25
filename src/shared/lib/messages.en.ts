/**
 * English message table. Each entry is the canonical English text for
 * a user-facing string the app surfaces (toasts, dialog headers,
 * empty-state copy). Adding a new key here is what makes it
 * translatable in the next locale bundle (#096).
 *
 * Keys are dotted, namespaced by feature:
 *   errors.<surface>.<action>
 *   toast.<surface>.<event>
 *
 * Placeholders use `{name}` syntax; the param object the call site
 * passes to `t(...)` must include matching keys.
 *
 * v2 roadmap finding R13 / GET-118.
 */

/** Interpolation params accepted by `t(...)`. */
export type MessageParams = Record<string, string | number>;

export const messages = {
  // ---- Library surface ------------------------------------------
  "errors.recordings.load": "Could not load recordings",
  "errors.recording.delete": "Could not delete recording",
  "errors.recording.reveal": "Could not open Finder",
  "errors.recording.load": "Could not load recording",

  // ---- Transcription surface ------------------------------------
  "errors.transcription.start": "Could not start transcription",
  "errors.transcription.retry": "Could not re-transcribe",
  "errors.transcript.save": "Could not save transcript",
  "errors.transcript.export": "Could not export transcript",

  // ---- Agents surface -------------------------------------------
  "errors.agents.load": "Could not load agents",
  "errors.agents.run": "{name} failed",
  "errors.agents.deleteResult": "Could not delete {name} result",
  "errors.agents.runs.load": "Could not load AI runs",

  // ---- Settings surface -----------------------------------------
  "errors.settings.load": "Could not load settings",
  "errors.settings.save": "Could not save settings",

  // ---- Webhooks surface -----------------------------------------
  "errors.webhooks.load": "Could not load webhooks",
  "errors.webhooks.save": "Could not save webhook",
  "errors.webhooks.delete": "Could not delete webhook",
  "errors.webhooks.test": "Webhook test failed",
  "errors.webhooks.update": "Could not update webhook",

  // ---- Snapshot / storage ---------------------------------------
  "errors.snapshot.export": "Could not export snapshot",

  // ---- Usage / costs --------------------------------------------
  "errors.usage.load": "Could not load usage",

  // ---- Memory / share -------------------------------------------
  "errors.share.openObsidian": "Could not open in Obsidian",
  "errors.share.copy": "Could not copy",

  // ---- Generic --------------------------------------------------
  "errors.generic": "Something went wrong",
} as const;

export type MessageKey = keyof typeof messages;
