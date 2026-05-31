import * as React from "react";
import { FileText, Loader2, Users } from "lucide-react";

import {
  buildConversation,
  type ConversationRow,
  otherSpeakerLabels,
} from "@/shared/lib/conversation";
import { listSessionSpeakers, readTranscript } from "@/shared/lib/ipc";
import { formatDuration } from "@/shared/lib/utils";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";
import type { SpeakerLabel } from "@/shared/types/SpeakerLabel";

interface Props {
  sessionDir: string;
}

/**
 * Read the transcript for a session and render it as one chronological,
 * speaker-labelled conversation. Loads lazily on mount so the recording
 * row only pays the IO cost when the user actually opens it.
 */
export function TranscriptView({ sessionDir }: Props) {
  const [transcript, setTranscript] = React.useState<SessionTranscript | null>(null);
  const [speakerLabels, setSpeakerLabels] = React.useState<SpeakerLabel[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    (async () => {
      try {
        const t = await readTranscript(sessionDir);
        if (!cancelled) setTranscript(t);
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    // Speaker names are best-effort: show "Speaker N" if they don't load.
    void listSessionSpeakers(sessionDir)
      .then((labels) => {
        if (!cancelled) setSpeakerLabels(labels);
      })
      .catch((e) => console.error("list_session_speakers:", e));
    return () => {
      cancelled = true;
    };
  }, [sessionDir]);

  // cluster id → real name, applied over the default "Speaker N".
  const speakerNames = React.useMemo(() => {
    const m = new Map<number, string>();
    for (const l of speakerLabels) {
      if (l.name !== null) m.set(l.cluster, l.name);
    }
    return m;
  }, [speakerLabels]);

  // Merge mic + system into one chronological conversation. Computed
  // unconditionally (hooks rules) — empty until the transcript loads.
  const rows = React.useMemo(
    () => (transcript ? buildConversation(transcript, speakerNames) : []),
    [transcript, speakerNames]
  );

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

  if (rows.length === 0) {
    return (
      <p className="text-xs text-muted-foreground">
        Transcript is empty — Whisper returned no segments for this audio.
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <header className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
        <FileText className="h-3.5 w-3.5" />
        Transcript
      </header>

      <ConversationList rows={rows} />
    </div>
  );
}

function ConversationList({ rows }: { rows: ConversationRow[] }) {
  const speakers = otherSpeakerLabels(rows);

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center gap-2 text-xs font-medium text-foreground">
        <Users className="h-3.5 w-3.5 text-muted-foreground" />
        Conversation
        <span className="text-2xs font-normal text-muted-foreground">
          {speakers.length === 0
            ? "· you only"
            : `· you + ${speakers.length} ${
                speakers.length === 1 ? "speaker" : "speakers"
              }`}
        </span>
      </div>

      <ol className="flex flex-col gap-2">
        {rows.map((row) => (
          <li
            key={`${row.channelIndex}-${row.segmentIndex}-${row.segment.start_seconds}`}
            className="grid grid-cols-[56px_1fr] gap-3 rounded-md border border-border bg-card px-3 py-2"
          >
            <span className="pt-0.5 font-mono text-2xs text-muted-foreground">
              {formatTimestamp(row.segment.start_seconds)}
            </span>
            <div>
              <span
                className={`mb-1 inline-flex items-center rounded-full px-2 py-0.5 text-2xs font-medium ${row.pillClass}`}
              >
                {row.label}
              </span>
              <p className="text-sm leading-relaxed" dir="auto">
                {row.segment.text}
              </p>
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}

function formatTimestamp(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  return formatDuration(seconds);
}
