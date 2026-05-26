import * as React from "react";
import {
  Bot,
  ChevronDown,
  ChevronUp,
  Loader2,
  RefreshCw,
  Sparkles,
  Trash2,
  Wand2,
} from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Markdown } from "@/shared/ui/markdown";
import { cn } from "@/shared/lib/utils";
import { deleteAgentRun, listAgentRuns, listAgents, runAgent } from "@/shared/lib/ipc";
import { useJobsStore } from "@/shared/stores/jobs-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";
import type { Agent } from "@/shared/types/Agent";
import type { AgentRun } from "@/shared/types/AgentRun";

interface Props {
  sessionDir: string;
}

/** Last path segment cross-platform — used for human-readable job labels. */
function basename(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const idx = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  return idx === -1 ? trimmed : trimmed.slice(idx + 1);
}

/**
 * Imperative handle exposed by [`AgentPanel`] so the recording detail
 * route can fire an agent in response to an autoRun nav-state hint
 * (e.g. the library row's [Summarize] button).
 */
export interface AgentPanelHandle {
  /** Run the agent with `agentId`. No-op (with a console warning) if
   * the agent is not in the list of loaded defaults. */
  runAgent: (agentId: string) => void;
}

/**
 * Per-recording agent panel.
 *
 * Lists every default agent in a compact grid that acts only as a
 * trigger (Run / Re-run / Delete). Every completed result renders
 * below the grid in a stacked list, always visible by default, with a
 * per-result collapse chevron for when the user wants to tidy up.
 *
 * Earlier iterations gated results behind a "Show result" button
 * which was friction — users want to read what just ran, not click
 * an extra control to see it.
 *
 * Results persist under `<session_dir>/agent_runs/<agent>.json` so
 * they survive a reload.
 */
