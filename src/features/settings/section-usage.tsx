import * as React from "react";
import { Loader2, RefreshCw, Wallet } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import { estimateChatCompletionCost } from "@/shared/lib/cost-models";
import { formatUsd } from "@/shared/lib/cost-estimate";
import { listAgentRuns, listRecordings } from "@/shared/lib/ipc";
import type { AgentRun } from "@/shared/types/AgentRun";

interface PerAgent {
  agentId: string;
  agentName: string;
  runs: number;
  usd: number;
  inputTokens: number;
  outputTokens: number;
}

interface PerDay {
  date: string;
  runs: number;
  usd: number;
}

interface UsageSummary {
  totalRuns: number;
  totalUsd: number;
  byAgent: PerAgent[];
  byDay: PerDay[];

  hasTokenData: boolean;
}

const DAYS = 30;

function isoDay(ts: string): string {
  return ts.slice(0, 10); // "2026-05-25T14:00:00Z" → "2026-05-25"
}

export function SectionUsage() {
  const [loading, setLoading] = React.useState(true);
  const [summary, setSummary] = React.useState<UsageSummary | null>(null);

  const refresh = React.useCallback(async () => {
    setLoading(true);
    try {
      const recordings = await listRecordings();
      const allRuns: AgentRun[] = [];

      const queue = [...recordings];
      const workers = Array.from({ length: 8 }, () => worker(queue, allRuns));
      await Promise.all(workers);
      setSummary(aggregate(allRuns));
    } catch (e) {
      console.error("usage refresh:", e);
      toast.error("Could not load usage", { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="font-serif text-2xl font-medium">Usage</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Token-based cost estimate across every agent run in your library. We
            multiply the provider-reported token counts by the published list price for
            each model. Local Whisper transcription has no LLM cost and is not counted
            here.
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={refresh}
          disabled={loading}
          className="gap-2"
        >
          {loading ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <RefreshCw className="h-3.5 w-3.5" />
          )}
          Refresh
        </Button>
      </div>

      {!summary ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : summary.totalRuns === 0 ? (
        <p className="rounded-lg border border-dashed border-border bg-card px-4 py-6 text-center text-xs text-muted-foreground">
          No agent runs yet. Run Summarize or Extract Tasks on a recording to start
          tracking usage.
        </p>
      ) : (
        <>
          <div className="grid grid-cols-2 gap-3">
            <StatCard
              icon={Wallet}
              label="Total estimate"
              value={formatUsd(summary.totalUsd)}
            />
            <StatCard
              icon={Wallet}
              label="Total runs"
              value={String(summary.totalRuns)}
            />
          </div>

          {!summary.hasTokenData && (
            <p className="rounded-md border border-amber-500/40 bg-amber-500/5 px-3 py-2 text-2xs text-muted-foreground">
              Your provider did not report token usage for these runs, so the estimate
              is $0. Set a provider that returns usage (the OpenAI chat-completions API
              does by default) to populate this view.
            </p>
          )}

          <DayBars days={summary.byDay} />

          <section aria-label="Cost by agent" className="flex flex-col gap-2">
            <h3 className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
              By agent
            </h3>
            <ul className="flex flex-col gap-1.5">
              {summary.byAgent.map((a) => (
                <li
                  key={a.agentId}
                  className="grid grid-cols-[1fr_auto_auto] items-baseline gap-3 rounded-md border border-border bg-card px-3 py-2"
                >
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">{a.agentName}</p>
                    <p className="font-mono text-2xs tabular-nums text-muted-foreground">
                      in {a.inputTokens.toLocaleString()} · out{" "}
                      {a.outputTokens.toLocaleString()}
                    </p>
                  </div>
                  <span className="text-2xs tabular-nums text-muted-foreground">
                    {a.runs} run{a.runs === 1 ? "" : "s"}
                  </span>
                  <span className="text-sm font-medium tabular-nums">
                    {formatUsd(a.usd)}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        </>
      )}
    </div>
  );
}

async function worker(queue: { session_dir: string }[], out: AgentRun[]) {
  while (queue.length > 0) {
    const item = queue.shift();
    if (!item) return;
    try {
      const runs = await listAgentRuns(item.session_dir);
      out.push(...runs);
    } catch (e) {
      console.info("listAgentRuns skipped:", item.session_dir, e);
    }
  }
}

function aggregate(runs: AgentRun[]): UsageSummary {
  let totalUsd = 0;
  let hasTokenData = false;
  const byAgent = new Map<string, PerAgent>();
  const byDay = new Map<string, PerDay>();

  for (const run of runs) {
    const pt = run.prompt_tokens ?? 0;
    const ct = run.completion_tokens ?? 0;
    if (pt > 0 || ct > 0) hasTokenData = true;
    const usd = estimateChatCompletionCost({
      model: run.model,
      promptTokens: run.prompt_tokens,
      completionTokens: run.completion_tokens,
    });
    totalUsd += usd;

    const a = byAgent.get(run.agent_id) ?? {
      agentId: run.agent_id,
      agentName: run.agent_name,
      runs: 0,
      usd: 0,
      inputTokens: 0,
      outputTokens: 0,
    };
    a.runs += 1;
    a.usd += usd;
    a.inputTokens += pt;
    a.outputTokens += ct;
    byAgent.set(run.agent_id, a);

    const day = isoDay(run.finished_at);
    const d = byDay.get(day) ?? { date: day, runs: 0, usd: 0 };
    d.runs += 1;
    d.usd += usd;
    byDay.set(day, d);
  }

  const days = Array.from(byDay.values())
    .sort((a, b) => a.date.localeCompare(b.date))
    .slice(-DAYS);
  const agents = Array.from(byAgent.values()).sort((a, b) => b.usd - a.usd);

  return {
    totalRuns: runs.length,
    totalUsd,
    byAgent: agents,
    byDay: days,
    hasTokenData,
  };
}

function StatCard({
  icon: Icon,
  label,
  value,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-lg border border-border bg-card px-4 py-3">
      <div className="flex items-center gap-2 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        <Icon className="h-3.5 w-3.5" />
        {label}
      </div>
      <p className="mt-1 text-2xl font-medium tabular-nums">{value}</p>
    </div>
  );
}

function DayBars({ days }: { days: PerDay[] }) {
  if (days.length === 0) return null;
  const max = Math.max(...days.map((d) => d.usd), 0.0001);
  return (
    <section
      aria-label="Daily cost (last 30 days)"
      className="flex flex-col gap-2 rounded-lg border border-border bg-card p-4"
    >
      <header className="flex items-center justify-between">
        <h3 className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
          Daily cost
        </h3>
        <span className="text-2xs tabular-nums text-muted-foreground">
          peak {formatUsd(max)}
        </span>
      </header>
      <div className="flex h-24 items-end gap-1">
        {days.map((d) => (
          <div
            key={d.date}
            className={cn(
              "flex-1 rounded-sm bg-primary/40 transition-colors hover:bg-primary",
              d.usd === 0 && "bg-muted"
            )}
            style={{
              height: `${Math.max(2, Math.round((d.usd / max) * 100))}%`,
            }}
            title={`${d.date} · ${formatUsd(d.usd)} · ${d.runs} run${d.runs === 1 ? "" : "s"}`}
          />
        ))}
      </div>
    </section>
  );
}
