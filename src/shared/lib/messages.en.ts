export type MessageParams = Record<string, string | number>;

export const messages = {
  "errors.recordings.load": "Could not load recordings",
  "errors.recording.delete": "Could not delete recording",
  "errors.recording.reveal": "Could not open Finder",
  "errors.recording.load": "Could not load recording",

  "errors.transcription.start": "Could not start transcription",
  "errors.transcription.retry": "Could not re-transcribe",
  "errors.transcript.save": "Could not save transcript",
  "errors.transcript.export": "Could not export transcript",

  "errors.agents.load": "Could not load agents",
  "errors.agents.run": "{name} failed",
  "errors.agents.deleteResult": "Could not delete {name} result",
  "errors.agents.runs.load": "Could not load AI runs",

  "errors.settings.load": "Could not load settings",
  "errors.settings.save": "Could not save settings",

  "errors.webhooks.load": "Could not load webhooks",
  "errors.webhooks.save": "Could not save webhook",
  "errors.webhooks.delete": "Could not delete webhook",
  "errors.webhooks.test": "Webhook test failed",
  "errors.webhooks.update": "Could not update webhook",

  "errors.snapshot.export": "Could not export snapshot",

  "errors.usage.load": "Could not load usage",

  "errors.share.openObsidian": "Could not open in Obsidian",
  "errors.share.copy": "Could not copy",

  "errors.generic": "Something went wrong",
} as const;

export type MessageKey = keyof typeof messages;