export const AgentPanel = React.forwardRef<AgentPanelHandle, Props>(function AgentPanel(
  { sessionDir },
  ref
) {
  const [agents, setAgents] = React.useState<Agent[] | null>(null);
  const [runs, setRuns] = React.useState<Record<string, AgentRun>>({});
  const [running, setRunning] = React.useState<Set<string>>(new Set());
  // Per-result collapse state. Default is open (id NOT in the set).
  // Stays local to the component — no persistence; collapse a result,
  // navigate away, come back, it is open again.
  const [collapsed, setCollapsed] = React.useState<Set<string>>(new Set());

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [agentList, runList] = await Promise.all([
          listAgents(),
          listAgentRuns(sessionDir),
        ]);
        if (cancelled) return;
        setAgents(agentList);
        const map: Record<string, AgentRun> = {};
        for (const r of runList) map[r.agent_id] = r;
        setRuns(map);
      } catch (e) {
        if (cancelled) return;
        console.error("agent init:", e);
        toast.error("Could not load agents", { description: String(e) });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionDir]);

  const handleRun = React.useCallback(
    async (agent: Agent) => {
      setRunning((prev) => {
        const next = new Set(prev);
        next.add(agent.id);
        return next;
      });
      // Re-run should bring the result back into view if the user had
      // collapsed it; otherwise it's confusing to see "no spinner, no
      // result update" after a re-run click.
      setCollapsed((prev) => {
        if (!prev.has(agent.id)) return prev;
        const next = new Set(prev);
        next.delete(agent.id);
        return next;
      });
      const jobId = `agent:${agent.id}:${sessionDir}`;
      // Push into the cross-cutting jobs store so the user can see
      // the agent run in the top JobStrip even if they navigate away
      // from this recording while it runs.
      useJobsStore.getState().push({
        id: jobId,
        kind: "agent",
        label: `${agent.name}`,
        detail: basename(sessionDir),
        sessionDir,
        recordingLabel: basename(sessionDir),
      });
      try {
        const run = await runAgent(sessionDir, agent.id);
        setRuns((prev) => ({ ...prev, [agent.id]: run }));
        toast.success(`${agent.name} finished`);
      } catch (e) {
        console.error(`run_agent ${agent.id}:`, e);
        toast.error(`${agent.name} failed`, { description: String(e) });
      } finally {
        setRunning((prev) => {
          const next = new Set(prev);
          next.delete(agent.id);
          return next;
        });
        useJobsStore.getState().pop(jobId);
      }
    },
    [sessionDir]
  );

  // Imperative handle: lets the recording-detail route fire an agent
  // by id (used by the autoRun nav-state hint). Reads the latest
  // agents via a ref so it works even if the parent triggers it
  // during the same render cycle that loaded them.
  const agentsRef = React.useRef<Agent[] | null>(agents);
  React.useEffect(() => {
    agentsRef.current = agents;
  }, [agents]);
  React.useImperativeHandle(
    ref,
    () => ({
      runAgent: (agentId: string) => {
        const target = agentsRef.current?.find((a) => a.id === agentId);
        if (!target) {
          console.warn(`AgentPanel.runAgent: unknown agent id "${agentId}"`);
          return;
        }
        void handleRun(target);
      },
    }),
    [handleRun]
  );

  const handleDelete = async (agent: Agent) => {
    if (!window.confirm(`Delete the ${agent.name} result for this recording?`)) return;
    try {
      await deleteAgentRun(sessionDir, agent.id);
      setRuns((prev) => {
        const next = { ...prev };
        delete next[agent.id];
        return next;
      });
      setCollapsed((prev) => {
        if (!prev.has(agent.id)) return prev;
        const next = new Set(prev);
        next.delete(agent.id);
        return next;
      });
      toast.success(`${agent.name} result deleted`);
    } catch (e) {
      toast.error(`Could not delete ${agent.name} result`, {
        description: String(e),
      });
    }
  };

  const toggleCollapsed = (agentId: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(agentId)) next.delete(agentId);
      else next.add(agentId);
      return next;
    });
  };

  if (agents === null) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        <span>Loading agents…</span>
      </div>
    );
  }

  // Results render in the order agents are defined — stable, matches
  // the grid above so the user can visually link card to result.
  const completedAgents = agents.filter((a) => runs[a.id]);

  return (
    <div className="space-y-4">
      <header className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Bot className="h-4 w-4 text-muted-foreground" />
          <h2 className="text-sm font-semibold">AI agents</h2>
        </div>
        <p className="text-2xs text-muted-foreground">
          Uses your OpenAI key ·{" "}
          <button
            type="button"
            onClick={() => useSettingsUiStore.getState().openAt("ai")}
            className="rounded-sm font-medium text-foreground underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
          >
            configure in Settings → AI
          </button>
        </p>
      </header>

      <div className="grid gap-2 sm:grid-cols-2">
        {agents.map((agent) => (
          <AgentCard
            key={agent.id}
            agent={agent}
            run={runs[agent.id]}
            isRunning={running.has(agent.id)}
            onRun={() => handleRun(agent)}
            onDelete={() => handleDelete(agent)}
          />
        ))}
      </div>

      {completedAgents.length > 0 ? (
        <div className="space-y-3">
          {completedAgents.map((agent) => {
            const run = runs[agent.id];
            if (!run) return null;
            return (
            <AgentResult
              key={agent.id}
              agent={agent}
              run={run}
              collapsed={collapsed.has(agent.id)}
              isRunning={running.has(agent.id)}
              onToggleCollapsed={() => toggleCollapsed(agent.id)}
              onRerun={() => handleRun(agent)}
              onDelete={() => handleDelete(agent)}
            />
            );
          })}
        </div>
      ) : null}
    </div>
  );
});

interface CardProps {
  agent: Agent;
  run: AgentRun | undefined;
  isRunning: boolean;
  onRun: () => void;
  onDelete: () => void;
}

function AgentCard({ agent, run, isRunning, onRun, onDelete }: CardProps) {
  const hasRun = Boolean(run);
  return (
    <div
      className={cn(
        "flex flex-col rounded-lg border border-border bg-card p-3 transition-colors",
        hasRun && "border-primary/30"
      )}
    >
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{agent.name}</p>
          <p className="line-clamp-2 text-2xs text-muted-foreground">
            {agent.description}
          </p>
        </div>
        {hasRun ? (
          <Badge variant="secondary" className="shrink-0 gap-1 text-2xs">
            <Sparkles className="h-2.5 w-2.5" />
            Ran
          </Badge>
        ) : null}
      </div>
      <div className="mt-auto flex items-center gap-1.5 pt-2">
        {hasRun ? (
          <>
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={onRun}
              disabled={isRunning}
              className="flex-1 gap-1.5"
            >
              {isRunning ? (
                <>
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  Running…
                </>
              ) : (
                <>
                  <RefreshCw className="h-3.5 w-3.5" />
                  Re-run
                </>
              )}
            </Button>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              onClick={onDelete}
              aria-label="Delete result"
              title="Delete result"
              className="text-muted-foreground hover:text-destructive"
            >
              <Trash2 className="h-3.5 w-3.5" />
            </Button>
          </>
        ) : (
          <Button
            type="button"
            size="sm"
            onClick={onRun}
            disabled={isRunning}
            className="w-full gap-1.5"
          >
            {isRunning ? (
              <>
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                Running…
              </>
            ) : (
              <>
                <Sparkles className="h-3.5 w-3.5" />
                Run {agent.name}
              </>
            )}
          </Button>
        )}
      </div>
    </div>
  );
}

