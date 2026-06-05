import * as React from "react";
import { CornerDownRight, Loader2, X } from "lucide-react";

import { Markdown } from "@/shared/ui/markdown";
import { cn, formatDuration } from "@/shared/lib/utils";
import { locateNoteEvidence } from "@/shared/lib/ipc";
import type { TranscriptHit } from "@/shared/types/TranscriptHit";

import { dispatchSeekAudio } from "./seek-audio";

interface Props {
  response: string;
  sessionDir: string;

  muted: boolean;
}

interface Active {
  line: string;
  hit: TranscriptHit | null;
  loading: boolean;
}

export function EnhancedNotesBody({ response, sessionDir, muted }: Props) {
  const containerRef = React.useRef<HTMLDivElement>(null);
  const [active, setActive] = React.useState<Active | null>(null);

  const onClick = React.useCallback(
    async (e: React.MouseEvent<HTMLDivElement>) => {
      const block = (e.target as HTMLElement).closest(
        "li, p, h1, h2, h3, h4, h5, h6, blockquote"
      );
      if (!block || !containerRef.current?.contains(block)) return;
      const line = (block.textContent ?? "").trim();

      if (line.length < 8) return;

      setActive({ line, hit: null, loading: true });
      try {
        const hit = await locateNoteEvidence(sessionDir, line);
        setActive({ line, hit, loading: false });
        if (hit) {
          dispatchSeekAudio({ channel: hit.channel, seconds: hit.start_seconds });
        }
      } catch (err) {
        console.error("locate_note_evidence:", err);
        setActive(null);
      }
    },
    [sessionDir]
  );

  return (
    <div className="space-y-2">
      {/* eslint-disable-next-line jsx-a11y/no-static-element-interactions, jsx-a11y/click-events-have-key-events */}
      <div
        ref={containerRef}
        onClick={(e) => void onClick(e)}
        className={cn(
          "prose prose-sm prose-neutral dark:prose-invert max-w-none transition-opacity",

          "[&_li]:cursor-pointer [&_li]:rounded [&_p]:cursor-pointer [&_p]:rounded",
          "[&_li:hover]:bg-accent/40 [&_p:hover]:bg-accent/40",
          muted ? "opacity-60" : ""
        )}
      >
        <Markdown>{response}</Markdown>
      </div>

      {active && <EvidenceCard active={active} onClose={() => setActive(null)} />}
    </div>
  );
}

function EvidenceCard({ active, onClose }: { active: Active; onClose: () => void }) {
  const { hit, loading } = active;
  return (
    <div className="rounded-md border border-border bg-card/60 p-3 text-xs">
      <div className="flex items-start justify-between gap-2">
        <span className="flex items-center gap-1.5 font-medium text-muted-foreground">
          <CornerDownRight className="h-3.5 w-3.5" />
          {loading
            ? "Finding the source…"
            : hit
              ? "From the transcript"
              : "No clear source"}
        </span>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close"
          className="inline-flex h-5 w-5 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      {loading ? (
        <div className="mt-1 flex items-center gap-2 text-muted-foreground">
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          <span className="truncate">{active.line}</span>
        </div>
      ) : hit ? (
        <button
          type="button"
          onClick={() =>
            dispatchSeekAudio({ channel: hit.channel, seconds: hit.start_seconds })
          }
          className="mt-1.5 flex w-full items-start gap-2 rounded text-left hover:bg-accent/40"
          title="Jump audio to this moment"
        >
          <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-2xs text-muted-foreground">
            {formatTimestamp(hit.start_seconds)} · {channelLabel(hit.channel)}
          </span>
          <span className="leading-relaxed text-foreground" dir="auto">
            {hit.matched_text}
          </span>
        </button>
      ) : (
        <p className="mt-1 text-muted-foreground">
          This line paraphrases the meeting but doesn&apos;t map to one specific moment
          — likely synthesized across the discussion.
        </p>
      )}
    </div>
  );
}

function channelLabel(channel: string): string {
  switch (channel) {
    case "mic":
      return "You";
    case "system":
      return "Others";
    default:
      return channel;
  }
}

function formatTimestamp(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  return formatDuration(seconds);
}
