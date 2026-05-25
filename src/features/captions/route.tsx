import { useRecording } from "@/shared/stores/recording-store";

/**
 * Captions window route. Hosted in a separate borderless always-on-top
 * Tauri window the user opens from the Record page. Renders the last
 * ~3 sentences in 22-48pt type so the user can keep meeting context
 * visible while attending to something else.
 *
 * v2 roadmap finding 103 / GET-115. Streaming partial transcripts are
 * the eventual data source; for now we surface a status line driven
 * by the recording-store + the most recent finalised transcript snippet
 * the store cached. The route mounts inside the same React app and
 * picks up the same Zustand state because Tauri windows share the
 * same WebView in single-process mode (via the captions Vite entry).
 */
export default function CaptionsRoute() {
  const recording = useRecording((s) => s.recording);
  const elapsed = useRecording((s) => s.elapsed);
  const lastTranscript = useRecording((s) => s.lastTranscriptPath);

  const headline = recording ? "Recording" : "Idle";
  const subline = lastTranscript
    ? `Last transcript: ${lastTranscript.split("/").slice(-2).join("/")}`
    : "No transcript captured yet";
  const elapsedLabel = recording
    ? `${Math.floor(elapsed / 60)
        .toString()
        .padStart(2, "0")}:${(elapsed % 60).toString().padStart(2, "0")}`
    : "";

  return (
    <div
      data-drag=""
      className="flex h-screen w-screen select-none flex-col items-center justify-center gap-4 bg-black/85 px-8 py-6 text-center text-white backdrop-blur"
      role="region"
      aria-label="Live captions"
    >
      <div
        className="text-[32px] font-medium leading-tight tracking-tight"
        aria-live="polite"
      >
        {headline}
        {recording && elapsedLabel ? (
          <span className="ml-3 font-mono text-[24px] tabular-nums opacity-70">
            {elapsedLabel}
          </span>
        ) : null}
      </div>
      <p className="max-w-[80ch] text-[18px] text-white/70">{subline}</p>
      <p className="text-[12px] uppercase tracking-[0.25em] text-white/40">
        Live captions · always on top
      </p>
    </div>
  );
}
