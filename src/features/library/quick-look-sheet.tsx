import * as React from "react";
import {
  FileAudio,
  FolderOpen,
  Loader2,
  Lock,
  Pencil,
  Send,
  Share2,
  Sparkles,
  X,
} from "lucide-react";
import { save as showSaveDialog } from "@tauri-apps/plugin-dialog";
import { exportShareBundle } from "@/shared/lib/ipc";
import { toast } from "sonner";

import { AudioPlayer } from "@/features/recording/audio-player";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/shared/ui/dialog";
import { ScrollArea } from "@/shared/ui/scroll-area";
import { Separator } from "@/shared/ui/separator";
import { readTranscript, sharePaths } from "@/shared/lib/ipc";
import { copyToClipboard } from "@/shared/lib/share";
import { formatBytes, formatDuration } from "@/shared/lib/utils";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";

interface Props {
  recording: RecordingSummary | null;
  onClose: () => void;
  onOpenInEditor: (recording: RecordingSummary) => void;
  onReveal: (recording: RecordingSummary) => void;
}

/** Cap on the transcript preview — matches the "first 30 lines" the
 *  v2 finding 012 asked for. We count segments rather than visual
 *  lines because that's what Whisper actually emits; 30 segments at
 *  typical cadence is a couple of minutes of speech, enough to gauge
 *  whether the recording is the one the user was looking for. */
const TRANSCRIPT_PREVIEW_SEGMENTS = 30;

/**
 * Quick Look-style preview panel for a Library recording (v2 finding
 * 012 / GET-46). Opens via Spacebar on a focused row in `/library`.
 * Renders the meta header, the mic audio player (which doubles as the
 * waveform surrogate — the real waveform render is a follow-up), the
 * first ~30 transcript segments if one exists, and Open / Reveal /
 * Share actions. Esc or Space (or clicking the backdrop) closes.
 */
