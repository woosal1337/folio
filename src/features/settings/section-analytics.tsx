import * as React from "react";
import { Brain, CheckSquare, Clock, FileAudio, Lightbulb, Lock } from "lucide-react";

import { Label } from "@/shared/ui/label";
import { listMemories, listRecordings, listTasks } from "@/shared/lib/ipc";
import { humanizeError } from "@/shared/lib/errors";
import type { Memory } from "@/shared/types/Memory";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";
import type { Task } from "@/shared/types/Task";

type Range = "7d" | "30d" | "90d" | "all";

const RANGES: { id: Range; label: string; days: number | null }[] = [
  { id: "7d", label: "Last 7 days", days: 7 },
  { id: "30d", label: "Last 30 days", days: 30 },
  { id: "90d", label: "Last 90 days", days: 90 },
  { id: "all", label: "All time", days: null },
];

export function SectionAnalytics() {
  const [range, setRange] = React.useState<Range>("30d");
  const [recordings, setRecordings] = React.useState<RecordingSummary[]>([]);
  const [tasks, setTasks] = React.useState<Task[]>([]);
  const [memories, setMemories] = React.useState<Memory[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [r, t, m] = await Promise.all([
          listRecordings(),
          listTasks(),
          listMemories({
            query: null,
            kinds: ["observe", "claim", "pref", "person"],
            include_archived: false,
            limit: null,
          }),
        ]);
        if (cancelled) return;
        setRecordings(r);
        setTasks(t);
        setMemories(m);
      } catch (e) {
        if (!cancelled) setError(humanizeError(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const stats = React.useMemo(() => {
    const days = RANGES.find((r) => r.id === range)?.days ?? null;
    const cutoff = days === null ? 0 : Date.now() - days * 86_400_000;
    const inRange = (iso: string | null) =>
      days === null ? true : iso !== null && new Date(iso).getTime() >= cutoff;

    const meetings = recordings.filter(
      (r) => (r.mic_bytes !== null || r.system_bytes !== null) && inRange(r.created_at)
    );
    const totalMinutes = Math.round(
      meetings.reduce((acc, r) => acc + Number(r.duration_seconds), 0) / 60
    );
    return {
      meetings: meetings.length,
      totalMinutes,
      actionItems: tasks.filter((t) => inRange(t.created_at)).length,
      decisions: memories.filter((m) => m.kind === "claim" && inRange(m.created_at))
        .length,
      memories: memories.filter((m) => inRange(m.created_at)).length,
    };
  }, [range, recordings, tasks, memories]);

  const fmt = (n: number) => (loading ? "—" : n.toLocaleString());

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Analytics</h2>
        <p className="text-sm text-muted-foreground">
          Your own activity totals, computed on this Mac. No surveillance — we
          don&apos;t score attention, engagement, or talk-time.
        </p>
      </header>

      <div className="flex flex-wrap gap-1.5">
        {RANGES.map((r) => (
          <button
            key={r.id}
            type="button"
            onClick={() => setRange(r.id)}
            aria-pressed={range === r.id}
            className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
              range === r.id
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground hover:bg-muted/70"
            }`}
          >
            {r.label}
          </button>
        ))}
      </div>

      <Group title="Activity">
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <Stat
            icon={FileAudio}
            label="Meetings recorded"
            value={fmt(stats.meetings)}
          />
          <Stat icon={Clock} label="Total minutes" value={fmt(stats.totalMinutes)} />
          <Stat
            icon={CheckSquare}
            label="Action items created"
            value={fmt(stats.actionItems)}
          />
          <Stat
            icon={Lightbulb}
            label="Decisions captured"
            value={fmt(stats.decisions)}
          />
          <Stat icon={Brain} label="Memories captured" value={fmt(stats.memories)} />
        </div>
        {error ? (
          <p className="text-2xs text-destructive">{error}</p>
        ) : (
          <p className="text-2xs text-muted-foreground">
            Counted from your local recordings, tasks, and memories.
          </p>
        )}
      </Group>

      <RejectedFeatureCard />
    </section>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-3">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {title}
      </Label>
      {children}
    </div>
  );
}

function Stat({
  icon: Icon,
  label,
  value,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="flex items-center gap-2 text-muted-foreground">
        <Icon className="h-3.5 w-3.5" />
        <p className="text-2xs uppercase tracking-wider">{label}</p>
      </div>
      <p className="mt-2 font-serif text-2xl font-medium tabular-nums">{value}</p>
    </div>
  );
}

function RejectedFeatureCard() {
  return (
    <div className="flex items-start gap-3 rounded-lg border border-border bg-muted/30 p-4">
      <Lock className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1 space-y-0.5">
        <p className="text-sm font-medium">No engagement scoring</p>
        <p className="max-w-prose text-xs text-muted-foreground">
          Folio does not compute per-person talk-time, attention, or engagement scores.
          Meeting analytics that surveil individuals are out of scope by policy — the
          only counts you&apos;ll ever see are your own totals.
        </p>
      </div>
    </div>
  );
}
