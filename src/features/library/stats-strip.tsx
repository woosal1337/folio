import { Card, CardContent } from "@/shared/ui/card";
import { formatDuration } from "@/shared/lib/utils";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

interface Props {
  recordings: RecordingSummary[];
}

/**
 * Aggregate counters over the recordings list. Cheap derived state —
 * recomputed on every render but the inputs are short (we don't expect
 * thousands of recordings).
 */
export function StatsStrip({ recordings }: Props) {
  const count = recordings.length;
  const transcribed = recordings.filter((r) => r.has_transcript).length;
  const totalSeconds = recordings.reduce(
    (acc, r) => acc + Number(r.duration_seconds),
    0
  );

  return (
    <Card>
      <CardContent className="grid grid-cols-3 gap-6 py-4 text-center">
        <Stat label="Recordings" value={count.toString()} />
        <Stat label="Transcribed" value={`${transcribed} / ${count}`} />
        <Stat label="Total duration" value={formatDuration(totalSeconds)} />
      </CardContent>
    </Card>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </span>
      <span className="font-mono text-lg tracking-tight">{value}</span>
    </div>
  );
}