interface ResultProps {
  agent: Agent;
  run: AgentRun;
  collapsed: boolean;
  isRunning: boolean;
  onToggleCollapsed: () => void;
  onRerun: () => void;
  onDelete: () => void;
}

function AgentResult({
  agent,
  run,
  collapsed,
  isRunning,
  onToggleCollapsed,
  onRerun,
  onDelete,
}: ResultProps) {
  const finishedAgo = relativeTime(run.finished_at);
  return (
    <div className="rounded-lg border border-border bg-muted/30">
      <button
        type="button"
        onClick={onToggleCollapsed}
        className="flex w-full items-center gap-2 px-4 py-3 text-left hover:bg-muted/50"
        aria-expanded={!collapsed}
      >
        {collapsed ? (
          <ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronUp className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        )}
        <Sparkles className="h-3 w-3 shrink-0 text-muted-foreground" />
        <span className="text-sm font-medium">{agent.name}</span>
        <span aria-hidden="true" className="text-2xs text-muted-foreground">
          ·
        </span>
        <span className="truncate text-2xs text-muted-foreground">{run.model}</span>
        <span aria-hidden="true" className="text-2xs text-muted-foreground">
          ·
        </span>
        <span className="shrink-0 text-2xs text-muted-foreground">{finishedAgo}</span>
        {run.prompt_tokens !== null || run.completion_tokens !== null ? (
          <span className="ml-auto shrink-0 font-mono text-2xs tabular-nums text-muted-foreground">
            {run.prompt_tokens ?? "?"} in <span aria-hidden="true">·</span>{" "}
            {run.completion_tokens ?? "?"} out
          </span>
        ) : (
          <span className="ml-auto" />
        )}
        {/* NOTE: container intercepts a click that would otherwise bubble to the parent header's onClick. It does not itself act as a button; the children are real <Button> elements with full keyboard handling. Hence the eslint-disable. */}
        {/* eslint-disable-next-line jsx-a11y/no-static-element-interactions, jsx-a11y/click-events-have-key-events */}
        <span className="flex items-center gap-1" onClick={(e) => e.stopPropagation()}>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            onClick={onRerun}
            disabled={isRunning}
            aria-label="Re-run agent"
            title="Re-run agent"
            className="h-6 w-6"
          >
            {isRunning ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <RefreshCw className="h-3 w-3" />
            )}
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            onClick={onDelete}
            aria-label="Delete result"
            title="Delete result"
            className="h-6 w-6 text-muted-foreground hover:text-destructive"
          >
            <Trash2 className="h-3 w-3" />
          </Button>
        </span>
      </button>
      {!collapsed ? (
        <div className="border-t border-border px-4 py-4">
          {agent.id === "autoname" ? (
            <AutonameResultBody response={run.response} />
          ) : (
            <Markdown>{run.response}</Markdown>
          )}
          <RefineRow run={run} />
        </div>
      ) : null}
    </div>
  );
}

interface AutonameParsed {
  title: string;
  tags: string[];
  subtitle: string;
}

/**
 * Best-effort parse of the autoname agent's JSON response. The agent
 * prompt asks for `{title, tags, subtitle}` JSON only, but we tolerate
 * stray prose / fences by lifting the first balanced `{...}` block.
 * Mirrors the Rust-side parser in
 * `crates/attune-core/src/storage/session.rs` so the React card and
 * the library row never disagree.
 */
export function parseAutoname(response: string): AutonameParsed | null {
  const start = response.indexOf("{");
  if (start === -1) return null;
  let depth = 0;
  let inString = false;
  let escaped = false;
  let end = -1;
  for (let i = start; i < response.length; i++) {
    const ch = response[i];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === '"') {
        inString = false;
      }
      continue;
    }
    if (ch === '"') inString = true;
    else if (ch === "{") depth += 1;
    else if (ch === "}") {
      depth -= 1;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  if (end === -1) return null;
  try {
    const parsed = JSON.parse(response.slice(start, end + 1)) as unknown;
    if (!parsed || typeof parsed !== "object") return null;
    const obj = parsed as Record<string, unknown>;
    const title = typeof obj.title === "string" ? obj.title.trim() : "";
    const subtitle = typeof obj.subtitle === "string" ? obj.subtitle.trim() : "";
    const tags = Array.isArray(obj.tags)
      ? obj.tags.filter((t): t is string => typeof t === "string" && t.trim().length > 0)
      : [];
    return { title, tags, subtitle };
  } catch {
    return null;
  }
}

