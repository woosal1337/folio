import { useSettingsStore } from "@/shared/stores/settings-store";

export interface TranscriberCopy {
  isCloud: boolean;

  progressLabel: string;

  triggerTooltip: string;

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
