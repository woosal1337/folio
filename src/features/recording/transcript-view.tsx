import * as React from "react";
import { FileText, Loader2 } from "lucide-react";

import { readTranscript } from "@/shared/lib/ipc";
import { formatDuration } from "@/shared/lib/utils";
import type { Transcript } from "@/shared/types/Transcript";

interface Props {
  sessionDir: string;
}

/**
 * Read the transcript for a session and render its segments. Loads
 * lazily on mount so the recording-row only pays the IO cost when the
 * user actually opens the row.
 */
export function TranscriptView({ sessionDir }: Props) {
  const [transcript, setTranscript] = React.useState<Transcript | null>(null);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    readTranscript(sessionDir)
      .then((t) => {
        if (cancelled) return;
        setTranscript(t);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(String(e));
      })
      .finally(() => {
        if (cancelled) return;
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [sessionDir]);

  if (loading) {
    return (
      <div
        className="flex items-center gap-2 text-xs text-muted-foreground"
        role="status"
        aria-live="polite"
      >
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
        <span>Loading transcript…</span>
      </div>
    );
  }

  if (error) {
    return <p className="text-xs text-destructive">Transcript: {error}</p>;
  }

  if (!transcript || transcript.segments.length === 0) {
    return (
      <p className="text-xs text-muted-foreground">
        Transcript is empty — Whisper returned no segments for this audio.
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <header className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
          <FileText className="h-3.5 w-3.5" />
          Transcript
        </div>
        {transcript.language && (
          <span className="font-mono text-2xs text-muted-foreground">
            {transcript.language}
          </span>
        )}
      </header>
      <ol className="flex flex-col gap-2">
        {transcript.segments.map((seg, i) => (
          <li
            key={`${i}-${seg.start_seconds}`}
            className="grid grid-cols-[56px_1fr] gap-3 rounded-md border border-border bg-card px-3 py-2"
          >
            <span className="pt-0.5 font-mono text-2xs text-muted-foreground">
              {formatTimestamp(seg.start_seconds)}
            </span>
            <p className="text-sm leading-relaxed">{seg.text}</p>
          </li>
        ))}
      </ol>
    </div>
  );
}

function formatTimestamp(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  return formatDuration(seconds);
}