export function QuickLookSheet({
  recording,
  onClose,
  onOpenInEditor,
  onReveal,
}: Props) {
  const open = recording !== null;
  const [transcript, setTranscript] = React.useState<SessionTranscript | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  // Load (or clear) the transcript whenever the previewed recording
  // changes. Cancel in-flight loads on rapid Space-presses across rows.
  React.useEffect(() => {
    if (!recording) {
      setTranscript(null);
      setError(null);
      setLoading(false);
      return;
    }
    if (!recording.has_transcript) {
      setTranscript(null);
      setError(null);
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    readTranscript(recording.session_dir)
      .then((t) => {
        if (cancelled) return;
        setTranscript(t);
      })
      .catch((e) => {
        if (cancelled) return;
        console.error("quick-look transcript:", e);
        setError(String(e));
      })
      .finally(() => {
        if (cancelled) return;
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [recording]);

  // Space toggles closed when the sheet is open. Radix Dialog already
  // handles Escape; we add Space because Quick Look semantics on macOS
  // dismiss on Space as well as Esc.
  React.useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === " " && !(e.target instanceof HTMLInputElement)) {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const previewSegments = React.useMemo(() => {
    if (!transcript) return [];
    return transcript.channels
      .flatMap((ch) =>
        ch.segments.map((s) => ({
          channel: ch.channel,
          start: s.start_seconds,
          text: s.text,
        }))
      )
      .sort((a, b) => a.start - b.start)
      .slice(0, TRANSCRIPT_PREVIEW_SEGMENTS);
  }, [transcript]);

  const [sealing, setSealing] = React.useState(false);

  if (!recording) return null;

  const micPath = recording.mic_bytes ? `${recording.session_dir}/mic.wav` : null;
  const systemPath = recording.system_bytes
    ? `${recording.session_dir}/system.wav`
    : null;
  const totalBytes =
    Number(recording.mic_bytes ?? 0n) + Number(recording.system_bytes ?? 0n);

  const handleSealedBundle = async () => {
    if (!recording) return;
    setSealing(true);
    try {
      const leaf = recording.label || "recording";
      const dest = await showSaveDialog({
        defaultPath: `${leaf}.attune-share`,
        filters: [{ name: "Attune share", extensions: ["attune-share"] }],
      });
      if (!dest) return;
      const summary = await exportShareBundle(recording.session_dir, dest);
      toast.success("Sealed bundle exported", {
        description: `${summary.files} files · sha256 ${summary.manifest_sha256.slice(0, 12)}…`,
      });
    } catch (e) {
      console.error("export_share_bundle:", e);
      toast.error("Could not export sealed bundle", { description: String(e) });
    } finally {
      setSealing(false);
    }
  };
  const handleShare = async () => {
    const header = `${recording.label}  ·  ${formatDuration(Number(recording.duration_seconds))}  ·  ${formatBytes(totalBytes)}`;
    const body = previewSegments
      .map((s) => `${formatStamp(s.start)}  ${speakerFor(s.channel)}: ${s.text.trim()}`)
      .join("\n");
    const summary = transcript
      ? `\n\n--- Preview (${previewSegments.length} segment${previewSegments.length === 1 ? "" : "s"}) ---\n${body}`
      : "";
    await copyToClipboard(`${header}${summary}`, "Quick Look summary copied");
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) onClose();
      }}
    >
      <DialogContent className="grid h-[640px] max-h-[85vh] w-[640px] max-w-[90vw] grid-rows-[auto_1fr_auto] gap-0 overflow-hidden p-0">
        <DialogTitle className="sr-only">Quick Look: {recording.label}</DialogTitle>
        <DialogDescription className="sr-only">
          Audio preview and the first {TRANSCRIPT_PREVIEW_SEGMENTS} transcript segments
          for this recording. Press Space or Escape to close.
        </DialogDescription>

        <header className="flex items-start justify-between gap-3 border-b border-border bg-secondary/40 px-6 py-4">
          <div className="flex min-w-0 flex-col gap-1">
            <div className="flex items-center gap-2">
              <FileAudio className="h-4 w-4 shrink-0 text-muted-foreground" />
              <h2 className="truncate font-mono text-sm">{recording.label}</h2>
              {recording.has_transcript && (
                <Badge variant="accent" className="gap-1 text-2xs">
                  <Sparkles className="h-3 w-3" />
                  transcribed
                </Badge>
              )}
            </div>
            <span className="font-mono text-2xs tabular-nums text-muted-foreground">
              {formatDuration(Number(recording.duration_seconds))} ·{" "}
              {formatBytes(totalBytes)}
            </span>
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={onClose}
            aria-label="Close preview"
            className="h-7 w-7 text-muted-foreground hover:text-foreground"
          >
            <X className="h-4 w-4" />
          </Button>
        </header>

        <ScrollArea className="min-h-0">
          <div className="flex flex-col gap-4 px-6 py-5">
            {micPath ? (
              <AudioPlayer filePath={micPath} label="Mic" channel="mic" />
            ) : (
              <p className="text-xs text-muted-foreground">No mic track.</p>
            )}
            {systemPath && <Separator />}
            {systemPath && (
              <AudioPlayer filePath={systemPath} label="System" channel="system" />
            )}

            <Separator />

            <section aria-label="Transcript preview" className="flex flex-col gap-2">
              <header className="flex items-center justify-between">
                <h3 className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
                  Transcript preview
                </h3>
                {transcript && (
                  <span className="text-2xs tabular-nums text-muted-foreground">
                    first {previewSegments.length} segment
                    {previewSegments.length === 1 ? "" : "s"}
                  </span>
                )}
              </header>

              {!recording.has_transcript ? (
                <p className="rounded-md border border-dashed border-border bg-card px-3 py-4 text-xs text-muted-foreground">
                  No transcript yet. Press Edit below to open and transcribe.
                </p>
              ) : loading ? (
                <div className="flex items-center gap-2 px-1 py-2 text-xs text-muted-foreground">
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  Loading transcript…
                </div>
              ) : error ? (
                <p className="text-xs text-destructive">{error}</p>
              ) : previewSegments.length === 0 ? (
                <p className="text-xs text-muted-foreground">
                  Whisper returned no segments for this recording.
                </p>
              ) : (
                <ol className="flex flex-col gap-1.5">
                  {previewSegments.map((s, i) => (
                    <li
                      key={`${i}-${s.start}`}
                      className="grid grid-cols-[64px_64px_1fr] gap-2 text-xs leading-relaxed"
                    >
                      <span className="font-mono text-2xs tabular-nums text-muted-foreground">
                        {formatStamp(s.start)}
                      </span>
                      <span className="text-2xs uppercase tracking-wider text-muted-foreground">
                        {speakerFor(s.channel)}
                      </span>
                      <span className="text-foreground">{s.text.trim()}</span>
                    </li>
                  ))}
                </ol>
              )}
            </section>
          </div>
        </ScrollArea>

        <footer className="flex items-center justify-between gap-2 border-t border-border bg-card px-6 py-3">
          <p className="text-2xs text-muted-foreground">Space or Esc to close</p>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                const files = [`${recording.session_dir}/mic.wav`];
                if (recording.has_transcript) {
                  files.push(`${recording.session_dir}/transcript.json`);
                }
                sharePaths(files).catch((e) => {
                  console.error("share_paths:", e);
                  toast.error("Could not open share sheet", { description: String(e) });
                });
              }}
              className="gap-2"
              aria-label="Send via macOS share sheet (AirDrop, Mail, Messages…)"
              title="Send via AirDrop, Mail, Messages, Notes, Reminders…"
            >
              <Send className="h-3.5 w-3.5" />
              Send
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleShare}
              className="gap-2"
              aria-label="Copy a preview summary to the clipboard"
            >
              <Share2 className="h-3.5 w-3.5" />
              Copy
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleSealedBundle}
              disabled={sealing}
              className="gap-2"
              aria-label="Export a sealed share bundle"
              title="Export the recording as a signed .attune-share zip"
            >
              {sealing ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <Lock className="h-3.5 w-3.5" />
              )}
              Seal
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                onReveal(recording);
                toast.success("Revealed in Finder");
              }}
              className="gap-2"
              aria-label="Reveal recording folder in Finder"
            >
              <FolderOpen className="h-3.5 w-3.5" />
              Reveal
            </Button>
            <Button
              size="sm"
              onClick={() => onOpenInEditor(recording)}
              className="gap-2"
              aria-label="Open recording in editor"
            >
              <Pencil className="h-3.5 w-3.5" />
              Open
            </Button>
          </div>
        </footer>
      </DialogContent>
    </Dialog>
  );
}

function formatStamp(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  return formatDuration(seconds);
}

function speakerFor(channel: string): string {
  if (channel === "mic") return "You";
  if (channel === "system") return "Others";
  return channel;
}
