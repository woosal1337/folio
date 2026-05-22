import {
  ChevronDown,
  ChevronRight,
  FileAudio,
  FolderOpen,
  Loader2,
  Pencil,
  Sparkles,
  Trash2,
} from "lucide-react";

import { AudioPlayer } from "@/features/recording/audio-player";
import { TranscriptView } from "@/features/recording/transcript-view";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { Separator } from "@/shared/ui/separator";
import { formatBytes, formatDuration } from "@/shared/lib/utils";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

interface Props {
  item: RecordingSummary;
  open: boolean;
  transcribing: boolean;
  onToggle: () => void;
  onReveal: () => void;
  onDelete: () => void;
  /**
   * Optional "Open in Editor" action. The Library page wires this to
   * navigate to /editor/<label>; the Record page leaves it out so the
   * recent-recordings row stays focused on capture.
   */
  onOpenInEditor?: () => void;
  /**
   * Optional "Transcribe" action. Rendered only when the recording is
   * not currently transcribing and does not yet have a transcript on
   * disk — covers both never-attempted recordings and ones whose
   * earlier auto-transcription failed.
   */
  onTranscribe?: () => void;
}

/**
 * Single recording row used by both the Record page's "Recent
 * recordings" strip and the Library page's full list. Header shows
 * label + duration + size and a status badge (`transcribing` while a
 * job is running, `transcribed` once a transcript exists). Expanded
 * body renders the per-channel audio players and the transcript view
 * when one is available.
 */
export function RecordingRow({
  item,
  open,
  transcribing,
  onToggle,
  onReveal,
  onDelete,
  onOpenInEditor,
  onTranscribe,
}: Props) {
  const canTranscribe = !transcribing && !item.has_transcript && Boolean(onTranscribe);
  // ts-rs maps Rust's i64 / u64 to TypeScript `bigint`. At JSON-parse time
  // these arrive as plain numbers, but the type system insists. Cast back
  // to number here — recording files are well under 2^53 bytes.
  const totalBytes = Number(item.mic_bytes ?? 0n) + Number(item.system_bytes ?? 0n);
  const parts: string[] = [
    formatDuration(Number(item.duration_seconds)),
    formatBytes(totalBytes),
  ];

  const micPath = item.mic_bytes ? `${item.session_dir}/mic.wav` : null;
  const systemPath = item.system_bytes ? `${item.session_dir}/system.wav` : null;

  return (
    <Card className="overflow-hidden">
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-center gap-4 px-5 py-3 text-left transition-colors hover:bg-accent/40"
        aria-expanded={open}
      >
        {open ? (
          <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
        )}
        <FileAudio className="h-4 w-4 shrink-0 text-muted-foreground" />
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="truncate font-mono text-sm">{item.label}</span>
          <span className="font-mono text-2xs text-muted-foreground">
            {parts.join("  ·  ")}
          </span>
        </div>
        {transcribing ? (
          <Badge
            variant="accent"
            className="gap-1.5 font-mono text-2xs"
            role="status"
            aria-live="polite"
          >
            <Loader2 className="h-3 w-3 animate-spin" />
            transcribing
          </Badge>
        ) : item.has_transcript ? (
          <Badge
            variant="accent"
            className="gap-1.5 font-mono text-2xs"
            title="A transcript is available — expand the row to read it."
          >
            <Sparkles className="h-3 w-3" />
            transcribed
          </Badge>
        ) : null}
        <span
          className="ml-auto inline-flex items-center gap-1"
          onClick={(e) => e.stopPropagation()}
        >
          {canTranscribe && (
            <Button
              variant="ghost"
              size="sm"
              className="gap-2 text-foreground"
              onClick={onTranscribe}
              aria-label="Transcribe recording"
              title="Send to OpenAI Whisper to generate a transcript."
            >
              <Sparkles className="h-3.5 w-3.5" />
              Transcribe
            </Button>
          )}
          {onOpenInEditor && (
            <Button
              variant="ghost"
              size="sm"
              className="gap-2"
              onClick={onOpenInEditor}
              aria-label="Open in editor"
            >
              <Pencil className="h-3.5 w-3.5" />
              Edit
            </Button>
          )}
          <Button variant="ghost" size="sm" className="gap-2" onClick={onReveal}>
            <FolderOpen className="h-3.5 w-3.5" />
            Reveal
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
            aria-label="Delete recording"
            title="Delete recording"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
        </span>
      </button>

      {open && (
        <CardContent className="flex flex-col gap-4 border-t border-border bg-secondary/40 px-5 py-4">
          {micPath ? (
            <AudioPlayer filePath={micPath} label="Mic" />
          ) : (
            <p className="text-xs text-muted-foreground">No mic track.</p>
          )}
          {systemPath && <Separator />}
          {systemPath && <AudioPlayer filePath={systemPath} label="System" />}
          {item.has_transcript && (
            <>
              <Separator />
              <TranscriptView sessionDir={item.session_dir} />
            </>
          )}
        </CardContent>
      )}
    </Card>
  );
}
