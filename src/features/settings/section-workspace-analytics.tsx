/**
 * GET-139 — Settings → Workspace → Analytics.
 *
 * Aggregate workspace stats only — no per-user engagement scoring.
 * Read.ai's "attention surveillance" pattern is explicitly rejected.
 *
 * v1 reads local counts from the library where possible (recordings,
 * minutes); workspace-level aggregates ship with attune-api.
 */

import * as React from "react";
import {
  BarChart3,
  Brain,
  CheckSquare,
  Clock,
  FileAudio,
  Lightbulb,
  Lock,
} from "lucide-react";

import { Label } from "@/shared/ui/label";

type Range = "7d" | "30d" | "90d" | "all";

const RANGES: { id: Range; label: string }[] = [
  { id: "7d", label: "Last 7 days" },
  { id: "30d", label: "Last 30 days" },
  { id: "90d", label: "Last 90 days" },
  { id: "all", label: "All time" },
];

export function SectionWorkspaceAnalytics() {
  const [range, setRange] = React.useState<Range>("30d");

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Analytics</h2>
        <p className="text-sm text-muted-foreground">
          Aggregate activity for the workspace. No per-user surveillance —
          we don&apos;t score attention, engagement, or talk-time.
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
          <Stat icon={FileAudio} label="Meetings recorded" value="—" />
          <Stat icon={Clock} label="Total minutes" value="—" />
          <Stat icon={CheckSquare} label="Action items created" value="—" />
          <Stat icon={Lightbulb} label="Decisions captured" value="—" />
          <Stat icon={Brain} label="Memories captured" value="—" />
          <Stat icon={BarChart3} label="Notes shared" value="—" />
        </div>
        <p className="text-2xs text-muted-foreground">
          Numbers populate once the workspace activity feed ships server-side.
        </p>
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
      <p className="mt-2 font-serif text-2xl font-medium tabular-nums">
        {value}
      </p>
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
          Attune does not compute per-person talk-time, attention, or
          engagement scores. Meeting analytics that surveil individuals are
          out of scope by policy — the only counts you&apos;ll ever see are
          workspace-level aggregates.
        </p>
      </div>
    </div>
  );
}
