import { useSettingsStore } from "@/shared/stores/settings-store";

/**
 * UI copy that varies based on which transcription backend is
 * currently selected (`settings.transcriber`). One source of truth so
 * the "sending audio to OpenAI" string never lies when the user has
 * switched to the local Whisper model.
 *
 * Falls back to the local-Whisper copy when settings haven't loaded
 * yet — local is the safer assumption on first paint and matches the
 * default configuration shipped with the app.
 */
export interface TranscriberCopy {
  /** True when the OpenAI cloud backend is selected. */
  isCloud: boolean;
  /** Inline progress message shown while transcription is running. */
  progressLabel: string;
  /** Tooltip shown on the "Transcribe" trigger button. */
  triggerTooltip: string;
  /** One-line explainer shown next to the empty-state "Transcribe now" CTA. */
  emptyStateHint: string;
}

export function useTranscriberCopy(): TranscriberCopy {
  const settings = useSettingsStore((s) => s.settings);
  const isCloud = settings?.transcriber === "openai";

  if (isCloud) {
    return {
      isCloud: true,
      progressLabel: "Sending audio to OpenAI Whisper…",
      triggerTooltip: "Send to OpenAI Whisper to generate a transcript.",
      emptyStateHint:
        "Uses the OpenAI Whisper API. Configure your key in Settings → Transcription.",
    };
  }

  return {
    isCloud: false,
    progressLabel: "Transcribing locally with Whisper…",
    triggerTooltip: "Transcribe locally with whisper.cpp on this Mac.",
    emptyStateHint:
      "Runs on this Mac via whisper.cpp. No audio leaves your machine. Switch backend in Settings → Transcription.",
  };
}
