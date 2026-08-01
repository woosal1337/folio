import * as React from "react";
import { useNavigate } from "react-router-dom";

import { listRecordings } from "@/shared/lib/ipc";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

interface DayCell {
  date: string;
  count: number;
  recordings: RecordingSummary[];
}

function toYMD(dateStr: string | null): string | null {
  if (!dateStr) return null;
  const d = new Date(dateStr);
  if (Number.isNaN(d.getTime())) return null;
  return d.toISOString().slice(0, 10);
}

function buildGrid(recordings: RecordingSummary[]): DayCell[][] {
  const byDay = new Map<string, RecordingSummary[]>();
  for (const r of recordings) {
    const ymd = toYMD(r.created_at ? String(r.created_at) : null);
    if (!ymd) continue;
    const arr = byDay.get(ymd) ?? [];
    arr.push(r);
    byDay.set(ymd, arr);
  }

  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const start = new Date(today);
  start.setDate(start.getDate() - 7 * 51 - today.getDay());

  const cols: DayCell[][] = [];
  const cursor = new Date(start);
  for (let w = 0; w < 52; w++) {
    const week: DayCell[] = [];
    for (let d = 0; d < 7; d++) {
      const ymd = cursor.toISOString().slice(0, 10);
      const recs = byDay.get(ymd) ?? [];
      week.push({ date: ymd, count: recs.length, recordings: recs });
      cursor.setDate(cursor.getDate() + 1);
    }
    cols.push(week);
  }
  return cols;
}

function cellColor(count: number, maxCount: number): string {
  if (count === 0) return "bg-muted/50";
  const ratio = count / Math.max(maxCount, 1);
  if (ratio < 0.25) return "bg-primary/20";
  if (ratio < 0.5) return "bg-primary/40";
  if (ratio < 0.75) return "bg-primary/70";
  return "bg-primary";
}

function formatDate(ymd: string): string {
  const d = new Date(ymd + "T00:00:00");
  return d.toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" });
}

const MONTH_LABELS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];
const DAY_LABELS = ["S", "M", "T", "W", "T", "F", "S"];

interface StatsResult {
  total: number;
  avgPerWeek: string;
  busiestDay: string | null;
  busiestDayCount: number;
  longestStreak: number;
}

function computeStats(recordings: RecordingSummary[]): StatsResult {
  const byDay = new Map<string, number>();
  for (const r of recordings) {
    const ymd = toYMD(r.created_at ? String(r.created_at) : null);
    if (!ymd) continue;
    byDay.set(ymd, (byDay.get(ymd) ?? 0) + 1);
  }
  const total = recordings.length;

  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const twelveWeeksAgo = new Date(today);
  twelveWeeksAgo.setDate(twelveWeeksAgo.getDate() - 84);
  let recentCount = 0;
  for (const r of recordings) {
    const ymd = toYMD(r.created_at ? String(r.created_at) : null);
    if (ymd && ymd >= twelveWeeksAgo.toISOString().slice(0, 10)) recentCount++;
  }
  const avgPerWeek = (recentCount / 12).toFixed(1);

  let busiestDay: string | null = null;
  let busiestDayCount = 0;
  for (const [ymd, count] of byDay.entries()) {
    if (count > busiestDayCount) {
      busiestDayCount = count;
      busiestDay = ymd;
    }
  }

  let longestStreak = 0;
  let streak = 0;
  const d = new Date(today);
  while (true) {
    const ymd = d.toISOString().slice(0, 10);
    if ((byDay.get(ymd) ?? 0) > 0) {
      streak++;
      longestStreak = Math.max(longestStreak, streak);
    } else {
      break;
    }
    d.setDate(d.getDate() - 1);
    if (streak > 365) break;
  }

  return { total, avgPerWeek, busiestDay, busiestDayCount, longestStreak };
}

