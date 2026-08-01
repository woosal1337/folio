import type { SessionTranscript } from "@/shared/types/SessionTranscript";
import type { TranscriptSegment } from "@/shared/types/TranscriptSegment";

export type ExportFormat = "srt" | "vtt" | "txt";

interface Cue {
  start: number;
  end: number;
  text: string;
  channel: string;
}

function flatten(transcript: SessionTranscript): Cue[] {
  const cues: Cue[] = [];
  for (const ch of transcript.channels) {
    for (const seg of ch.segments) {
      const text = seg.text.trim();
      if (text.length === 0) continue;
      cues.push({
        start: seg.start_seconds,
        end: seg.end_seconds,
        text,
        channel: ch.channel,
      });
    }
  }
  cues.sort((a, b) => a.start - b.start || a.end - b.end);
  return cues;
}

function speakerFor(channel: string): string {
  if (channel === "mic") return "You";
  if (channel === "system") return "Others";
  return channel;
}

function pad(n: number, width: number): string {
  return Math.floor(n).toString().padStart(width, "0");
}

export function srtTimestamp(seconds: number): string {
  const safe = Number.isFinite(seconds) && seconds > 0 ? seconds : 0;
  const total = Math.round(safe * 1000);
  const ms = total % 1000;
  const totalSeconds = Math.floor(total / 1000);
  const s = totalSeconds % 60;
  const m = Math.floor(totalSeconds / 60) % 60;
  const h = Math.floor(totalSeconds / 3600);
  return `${pad(h, 2)}:${pad(m, 2)}:${pad(s, 2)},${pad(ms, 3)}`;
}

export function vttTimestamp(seconds: number): string {
  return srtTimestamp(seconds).replace(",", ".");
}

export function txtTimestamp(seconds: number): string {
  return `[${vttTimestamp(seconds)}]`;
}

export function toSrt(transcript: SessionTranscript): string {
  const cues = flatten(transcript);
  const blocks = cues.map((c, i) => {
    const speaker = speakerFor(c.channel);
    const head = `${i + 1}`;
    const range = `${srtTimestamp(c.start)} --> ${srtTimestamp(c.end)}`;
    return `${head}\n${range}\n${speaker}: ${c.text}\n`;
  });
  return blocks.join("\n");
}

export function toVtt(transcript: SessionTranscript): string {
  const cues = flatten(transcript);
  const blocks = cues.map((c) => {
    const speaker = speakerFor(c.channel);
    const range = `${vttTimestamp(c.start)} --> ${vttTimestamp(c.end)}`;

    return `${range}\n<v ${speaker}>${c.text}\n`;
  });
  return `WEBVTT\n\n${blocks.join("\n")}`;
}

export function toTxt(transcript: SessionTranscript): string {
  const cues = flatten(transcript);
  return cues
    .map((c) => `${txtTimestamp(c.start)} ${speakerFor(c.channel)}: ${c.text}`)
    .join("\n");
}

export function renderTranscript(
  transcript: SessionTranscript,
  format: ExportFormat
): string {
  switch (format) {
    case "srt":
      return toSrt(transcript);
    case "vtt":
      return toVtt(transcript);
    case "txt":
      return toTxt(transcript);
  }
}

export function extensionFor(format: ExportFormat): string {
  return format;
}

export function segmentMatches(segment: TranscriptSegment, query: string): boolean {
  if (query.trim().length === 0) return true;
  return segment.text.toLowerCase().includes(query.toLowerCase());
}
