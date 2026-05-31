import * as React from "react";

import { formatDuration } from "@/shared/lib/utils";
import { cn } from "@/shared/lib/utils";
import type { TranscriptSegment } from "@/shared/types/TranscriptSegment";

import { dispatchSeekAudio } from "./seek-audio";

interface Props {
  segment: TranscriptSegment;
  index: number;
  /** Channel id ("mic" / "system") so the timestamp jumps the right player. */
  channel: string;
  /** Current search query; non-empty values highlight in-line. */
  query?: string;
  /** Diarized speaker label ("Speaker 1/2/3…"), when known. GET-189. */
  speakerLabel?: string;
  /** 1-based display number, used to colour the speaker pill. */
  speakerNumber?: number;
  onChange: (next: string) => void;
}

/** Distinct, theme-friendly pill colours cycled by speaker number. */
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

/**
 * Single transcript segment. The timestamp gutter is a click-to-seek
 * button that fires a window-level event the audio players subscribe
 * to (see `dispatchSeekAudio`). The right column is an auto-growing
 * textarea editable in place; when a search query is active the
 * matching substring is rendered behind the textarea as a yellow
 * highlight so the eye can find it without disturbing the editable
 * surface.
 */
export function SegmentRow({
  segment,
  index,
  channel,
  query,
  speakerLabel,
  speakerNumber,
  onChange,
}: Props) {
  const textareaRef = React.useRef<HTMLTextAreaElement | null>(null);

  React.useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [segment.text]);

  const start = formatTimestamp(segment.start_seconds);

  const handleSeek = () => {
    dispatchSeekAudio({ channel, seconds: segment.start_seconds });
  };

  const highlight = React.useMemo(
    () => buildHighlightSpans(segment.text, query ?? ""),
    [segment.text, query]
  );

  return (
    <li
      className={cn(
        "grid grid-cols-[68px_1fr] gap-3 rounded-md border border-border bg-card px-3 py-2",
        "focus-within:border-primary"
      )}
      aria-label={`Segment ${index + 1} at ${start}`}
    >
      <button
        type="button"
        onClick={handleSeek}
        className={cn(
          "self-start pt-2 text-left font-mono text-2xs text-muted-foreground transition-colors",
          "hover:text-primary focus:outline-none focus-visible:text-primary"
        )}
        title={`Jump audio to ${start}`}
        aria-label={`Jump audio to ${start}`}
      >
        {start}
      </button>
      <div className="relative">
        {speakerLabel && (
          <span
            className={cn(
              "mb-1 inline-flex items-center rounded-full px-2 py-0.5 text-2xs font-medium",
              SPEAKER_PILL_COLORS[
                ((speakerNumber ?? 1) - 1) % SPEAKER_PILL_COLORS.length
              ]
            )}
          >
            {speakerLabel}
          </span>
        )}
        {highlight && (
          <div
            aria-hidden
            className="pointer-events-none absolute inset-0 whitespace-pre-wrap break-words text-sm leading-relaxed text-transparent"
          >
            {highlight}
          </div>
        )}
        <textarea
          ref={textareaRef}
          value={segment.text}
          onChange={(e) => onChange(e.target.value)}
          rows={1}
          spellCheck
          dir="auto"
          aria-label={`Segment ${index + 1} text`}
          className={cn(
            "relative w-full resize-none bg-transparent text-sm leading-relaxed outline-none",
            "placeholder:text-muted-foreground"
          )}
          placeholder="(empty segment)"
        />
      </div>
    </li>
  );
}

function formatTimestamp(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  return formatDuration(seconds);
}

/**
 * Build the highlight overlay. Returns `null` when the query is empty
 * so the row skips the overlay entirely (cheap no-op for the common
 * unfiltered render).
 */
function buildHighlightSpans(text: string, query: string): React.ReactNode {
  const needle = query.trim();
  if (needle.length === 0) return null;
  const lowerHay = text.toLowerCase();
  const lowerNeedle = needle.toLowerCase();
  const out: React.ReactNode[] = [];
  let cursor = 0;
  while (cursor < text.length) {
    const hit = lowerHay.indexOf(lowerNeedle, cursor);
    if (hit === -1) {
      out.push(text.slice(cursor));
      break;
    }
    if (hit > cursor) out.push(text.slice(cursor, hit));
    out.push(
      <mark
        key={`${hit}-${needle.length}`}
        className="rounded-sm bg-yellow-200/70 text-transparent dark:bg-yellow-500/30"
      >
        {text.slice(hit, hit + needle.length)}
      </mark>
    );
    cursor = hit + needle.length;
  }
  return out;
}