export default function StatsRoute() {
  const navigate = useNavigate();
  const [recordings, setRecordings] = React.useState<RecordingSummary[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [tooltip, setTooltip] = React.useState<{
    cell: DayCell;
    x: number;
    y: number;
  } | null>(null);

  React.useEffect(() => {
    listRecordings()
      .then(setRecordings)
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const grid = React.useMemo(() => buildGrid(recordings), [recordings]);
  const maxCount = React.useMemo(
    () => Math.max(1, ...grid.flat().map((c) => c.count)),
    [grid]
  );
  const stats = React.useMemo(() => computeStats(recordings), [recordings]);

  const monthLabels = React.useMemo(() => {
    const labels: { col: number; label: string }[] = [];
    let lastMonth = -1;
    grid.forEach((week, w) => {
      const firstDay = week[0];
      if (!firstDay) return;
      const d = new Date(firstDay.date + "T00:00:00");
      const m = d.getMonth();
      if (m !== lastMonth) {
        labels.push({ col: w, label: MONTH_LABELS[m] ?? "" });
        lastMonth = m;
      }
    });
    return labels;
  }, [grid]);

  const openDay = (cell: DayCell) => {
    if (cell.recordings.length === 0) return;
    if (cell.recordings.length === 1) {
      const r = cell.recordings[0];
      if (!r) return;
      navigate(`/editor/${encodeURIComponent(r.label)}`, { state: { recording: r } });
    } else {
      navigate("/");
    }
  };

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 py-8">
      <header data-drag="" className="flex select-none items-baseline justify-between">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">
            Meeting history
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Your meeting cadence — local, private, yours.
          </p>
        </div>
      </header>

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {[
          { label: "Total meetings", value: String(stats.total) },
          { label: "Per week (12w avg)", value: stats.avgPerWeek },
          {
            label: "Busiest day",
            value: stats.busiestDay ? `${stats.busiestDayCount} meetings` : "—",
            sub: stats.busiestDay ? formatDate(stats.busiestDay) : undefined,
          },
          {
            label: "Current streak",
            value: stats.longestStreak > 0 ? `${stats.longestStreak}d` : "—",
          },
        ].map((s) => (
          <div
            key={s.label}
            className="rounded-xl border border-border bg-card px-4 py-3"
          >
            <p className="text-2xs uppercase tracking-wider text-muted-foreground">
              {s.label}
            </p>
            <p className="mt-1 font-serif text-2xl font-medium">{s.value}</p>
            {s.sub ? <p className="text-2xs text-muted-foreground">{s.sub}</p> : null}
          </div>
        ))}
      </div>

      <section className="space-y-2">
        <div className="overflow-x-auto">
          {loading ? (
            <div className="flex h-32 items-center justify-center text-sm text-muted-foreground">
              Loading…
            </div>
          ) : (
            <div className="relative inline-block">
              <div className="mb-1 flex gap-[3px] pl-5">
                {grid.map((_, w) => {
                  const label = monthLabels.find((ml) => ml.col === w);
                  return (
                    <div key={w} className="w-[11px] text-2xs text-muted-foreground">
                      {label ? label.label : ""}
                    </div>
                  );
                })}
              </div>

              <div className="flex gap-[3px]">
                <div className="mr-1 flex flex-col gap-[3px]">
                  {DAY_LABELS.map((d, i) => (
                    <div
                      key={i}
                      className="flex h-[11px] items-center text-2xs leading-none text-muted-foreground"
                    >
                      {i % 2 === 1 ? d : ""}
                    </div>
                  ))}
                </div>

                {grid.map((week, w) => (
                  <div key={w} className="flex flex-col gap-[3px]">
                    {week.map((cell) => (
                      <button
                        key={cell.date}
                        type="button"
                        aria-label={`${formatDate(cell.date)}: ${cell.count} meeting${cell.count !== 1 ? "s" : ""}`}
                        onClick={() => openDay(cell)}
                        onMouseEnter={(e) => {
                          const rect = (
                            e.target as HTMLElement
                          ).getBoundingClientRect();
                          setTooltip({ cell, x: rect.left, y: rect.top });
                        }}
                        onMouseLeave={() => setTooltip(null)}
                        className={`h-[11px] w-[11px] rounded-[2px] transition-opacity hover:opacity-80 ${cellColor(cell.count, maxCount)} ${cell.count > 0 ? "cursor-pointer" : "cursor-default"}`}
                      />
                    ))}
                  </div>
                ))}
              </div>

              <div className="mt-2 flex items-center gap-1.5 text-2xs text-muted-foreground">
                <span>Less</span>
                {[0, 0.2, 0.5, 0.75, 1].map((r) => (
                  <div
                    key={r}
                    className={`h-[11px] w-[11px] rounded-[2px] ${cellColor(Math.round(r * maxCount), maxCount)}`}
                  />
                ))}
                <span>More</span>
              </div>
            </div>
          )}
        </div>
      </section>

      {tooltip ? (
        <div
          className="pointer-events-none fixed z-50 rounded-md border border-border bg-popover px-2.5 py-1.5 text-2xs shadow-md"
          style={{ left: tooltip.x + 16, top: tooltip.y - 8 }}
        >
          <p className="font-medium">{formatDate(tooltip.cell.date)}</p>
          <p className="text-muted-foreground">
            {tooltip.cell.count === 0
              ? "No meetings"
              : `${tooltip.cell.count} meeting${tooltip.cell.count !== 1 ? "s" : ""}`}
          </p>
        </div>
      ) : null}
    </div>
  );
}
