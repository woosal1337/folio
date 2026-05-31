import * as React from "react";
import { FileText, Headphones, Loader2, Mic } from "lucide-react";

import { readTranscript } from "@/shared/lib/ipc";
import { formatDuration } from "@/shared/lib/utils";
import type { ChannelTranscript } from "@/shared/types/ChannelTranscript";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";

interface Props {
  sessionDir: string;
}

/**
 * Read the transcript for a session and render its per-channel
 * segments. Loads lazily on mount so the recording row only pays the
 * IO cost when the user actually opens it.
 */
export function TranscriptView({ sessionDir }: Props) {
  const [transcript, setTranscript] = React.useState<SessionTranscript | null>(null);
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

  const totalSegments =
    transcript?.channels.reduce((acc, ch) => acc + ch.segments.length, 0) ?? 0;
  if (!transcript || totalSegments === 0) {
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

      {transcript.channels.map((channel) => (
        <ChannelSection key={channel.channel} channel={channel} />
      ))}
    </div>
  );
}

/** Pill colours cycled by speaker number (matches the editor). */
const SPEAKER_PILL_COLORS = [
  "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
  "bg-sky-500/15 text-sky-600 dark:text-sky-400",
  "bg-amber-500/15 text-amber-600 dark:text-amber-400",
  "bg-violet-500/15 text-violet-600 dark:text-violet-400",
  "bg-rose-500/15 text-rose-600 dark:text-rose-400",
  "bg-teal-500/15 text-teal-600 dark:text-teal-400",
  "bg-fuchsia-500/15 text-fuchsia-600 dark:text-fuchsia-400",
  "bg-lime-500/15 text-lime-600 dark:text-lime-400",
];

/** Raw diarizer cluster index → 1-based "Speaker N" by first appearance. */
function buildSpeakerLabels(
  segments: ChannelTranscript["segments"]
): Map<number, number> {
  const map = new Map<number, number>();
  for (const s of segments) {
    if (s.speaker !== null && !map.has(s.speaker)) {
      map.set(s.speaker, map.size + 1);
    }
  }
  return map;
}

function ChannelSection({ channel }: { channel: ChannelTranscript }) {
  const meta = channelLabel(channel.channel);
  const speakerMap = buildSpeakerLabels(channel.segments);

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-medium text-foreground">
          <meta.Icon className="h-3.5 w-3.5 text-muted-foreground" />
          {meta.label}
          <span className="text-2xs font-normal text-muted-foreground">{meta.sub}</span>
          {speakerMap.size > 0 && (
            <span className="text-2xs font-normal text-muted-foreground">
              · {speakerMap.size} {speakerMap.size === 1 ? "speaker" : "speakers"}
            </span>
          )}
        </div>
        {channel.language && (
          <span className="font-mono text-2xs text-muted-foreground">
            {channel.language}
          </span>
        )}
      </div>

      {channel.segments.length === 0 ? (
        <p className="text-2xs text-muted-foreground">
          No speech detected on this channel.
        </p>
      ) : (
        <ol className="flex flex-col gap-2">
          {channel.segments.map((seg, i) => {
            const num = seg.speaker !== null ? speakerMap.get(seg.speaker) : undefined;
            return (
              <li
                key={`${i}-${seg.start_seconds}`}
                className="grid grid-cols-[56px_1fr] gap-3 rounded-md border border-border bg-card px-3 py-2"
              >
                <span className="pt-0.5 font-mono text-2xs text-muted-foreground">
                  {formatTimestamp(seg.start_seconds)}
                </span>
                <div>
                  {num && (
                    <span
                      className={`mb-1 inline-flex items-center rounded-full px-2 py-0.5 text-2xs font-medium ${
                        SPEAKER_PILL_COLORS[(num - 1) % SPEAKER_PILL_COLORS.length]
                      }`}
                    >
                      Speaker {num}
                    </span>
                  )}
                  <p className="text-sm leading-relaxed">{seg.text}</p>
                </div>
              </li>
            );
          })}
        </ol>
      )}
    </section>
  );
}

interface ChannelMeta {
  label: string;
  sub: string;
  Icon: React.ComponentType<{ className?: string }>;
}

function channelLabel(channel: string): ChannelMeta {
  switch (channel) {
    case "mic":
      return { label: "You", sub: "(microphone)", Icon: Mic };
    case "system":
      return { label: "Others", sub: "(system audio)", Icon: Headphones };
    default:
      return { label: channel, sub: "", Icon: FileText };
  }
}

function formatTimestamp(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  return formatDuration(seconds);
}
