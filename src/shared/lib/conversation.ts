import type { SessionTranscript } from "@/shared/types/SessionTranscript";
import type { TranscriptSegment } from "@/shared/types/TranscriptSegment";

/**
 * Distinct, theme-friendly pill colours cycled by speaker number. Kept in
 * sync with the agent-side labelling so "Speaker 2" is the same person in
 * the transcript and in the AI summary.
 */
export const SPEAKER_PILL_COLORS = [
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
 * The note-taker ("You") gets a fixed neutral pill, deliberately outside
 * the coloured participant palette so the user's own turns read at a
 * glance in the merged conversation.
 */
export const SELF_PILL_COLOR =
  "bg-neutral-500/15 text-neutral-600 dark:text-neutral-300";

/** Unattributed system audio / legacy single-channel transcripts. */
export const NEUTRAL_PILL_COLOR = "bg-muted text-muted-foreground";

export interface ConversationRow {
  segment: TranscriptSegment;
  /** Source channel id ("mic" / "system" / "legacy") — seeks the right player. */
  channelId: string;
  /** Index into `transcript.channels` — lets an editor route edits back. */
  channelIndex: number;
  /** Index into that channel's `segments`. */
  segmentIndex: number;
  /** Display label: "You", "Speaker 1", "Others", "Unknown speaker". */
  label: string;
  /** Tailwind classes for this speaker's pill. */
  pillClass: string;
  /** True for the note-taker's own ("You") turns. */
  isSelf: boolean;
}

/**
 * Flatten a multi-channel transcript into one chronological, speaker-
 * labelled conversation — the UI mirror of the Rust
 * `SessionTranscript::to_labeled_dialogue` the AI agents read, so the
 * transcript and the summary agree on who is "You" vs "Speaker N".
 *
 * - the mic channel is the note-taker → "You"
 * - system speakers become "Speaker 1/2/3…", numbered by first appearance
 * - un-diarized system audio falls back to "Others"; legacy single-channel
 *   audio → "Unknown speaker"
 *
 * Rows come back sorted by start time. Each row keeps its
 * (channelIndex, segmentIndex) so the editor can route an edit back to the
 * exact underlying segment.
 */
export function buildConversation(transcript: SessionTranscript): ConversationRow[] {
  // 1-based "Speaker N" per raw diarizer cluster index, by first
  // appearance across the system channel(s) in stored (time) order — the
  // same numbering the Rust side uses.
  const speakerNum = new Map<number, number>();
  transcript.channels.forEach((ch) => {
    if (ch.channel !== "system") return;
    ch.segments.forEach((s) => {
      if (s.speaker !== null && !speakerNum.has(s.speaker)) {
        speakerNum.set(s.speaker, speakerNum.size + 1);
      }
    });
  });

  const rows: ConversationRow[] = [];
  transcript.channels.forEach((ch, channelIndex) => {
    ch.segments.forEach((segment, segmentIndex) => {
      let label: string;
      let pillClass: string;
      let isSelf = false;
      if (ch.channel === "mic") {
        label = "You";
        pillClass = SELF_PILL_COLOR;
        isSelf = true;
      } else if (ch.channel === "system") {
        const n = segment.speaker !== null ? speakerNum.get(segment.speaker) : undefined;
        if (n !== undefined) {
          label = `Speaker ${n}`;
          pillClass =
            SPEAKER_PILL_COLORS[(n - 1) % SPEAKER_PILL_COLORS.length] ??
            NEUTRAL_PILL_COLOR;
        } else {
          label = "Others";
          pillClass = NEUTRAL_PILL_COLOR;
        }
      } else {
        label = "Unknown speaker";
        pillClass = NEUTRAL_PILL_COLOR;
      }
      rows.push({
        segment,
        channelId: ch.channel,
        channelIndex,
        segmentIndex,
        label,
        pillClass,
        isSelf,
      });
    });
  });

  // Stable sort by start time. Ties keep channel-then-segment order, which
  // matches the Rust formatter's stable merge.
  rows.sort((a, b) => a.segment.start_seconds - b.segment.start_seconds);
  return rows;
}

/**
 * Distinct non-"You" speaker labels present, in first-appearance order —
 * for a "N speakers" header summary. "You" is excluded (it's always the
 * note-taker, not a counted participant).
 */
export function otherSpeakerLabels(rows: ConversationRow[]): string[] {
  const seen: string[] = [];
  for (const r of rows) {
    if (r.isSelf) continue;
    if (!seen.includes(r.label)) seen.push(r.label);
  }
  return seen;
}