function AutonameResultBody({ response }: { response: string }) {
  const parsed = React.useMemo(() => parseAutoname(response), [response]);
  if (!parsed) {
    return <Markdown>{response}</Markdown>;
  }
  const isEmpty =
    parsed.title.length === 0 && parsed.tags.length === 0 && parsed.subtitle.length === 0;
  if (isEmpty) {
    return (
      <p className="text-sm text-muted-foreground">
        No name suggested — the transcript was too short or noisy to title reliably.
      </p>
    );
  }
  return (
    <dl className="grid grid-cols-[5rem_1fr] gap-x-3 gap-y-2 text-sm">
      {parsed.title.length > 0 ? (
        <>
          <dt className="text-xs text-muted-foreground">Title</dt>
          <dd className="font-medium">{parsed.title}</dd>
        </>
      ) : null}
      {parsed.subtitle.length > 0 ? (
        <>
          <dt className="text-xs text-muted-foreground">Subtitle</dt>
          <dd>{parsed.subtitle}</dd>
        </>
      ) : null}
      {parsed.tags.length > 0 ? (
        <>
          <dt className="text-xs text-muted-foreground">Tags</dt>
          <dd className="flex flex-wrap gap-1.5">
            {parsed.tags.map((t) => (
              <Badge key={t} variant="outline" className="text-2xs">
                {t}
              </Badge>
            ))}
          </dd>
        </>
      ) : null}
    </dl>
  );
}

/** Conversational-repair affordance (v2 finding 035 / GET-87).
 *
 *  The user types a refinement instruction → we copy a prompt
 *  template to the clipboard that wraps the prior agent response in
 *  a quote and prepends the refinement text. The user pastes into
 *  the Q&A agent (or any chat-style agent) to get a corrected
 *  output. Full tool-calling refinement that re-emits via the
 *  agent's create_* path stays as the next-PR target — this surface
 *  ships the affordance today and exercises the UX the deeper
 *  version will reuse. */
function RefineRow({ run }: { run: AgentRun }) {
  const [open, setOpen] = React.useState(false);
  const [instruction, setInstruction] = React.useState("");
  const handleCopy = async () => {
    const payload = [
      `Please refine the following ${run.agent_name} output.`,
      "",
      "Original:",
      ...run.response.split("\n").map((line) => `> ${line}`),
      "",
      `Refinement: ${instruction.trim()}`,
    ].join("\n");
    await navigator.clipboard.writeText(payload).catch((e) => {
      console.error("clipboard.writeText:", e);
    });
    toast.success("Refinement copied", {
      description: "Paste into the Q&A agent (or any chat) to apply.",
    });
    setOpen(false);
    setInstruction("");
  };
  if (!open) {
    return (
      <div className="mt-3 flex justify-end">
        <Button
          variant="ghost"
          size="sm"
          className="gap-1.5 text-2xs text-muted-foreground hover:text-foreground"
          onClick={() => setOpen(true)}
          aria-label="Refine this result"
        >
          <Wand2 className="h-3 w-3" />
          Refine
        </Button>
      </div>
    );
  }
  return (
    <div className="mt-3 flex flex-col gap-2 rounded-md border border-border bg-secondary/40 p-2">
      <textarea
        value={instruction}
        onChange={(e) => setInstruction(e.target.value)}
        placeholder="e.g. 'Drop the action items section and keep only the decision summary.'"
        rows={2}
        className="w-full resize-none rounded bg-transparent text-xs leading-relaxed outline-none placeholder:text-muted-foreground"
      />
      <div className="flex justify-end gap-1.5">
        <Button
          variant="ghost"
          size="sm"
          className="text-2xs"
          onClick={() => {
            setOpen(false);
            setInstruction("");
          }}
        >
          Cancel
        </Button>
        <Button
          size="sm"
          className="text-2xs"
          onClick={handleCopy}
          disabled={instruction.trim().length === 0}
        >
          Copy refinement
        </Button>
      </div>
    </div>
  );
}

function relativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  if (Number.isNaN(ms)) return "unknown";
  const sec = Math.round(ms / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.round(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.round(hr / 24);
  return `${day}d ago`;
}
