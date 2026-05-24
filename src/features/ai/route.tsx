import * as React from "react";
import { Bot, RefreshCw, Sparkles, ArrowUpRight, FileText } from "lucide-react";
import { Link } from "react-router-dom";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { Markdown } from "@/shared/ui/markdown";
import { cn } from "@/shared/lib/utils";
import { listAgentRuns, listRecordings } from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";
import type { AgentRun } from "@/shared/types/AgentRun";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

interface AgentRunWithRecording extends AgentRun {
  recording: RecordingSummary;
}

/**
 * Top-level AI page — a flat reverse-chronological view of every agent
 * run across every recording in the library. Mirrors the "Library" view
 * but scoped to AI output: the user comes here to find a summary they
 * remember running, without having to re-locate the original recording
 * first.
 *
 * Each card shows the recording label as a header link (navigates to
 * the editor with the agent's result already in view), the agent name
 * and timestamp, and a truncated markdown preview of the response.
 *
 * Data path: list_recordings → for each transcribed recording,
 * list_agent_runs(session_dir) → flatten → sort by finished_at desc.
 * This is N+1 but acceptable for v1 — once the library grows, a single
 * `list_all_agent_runs` backend command can drop in here without any
 * UI change. The keyed cache key + the recording-store update hooks
 * mean the page auto-refreshes when a new agent run lands elsewhere.
 */
export default function Ai() {
  const lastTranscriptPath = useRecording((s) => s.lastTranscriptPath);
  const lastSavedDir = useRecording((s) => s.lastSavedDir);

  const [runs, setRuns] = React.useState<AgentRunWithRecording[]>([]);
  const [loading, setLoading] = React.useState(true);

  const refresh = React.useCallback(async () => {
    setLoading(true);
    try {
      const recordings = await listRecordings();
      // Skip recordings without a transcript — they can't have agent runs.
      const candidates = recordings.filter((r) => r.has_transcript);
      const all = await Promise.all(
        candidates.map(async (recording) => {
          try {
            const list = await listAgentRuns(recording.session_dir);
            return list.map<AgentRunWithRecording>((run) => ({ ...run, recording }));
          } catch (e) {
            // One bad recording's agent_runs dir shouldn't tank the whole page.
            console.error(`list_agent_runs(${recording.session_dir}):`, e);
            return [];
          }
        })
      );
      const flat = all.flat().sort((a, b) => {
        // ISO strings sort lexicographically; reverse for newest first.
        return b.finished_at.localeCompare(a.finished_at);
      });
      setRuns(flat);
    } catch (e) {
      console.error("ai page refresh:", e);
      toast.error("Could not load AI runs", { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  // First-mount + reactive refresh whenever a recording or transcript
  // lands elsewhere in the app (e.g. user runs a summary in the editor
  // tab while looking at this page).
  React.useEffect(() => {
    refresh();
  }, [refresh, lastTranscriptPath, lastSavedDir]);

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-10">
      <header
        data-drag=""
        className="flex select-none items-baseline justify-between gap-4"
      >
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">
            AI insights
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Every summary, action-item list, and agent run across your recordings —
            newest first.
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={refresh}
          disabled={loading}
          className="gap-2"
          aria-label="Refresh AI runs"
        >
          <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
          Refresh
        </Button>
      </header>

      {loading && runs.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-sm text-muted-foreground">
            Loading…
          </CardContent>
        </Card>
      ) : runs.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center gap-3 py-12 text-center">
            <Bot className="h-7 w-7 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">
              No agent runs yet. Open a transcribed recording from the Library and run
              Summarize, Extract tasks, Find decisions, or QA — they&apos;ll show up
              here.
            </p>
            <Button asChild variant="outline" size="sm" className="mt-2">
              <Link to="/library">Go to Library</Link>
            </Button>
          </CardContent>
        </Card>
      ) : (
        <ul className="flex flex-col gap-3">
          {runs.map((run) => (
            <AgentRunCard
              key={`${run.recording.session_dir}::${run.agent_id}::${run.finished_at}`}
              run={run}
            />
          ))}
        </ul>
      )}
    </div>
  );
}

function AgentRunCard({ run }: { run: AgentRunWithRecording }) {
  const editorHref = `/editor/${encodeURIComponent(run.recording.label)}`;
  return (
    <li>
      <Card className="overflow-hidden">
        <CardContent className="flex flex-col gap-3 py-5">
          {/* Card header: recording label (clickable) + agent badge + timestamp. */}
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0 space-y-1">
              <Link
                to={editorHref}
                state={{ recording: run.recording }}
                className="group flex items-center gap-1.5 text-sm font-medium text-foreground hover:text-primary"
              >
                <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground group-hover:text-primary" />
                <span className="truncate">{run.recording.label}</span>
              </Link>
              <div className="flex items-center gap-2 text-2xs text-muted-foreground">
                <Badge variant="accent" className="gap-1 text-2xs">
                  <Sparkles className="h-3 w-3" />
                  {run.agent_name}
                </Badge>
                <span className="font-mono">{formatFinishedAt(run.finished_at)}</span>
              </div>
            </div>
            <Button
              asChild
              variant="ghost"
              size="sm"
              className="shrink-0 gap-1"
              title="Open full result in editor"
            >
              <Link to={editorHref} state={{ recording: run.recording }}>
                Open
                <ArrowUpRight className="h-3.5 w-3.5" />
              </Link>
            </Button>
          </div>

          {/* Body: clamped markdown preview. The full result lives on the
              editor page; this view is for browsing, not deep reading. */}
          <div className="relative max-h-[18rem] overflow-hidden">
            <div className="prose prose-sm prose-neutral dark:prose-invert max-w-none text-foreground">
              <Markdown>{run.response}</Markdown>
            </div>
            <div className="pointer-events-none absolute inset-x-0 bottom-0 h-12 bg-gradient-to-t from-card to-transparent" />
          </div>
        </CardContent>
      </Card>
    </li>
  );
}

function formatFinishedAt(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}
