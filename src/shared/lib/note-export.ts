import type { AgentRun } from "@/shared/types/AgentRun";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

function formatDuration(secs: number | bigint | null | undefined): string {
  const n = Number(secs ?? 0);
  if (!Number.isFinite(n) || n <= 0) return "—";
  const h = Math.floor(n / 3600);
  const m = Math.floor((n % 3600) / 60);
  const s = Math.floor(n % 60);
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

function participantCount(recording: RecordingSummary): number {
  let n = 0;
  if (recording.mic_bytes && Number(recording.mic_bytes) > 0) n += 1;
  if (recording.system_bytes && Number(recording.system_bytes) > 0) n += 1;
  return n;
}

interface SerialiseInput {
  recording: RecordingSummary;
  summary: AgentRun | null;
  tasks: AgentRun | null;
  memories: AgentRun | null;
}

export function serialiseAsPlainText({
  recording,
  summary,
  tasks,
  memories,
}: SerialiseInput): string {
  const title = recording.suggested_title?.trim() || recording.label;
  const lines: string[] = [
    title,
    `Duration: ${formatDuration(recording.duration_seconds)} · ${participantCount(recording)} channel(s)`,
    "",
  ];
  if (summary) lines.push("## Summary", "", summary.response.trim(), "");
  if (tasks) lines.push("## Action items", "", tasks.response.trim(), "");
  if (memories) lines.push("## Memories", "", memories.response.trim(), "");
  return lines.join("\n");
}
