import * as React from "react";
import { Link } from "react-router-dom";
import {
  ArrowUpRight,
  Brain,
  CheckCircle2,
  Circle,
  FileText,
  Inbox as InboxIcon,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { Markdown } from "@/shared/ui/markdown";
import { cn } from "@/shared/lib/utils";
import { isAutonameEmpty, parseAutoname } from "@/shared/lib/autoname";
import {
  listAgentRuns,
  listMemories,
  listRecordings,
  listTasks,
  setTaskStatus,
} from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";
import type { AgentRun } from "@/shared/types/AgentRun";
import type { Memory } from "@/shared/types/Memory";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";
import type { Task } from "@/shared/types/Task";

interface AgentRunWithRecording extends AgentRun {
  recording: RecordingSummary;
}

const FRESH_WINDOW_HOURS = 24;
const RECENT_RUN_LIMIT = 5;

/**
 * Inbox — today's open actions, decisions, and new memories. Replaces
 * the retired flat /ai page (v2 finding 016 / GET-50). Three stacked
 * groups in one column:
 *
 *   1. Open tasks (todo + in_progress), agent-origin first.
 *   2. New memories from the last 24h.
 *   3. Recent agent runs (last 5) rendered as compact run-cards.
 *
 * Each card deep-links to the source recording or task board. The page
 * auto-refreshes when a new recording or transcript lands.
 */
export default function Inbox() {
  const lastTranscriptPath = useRecording((s) => s.lastTranscriptPath);
  const lastSavedDir = useRecording((s) => s.lastSavedDir);

  const [tasks, setTasks] = React.useState<Task[]>([]);
  const [memories, setMemories] = React.useState<Memory[]>([]);
  const [runs, setRuns] = React.useState<AgentRunWithRecording[]>([]);
  const [loading, setLoading] = React.useState(true);

  const refresh = React.useCallback(async () => {
    setLoading(true);
    try {
      const [allTasks, freshMems, recordings] = await Promise.all([
        listTasks(),
        listMemories({ query: null, kinds: [], include_archived: false, limit: 20 }),
        listRecordings(),
      ]);

      const openTasks = allTasks
        .filter((t) => t.status !== "done")
        .sort((a, b) => {
          if (a.agent_origin !== b.agent_origin) return a.agent_origin ? -1 : 1;
          return b.created_at.localeCompare(a.created_at);
        });

      const cutoff = Date.now() - FRESH_WINDOW_HOURS * 60 * 60 * 1000;
      const freshMemories = freshMems.filter((m) => {
        const t = Date.parse(m.created_at);
        return Number.isFinite(t) && t >= cutoff;
      });

      const candidates = recordings.filter((r) => r.has_transcript);
      const all = await Promise.all(
        candidates.map(async (recording) => {
          try {
            const list = await listAgentRuns(recording.session_dir);
            return list.map<AgentRunWithRecording>((run) => ({ ...run, recording }));
          } catch (e) {
            console.error(`list_agent_runs(${recording.session_dir}):`, e);
            return [];
          }
        })
      );
      const flat = all
        .flat()
        .sort((a, b) => b.finished_at.localeCompare(a.finished_at))
        .slice(0, RECENT_RUN_LIMIT);

      setTasks(openTasks);
      setMemories(freshMemories);
      setRuns(flat);
    } catch (e) {
      console.error("inbox refresh:", e);
      toast.error("Could not load Inbox", { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    refresh();
  }, [refresh, lastTranscriptPath, lastSavedDir]);

  const completeTask = React.useCallback(
    async (id: string) => {
      try {
        await setTaskStatus(id, "done");
        await refresh();
      } catch (e) {
        toast.error("Could not complete task", { description: String(e) });
      }
    },
    [refresh]
  );

  const empty = !loading && tasks.length === 0 && memories.length === 0 && runs.length === 0;

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 py-8">
      <header
        data-drag=""
        className="flex select-none items-baseline justify-between gap-4"
      >
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">Inbox</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            What needs you today — open tasks, fresh memories, and recent agent runs.
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={refresh}
          disabled={loading}
          className="gap-2"
          aria-label="Refresh Inbox"
        >
          <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
          Refresh
        </Button>
      </header>

      {empty ? (
        <Card>
          <CardContent className="flex flex-col items-center gap-3 py-16 text-center">
            <div className="rounded-full border border-border bg-muted/40 p-3">
              <InboxIcon className="h-6 w-6 text-muted-foreground" />
            </div>
            <h2 className="font-serif text-lg font-medium">All quiet.</h2>
            <p className="max-w-sm text-sm text-muted-foreground">
              When you record a meeting, agent-extracted tasks, decisions, and memories
              land here.
            </p>
            <Button asChild variant="outline" size="sm" className="mt-2">
              <Link to="/record">Start recording</Link>
            </Button>
          </CardContent>
        </Card>
      ) : (
        <>
          <Section
            title="Open actions"
            count={tasks.length}
            href="/tasks"
            hrefLabel="Open kanban"
            empty="No open tasks."
          >
            {tasks.slice(0, 6).map((t) => (
              <TaskRow key={t.id} task={t} onComplete={completeTask} />
            ))}
          </Section>

          <Section
            title="New memories"
            count={memories.length}
            href="/memory"
            hrefLabel="Open memory"
            empty="No memories in the last 24h."
          >
            {memories.slice(0, 6).map((m) => (
              <MemoryRow key={m.id} memory={m} />
            ))}
          </Section>

          <Section
            title="Recent runs"
            count={runs.length}
            href="/library"
            hrefLabel="Open library"
            empty="No agent runs yet."
          >
            {runs.map((run) => (
              <AgentRunCard
                key={`${run.recording.session_dir}::${run.agent_id}::${run.finished_at}`}
                run={run}
              />
            ))}
          </Section>
        </>
      )}
    </div>
  );
}

function Section({
  title,
  count,
  href,
  hrefLabel,
  empty,
  children,
}: {
  title: string;
  count: number;
  href: string;
  hrefLabel: string;
  empty: string;
  children: React.ReactNode;
}) {
  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between">
        <h2 className="text-sm font-medium tracking-tight">
          {title}{" "}
          <span className="ml-1 text-2xs font-normal text-muted-foreground">({count})</span>
        </h2>
        <Button asChild variant="ghost" size="sm" className="gap-1 text-2xs">
          <Link to={href}>
            {hrefLabel}
            <ArrowUpRight className="h-3 w-3" />
          </Link>
        </Button>
      </div>
      {count === 0 ? (
        <p className="px-1 text-xs italic text-muted-foreground">{empty}</p>
      ) : (
        <ul className="flex flex-col gap-2">{children}</ul>
      )}
    </section>
  );
}

function TaskRow({
  task,
  onComplete,
}: {
  task: Task;
  onComplete: (id: string) => void;
}) {
  return (
    <li>
      <Card className="overflow-hidden">
        <CardContent className="flex items-start gap-3 py-3">
          <button
            type="button"
            onClick={() => onComplete(task.id)}
            className="mt-0.5 shrink-0 rounded-full text-muted-foreground transition-colors hover:text-foreground"
            aria-label="Mark complete"
            title="Mark complete"
          >
            {task.status === "doing" ? (
              <CheckCircle2 className="h-4 w-4" />
            ) : (
              <Circle className="h-4 w-4" />
            )}
          </button>
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm text-foreground">{task.title}</p>
            <div className="mt-1 flex flex-wrap items-center gap-2 text-2xs text-muted-foreground">
              {task.agent_origin ? (
                <Badge variant="accent" className="gap-1 text-2xs">
                  <Sparkles className="h-3 w-3" />
                  Agent
                </Badge>
              ) : null}
              {task.owner ? <span>{task.owner}</span> : null}
              {task.due ? <span>· due {task.due}</span> : null}
              {task.source_session_label ? (
                <Link
                  to={`/editor/${encodeURIComponent(task.source_session_label)}`}
                  className="inline-flex items-center gap-1 hover:text-foreground"
                >
                  <FileText className="h-3 w-3" />
                  {task.source_session_label}
                </Link>
              ) : null}
            </div>
          </div>
        </CardContent>
      </Card>
    </li>
  );
}

function MemoryRow({ memory }: { memory: Memory }) {
  return (
    <li>
      <Card className="overflow-hidden">
        <CardContent className="flex items-start gap-3 py-3">
          <Brain className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1">
            <p className="text-sm text-foreground">{memory.content}</p>
            <div className="mt-1 flex flex-wrap items-center gap-2 text-2xs text-muted-foreground">
              <Badge variant="outline" className="text-2xs capitalize">
                {memory.kind}
              </Badge>
              {memory.key ? <span className="font-mono">{memory.key}</span> : null}
              {memory.source_session_label ? (
                <Link
                  to={`/editor/${encodeURIComponent(memory.source_session_label)}`}
                  className="inline-flex items-center gap-1 hover:text-foreground"
                >
                  <FileText className="h-3 w-3" />
                  {memory.source_session_label}
                </Link>
              ) : null}
            </div>
          </div>
        </CardContent>
      </Card>
    </li>
  );
}

function AgentRunCard({ run }: { run: AgentRunWithRecording }) {
  const editorHref = `/editor/${encodeURIComponent(run.recording.label)}`;
  return (
    <li>
      <Card className="overflow-hidden">
        <CardContent className="flex flex-col gap-2 py-4">
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
              title="Open run-card in editor"
            >
              <Link to={editorHref} state={{ recording: run.recording }}>
                Open
                <ArrowUpRight className="h-3.5 w-3.5" />
              </Link>
            </Button>
          </div>
          <div className="relative max-h-[10rem] overflow-hidden">
            {run.agent_id === "autoname" ? (
              <AutonameInboxPreview response={run.response} />
            ) : (
              <div className="prose prose-sm prose-neutral dark:prose-invert max-w-none text-foreground">
                <Markdown>{run.response}</Markdown>
              </div>
            )}
            <div className="pointer-events-none absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-card to-transparent" />
          </div>
        </CardContent>
      </Card>
    </li>
  );
}

/**
 * Compact inbox preview for the `autoname` agent. The agent emits a
 * JSON object ({title, tags, subtitle}); rendering that JSON
 * verbatim — which is what the default Markdown preview did — looks
 * like a parsing bug to the user. We parse it here and either show a
 * one-line "Title · subtitle [tag, tag]" line, or fall back to the
 * raw response when the JSON can't be parsed, so we never silently
 * hide a malformed model output.
 */
function AutonameInboxPreview({ response }: { response: string }) {
  const parsed = React.useMemo(() => parseAutoname(response), [response]);
  if (!parsed) {
    return (
      <div className="prose prose-sm prose-neutral dark:prose-invert max-w-none text-foreground">
        <Markdown>{response}</Markdown>
      </div>
    );
  }
  if (isAutonameEmpty(parsed)) {
    return (
      <p className="text-sm italic text-muted-foreground">
        No name suggested — the transcript was too short or noisy to title reliably.
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-1.5 text-sm">
      {parsed.title.length > 0 ? (
        <p className="font-medium text-foreground">{parsed.title}</p>
      ) : null}
      {parsed.subtitle.length > 0 ? (
        <p className="text-muted-foreground">{parsed.subtitle}</p>
      ) : null}
      {parsed.tags.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {parsed.tags.map((t) => (
            <Badge key={t} variant="outline" className="text-2xs">
              {t}
            </Badge>
          ))}
        </div>
      ) : null}
    </div>
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
