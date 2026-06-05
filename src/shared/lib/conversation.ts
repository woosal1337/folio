import type { SessionTranscript } from "@/shared/types/SessionTranscript";
import type { TranscriptSegment } from "@/shared/types/TranscriptSegment";

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

export const SELF_PILL_COLOR =
  "bg-neutral-500/15 text-neutral-600 dark:text-neutral-300";

export const NEUTRAL_PILL_COLOR = "bg-muted text-muted-foreground";

export interface ConversationRow {
  segment: TranscriptSegment;

  channelId: string;

  channelIndex: number;

  segmentIndex: number;

  cluster: number | null;

  speakerNumber?: number;

  label: string;

  pillClass: string;

  isSelf: boolean;
}

export interface ConversationSpeaker {
  cluster: number;

  number: number;

  label: string;

  named: boolean;

  pillClass: string;
}

export function buildConversation(
  transcript: SessionTranscript,
  names?: Map<number, string>
): ConversationRow[] {
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
      let cluster: number | null = null;
      let speakerNumber: number | undefined;
      if (ch.channel === "mic") {
        label = "You";
        pillClass = SELF_PILL_COLOR;
        isSelf = true;
      } else if (ch.channel === "system") {
        const n =
          segment.speaker !== null ? speakerNum.get(segment.speaker) : undefined;
        if (n !== undefined && segment.speaker !== null) {
          cluster = segment.speaker;
          speakerNumber = n;
          label = names?.get(segment.speaker) ?? `Speaker ${n}`;
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
        cluster,
        speakerNumber,
        label,
        pillClass,
        isSelf,
      });
    });
  });

  rows.sort((a, b) => a.segment.start_seconds - b.segment.start_seconds);
  return rows;
}

export function conversationSpeakers(rows: ConversationRow[]): ConversationSpeaker[] {
  const seen = new Map<number, ConversationSpeaker>();
  for (const r of rows) {
    if (r.cluster === null || r.speakerNumber === undefined) continue;
    if (seen.has(r.cluster)) continue;
    seen.set(r.cluster, {
      cluster: r.cluster,
      number: r.speakerNumber,
      label: r.label,
      named: r.label !== `Speaker ${r.speakerNumber}`,
      pillClass: r.pillClass,
    });
  }
  return [...seen.values()].sort((a, b) => a.number - b.number);
}

export function conversationLanguages(rows: ConversationRow[]): string[] {
  const seen: string[] = [];
  for (const r of rows) {
    const lang = r.segment.language;
    if (lang && !seen.includes(lang)) seen.push(lang);
  }
  return seen;
}

export function otherSpeakerLabels(rows: ConversationRow[]): string[] {
  const seen: string[] = [];
  for (const r of rows) {
    if (r.isSelf) continue;
    if (!seen.includes(r.label)) seen.push(r.label);
  }
  return seen;
}
