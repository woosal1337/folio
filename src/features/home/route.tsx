/**
 * GET-151 — Home.
 *
 * The signed-in landing surface, replacing the bare Record page:
 *   - a "Coming up" card (next calendar event + Take Notes),
 *   - recent notes grouped Today / Yesterday / Earlier,
 *   - a + Quick note action.
 *
 * Recording becomes a verb (sidebar + Cmd-R + the meeting HUD), not the
 * home tab. The EventKit next-event reader is still deferred (GET-134),
 * so "Coming up" shows a graceful empty state for now; the Take Notes
 * affordance works regardless.
 */

import * as React from "react";
import { useNavigate } from "react-router-dom";
import { CalendarClock, FileAudio, Mic, Plus } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { listRecordings } from "@/shared/lib/ipc";
import { useQuickNote, useTakeNotes } from "@/shared/hooks/use-take-notes";
import { AskBar } from "@/chrome/ask-bar";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

type Group = "Today" | "Yesterday" | "Earlier";

function groupFor(createdAt: string | null): Group {
  if (!createdAt) return "Earlier";
  const d = new Date(createdAt);
  if (Number.isNaN(d.getTime())) return "Earlier";
  const startOfToday = new Date();
  startOfToday.setHours(0, 0, 0, 0);
  const startOfYesterday = new Date(startOfToday);
  startOfYesterday.setDate(startOfYesterday.getDate() - 1);
  if (d >= startOfToday) return "Today";
  if (d >= startOfYesterday) return "Yesterday";
  return "Earlier";
}

function timeLabel(createdAt: string | null): string {
  if (!createdAt) return "";
  const d = new Date(createdAt);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

export default function Home() {
  const navigate = useNavigate();
  const takeNotes = useTakeNotes();
  const quickNote = useQuickNote();
  const [recordings, setRecordings] = React.useState<RecordingSummary[]>([]);
  const [loading, setLoading] = React.useState(true);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const rs = await listRecordings();
        if (!cancelled) setRecordings(rs);
      } catch (e) {
        console.error("home: listRecordings failed", e);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const groups = React.useMemo(() => {
    const buckets: Record<Group, RecordingSummary[]> = {
      Today: [],
      Yesterday: [],
      Earlier: [],
    };
    for (const r of recordings) buckets[groupFor(r.created_at)].push(r);
    return (["Today", "Yesterday", "Earlier"] as Group[])
      .map((g) => ({ group: g, items: buckets[g] }))
      .filter((b) => b.items.length > 0);
  }, [recordings]);

  const openNote = React.useCallback(
    (r: RecordingSummary) => {
      navigate(`/editor/${encodeURIComponent(r.label)}`, { state: { recording: r } });
    },
    [navigate]
  );

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-8">
      <header data-drag="" className="flex select-none items-baseline justify-between">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">Home</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {`What's coming up, and the notes you've taken.`}
          </p>
        </div>
        <Button variant="outline" className="gap-2" onClick={quickNote}>
          <Plus className="h-4 w-4" />
          Quick note
        </Button>
      </header>

      {/* Coming up */}
      <section className="space-y-2">
        <h2 className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Coming up
        </h2>
        <Card>
          <CardContent className="flex items-center justify-between gap-4 py-5">
            <div className="flex items-center gap-3">
              <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
                <CalendarClock className="h-4 w-4" />
              </span>
              <div className="min-w-0">
                <p className="text-sm font-medium">No upcoming meetings</p>
                <p className="text-xs text-muted-foreground">
                  {`Connect your calendar to see what's next — or just start a note.`}
                </p>
              </div>
            </div>
            <Button className="shrink-0 gap-2" onClick={takeNotes}>
              <Mic className="h-4 w-4" />
              Take notes
            </Button>
          </CardContent>
        </Card>
      </section>

      {/* Recent notes */}
      <section className="space-y-3">
        <h2 className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Recent notes
        </h2>
        {loading ? null : groups.length === 0 ? (
          <Card>
            <CardContent className="flex flex-col items-center gap-3 py-12 text-center">
              <FileAudio className="h-7 w-7 text-muted-foreground" />
              <p className="text-sm text-muted-foreground">
                {`No notes yet. Take notes in your next meeting and they'll show up here.`}
              </p>
            </CardContent>
          </Card>
        ) : (
          groups.map(({ group, items }) => (
            <div key={group} className="space-y-1.5">
              <p className="px-1 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
                {group}
              </p>
              <div className="flex flex-col gap-1.5">
                {items.map((r) => (
                  <button
                    key={r.session_dir}
                    type="button"
                    onClick={() => openNote(r)}
                    className="flex items-center justify-between gap-3 rounded-lg border border-border bg-card px-4 py-3 text-left transition-colors hover:bg-muted/40"
                  >
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium">
                        {r.suggested_title || r.label}
                      </p>
                      {r.suggested_subtitle ? (
                        <p className="truncate text-xs text-muted-foreground">
                          {r.suggested_subtitle}
                        </p>
                      ) : null}
                    </div>
                    <span className="shrink-0 font-mono text-xs text-muted-foreground">
                      {timeLabel(r.created_at)}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          ))
        )}
      </section>

      <div className="mt-auto" />
      <AskBar />
    </div>
  );
}
