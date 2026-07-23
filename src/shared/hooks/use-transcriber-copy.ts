import { useSettingsStore } from "@/shared/stores/settings-store";

export interface TranscriberCopy {
  isCloud: boolean;

  progressLabel: string;

  triggerTooltip: string;

  emptyStateHint: string;
}

export function useTranscriberCopy(): TranscriberCopy {
  const settings = useSettingsStore((s) => s.settings);

  if (settings?.transcriber === "openai") {
    return {
      isCloud: true,
      progressLabel: "Sending audio to OpenAI Whisper…",
      triggerTooltip: "Send to OpenAI Whisper to generate a transcript.",
      emptyStateHint:
        "Uses the OpenAI Whisper API. Configure your key in Settings → Transcription.",
    };
  }

  if (settings?.transcriber === "remote_server") {
    return {
      isCloud: true,
      progressLabel: "Processing on your server…",
      triggerTooltip: "Upload to your Folio Server and sync the transcript back.",
      emptyStateHint:
        "Audio uploads to your Folio Server for GPU transcription, then the transcript syncs back to this Mac. Manage the connection in Account.",
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
