import * as React from "react";

import { formatDuration } from "@/shared/lib/utils";
import { cn } from "@/shared/lib/utils";
import type { TranscriptSegment } from "@/shared/types/TranscriptSegment";

interface Props {
  segment: TranscriptSegment;
  index: number;
  onChange: (next: string) => void;
}

/**
 * Single transcript segment. The timestamp gutter is static (we don't
 * try to recompute timestamps as text edits happen); the right column
 * is an auto-growing textarea that the user can edit in place.
 */
export function SegmentRow({ segment, index, onChange }: Props) {
  const textareaRef = React.useRef<HTMLTextAreaElement | null>(null);

  // Auto-grow the textarea on every change so the row matches the
  // visible text height.
  React.useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [segment.text]);

  const start = formatTimestamp(segment.start_seconds);

  return (
    <li
      className={cn(
        "grid grid-cols-[68px_1fr] gap-3 rounded-md border border-border bg-card px-3 py-2",
        "focus-within:border-primary"
      )}
      aria-label={`Segment ${index + 1} at ${start}`}
    >
      <span className="pt-2 font-mono text-2xs text-muted-foreground" aria-hidden>
        {start}
      </span>
      <textarea
        ref={textareaRef}
        value={segment.text}
        onChange={(e) => onChange(e.target.value)}
        rows={1}
        spellCheck
        aria-label={`Segment ${index + 1} text`}
        className={cn(
          "w-full resize-none bg-transparent text-sm leading-relaxed outline-none",
          "placeholder:text-muted-foreground"
        )}
        placeholder="(empty segment)"
      />
    </li>
  );
}

function formatTimestamp(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  return formatDuration(seconds);
}
