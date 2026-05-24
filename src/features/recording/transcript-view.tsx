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

function ChannelSection({ channel }: { channel: ChannelTranscript }) {
  const meta = channelLabel(channel.channel);

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-medium text-foreground">
          <meta.Icon className="h-3.5 w-3.5 text-muted-foreground" />
          {meta.label}
          <span className="text-2xs font-normal text-muted-foreground">{meta.sub}</span>
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
          {channel.segments.map((seg, i) => (
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
