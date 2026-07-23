import * as React from "react";
import { useNavigate } from "react-router-dom";
import { Cloud, FileAudio, FileText, Loader2, Lock, Mic, Plus } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { listRecordings, onRemoteSyncProgress } from "@/shared/lib/ipc";
import { useQuickNote, useTakeNotes } from "@/shared/hooks/use-take-notes";
import { useNoteContextMenu } from "@/shared/hooks/use-note-context-menu";
import { useJobsStore } from "@/shared/stores/jobs-store";
import { useRecording } from "@/shared/stores/recording-store";
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

  const reload = React.useCallback(async () => {
    try {
      setRecordings(await listRecordings());
    } catch (e) {
      console.error("home: listRecordings failed", e);
    } finally {
      setLoading(false);
    }
  }, []);

  const lastSavedDir = useRecording((s) => s.lastSavedDir);
  const lastTranscriptPath = useRecording((s) => s.lastTranscriptPath);
  const transcribingDir = useRecording((s) => s.transcribingDir);
  React.useEffect(() => {
    void reload();
  }, [reload, lastSavedDir, lastTranscriptPath]);

  const jobCount = useJobsStore((s) => Object.keys(s.jobs).length);
  const prevJobCount = React.useRef(jobCount);
  React.useEffect(() => {
    if (jobCount < prevJobCount.current) void reload();
    prevJobCount.current = jobCount;
  }, [jobCount, reload]);

  React.useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onRemoteSyncProgress(() => void reload()).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [reload]);

  const openContextMenu = useNoteContextMenu(reload);

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
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 py-8">
      <header data-drag="" className="flex select-none items-baseline justify-between">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">Home</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {`The notes you've taken.`}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button className="gap-2" onClick={() => takeNotes()}>
            <Mic className="h-4 w-4" />
            Take notes
          </Button>
          <Button variant="outline" className="gap-2" onClick={quickNote}>
            <Plus className="h-4 w-4" />
            Quick note
          </Button>
        </div>
      </header>

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
                    onContextMenu={(e) => openContextMenu(r, e)}
                    className="flex items-center justify-between gap-3 rounded-lg border border-border bg-card px-4 py-3 text-left transition-colors hover:bg-muted/40"
                  >
                    <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium">
                        {r.title?.trim() ||
                          r.suggested_title ||
                          r.draft_name ||
                          r.label}
                      </p>
                      <p className="truncate text-xs text-muted-foreground">
                        {r.suggested_subtitle || "Me"}
                      </p>
                    </div>
                    {transcribingDir === r.session_dir ? (
                      <Loader2
                        className="h-3 w-3 shrink-0 animate-spin text-muted-foreground"
                        aria-label="Transcribing"
                      />
                    ) : r.sync?.remote_status === "succeeded" ? (
                      <Cloud
                        className="h-3 w-3 shrink-0 text-sky-500"
                        aria-label="Synced to your server"
                      />
                    ) : (
                      <Lock
                        className="h-3 w-3 shrink-0 text-muted-foreground"
                        aria-label="Stored only on this Mac"
                      />
                    )}
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
