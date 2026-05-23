import * as React from "react";
import { Bot, Loader2, RefreshCw, Sparkles, Trash2 } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import { deleteAgentRun, listAgentRuns, listAgents, runAgent } from "@/shared/lib/ipc";
import type { Agent } from "@/shared/types/Agent";
import type { AgentRun } from "@/shared/types/AgentRun";

interface Props {
  sessionDir: string;
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
 * Phase 1.5 MVP. Lists every default agent. Clicking one runs it
 * against the recording's transcript using the configured OpenAI key
 * and shows the markdown response inline. Results persist under the
 * recording dir so they survive a reload.
 *
 * UX rules from the vault plan (`ai-chat-multi-provider.md`, section
 * "UX smoothness"):
 *   - One-click runs: pick agent, see result. No config in the flow.
 *   - Helpful empty states. "Configure your AI provider →" when no key.
 *   - Re-run button visible only after a run exists.
 *   - Markdown rendering is intentionally minimal (whitespace pre-wrap
 *     + monospace headings) until we adopt a real markdown renderer.
 */
export const AgentPanel = React.forwardRef<AgentPanelHandle, Props>(function AgentPanel(
  { sessionDir },
  ref
) {
  const [agents, setAgents] = React.useState<Agent[] | null>(null);
  const [runs, setRuns] = React.useState<Record<string, AgentRun>>({});
  const [running, setRunning] = React.useState<Set<string>>(new Set());
  const [expanded, setExpanded] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    Promise.all([listAgents(), listAgentRuns(sessionDir)])
      .then(([agentList, runList]) => {
        if (cancelled) return;
        setAgents(agentList);
        const map: Record<string, AgentRun> = {};
        for (const r of runList) map[r.agent_id] = r;
        setRuns(map);
        if (runList.length > 0) setExpanded(runList[0].agent_id);
      })
      .catch((e) => {
        if (cancelled) return;
        console.error("agent init:", e);
        toast.error("Could not load agents", { description: String(e) });
      });
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
      try {
        const run = await runAgent(sessionDir, agent.id);
        setRuns((prev) => ({ ...prev, [agent.id]: run }));
        setExpanded(agent.id);
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
      }
    },
    [sessionDir]
  );

  // Imperative handle: lets the recording-detail route fire an agent
  // by id (used by the autoRun nav-state hint). The handle's runAgent
  // reads the latest `agents` list via the ref so it always finds
  // freshly-loaded agents even if the parent triggers it during the
  // same render cycle that loaded them.
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
      if (expanded === agent.id) setExpanded(null);
      toast.success(`${agent.name} result deleted`);
    } catch (e) {
      toast.error(`Could not delete ${agent.name} result`, {
        description: String(e),
      });
    }
  };

  if (agents === null) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        <span>Loading agents…</span>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <header className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Bot className="h-4 w-4 text-muted-foreground" />
          <h2 className="text-sm font-semibold">AI agents</h2>
        </div>
        <p className="text-2xs text-muted-foreground">
          Uses your OpenAI key · configure in Settings → AI
        </p>
      </header>

      <div className="grid gap-2 sm:grid-cols-2">
        {agents.map((agent) => {
          const isRunning = running.has(agent.id);
          const existing = runs[agent.id];
          const isExpanded = expanded === agent.id;
          return (
            <AgentCard
              key={agent.id}
              agent={agent}
              run={existing}
              isRunning={isRunning}
              isExpanded={isExpanded}
              onRun={() => handleRun(agent)}
              onToggleExpand={() =>
                setExpanded((cur) => (cur === agent.id ? null : agent.id))
              }
              onDelete={() => handleDelete(agent)}
            />
          );
        })}
      </div>

      {expanded && runs[expanded] ? (
        <AgentResult
          agent={agents.find((a) => a.id === expanded)}
          run={runs[expanded]}
        />
      ) : null}
    </div>
  );
});

interface CardProps {
  agent: Agent;
  run: AgentRun | undefined;
  isRunning: boolean;
  isExpanded: boolean;
  onRun: () => void;
  onToggleExpand: () => void;
  onDelete: () => void;
}

function AgentCard({
  agent,
  run,
  isRunning,
  isExpanded,
  onRun,
  onToggleExpand,
  onDelete,
}: CardProps) {
  const hasRun = Boolean(run);
  return (
    <div
      className={cn(
        "flex flex-col rounded-lg border border-border bg-card p-3 transition-colors",
        isExpanded && "ring-1 ring-primary/40"
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
          <Badge variant="secondary" className="shrink-0 text-2xs">
            Ready
          </Badge>
        ) : null}
      </div>
      <div className="mt-auto flex items-center gap-1.5 pt-2">
        {hasRun ? (
          <>
            <Button
              type="button"
              size="sm"
              variant={isExpanded ? "secondary" : "outline"}
              onClick={onToggleExpand}
              className="flex-1"
            >
              {isExpanded ? "Hide result" : "Show result"}
            </Button>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              onClick={onRun}
              disabled={isRunning}
              aria-label="Re-run agent"
              title="Re-run agent"
            >
              {isRunning ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <RefreshCw className="h-3.5 w-3.5" />
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

function AgentResult({ agent, run }: { agent: Agent | undefined; run: AgentRun }) {
  const finishedAgo = relativeTime(run.finished_at);
  return (
    <div className="rounded-lg border border-border bg-muted/30 p-4">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2 text-2xs text-muted-foreground">
        <div className="flex items-center gap-2">
          <Sparkles className="h-3 w-3" />
          <span className="font-medium text-foreground">
            {agent?.name ?? run.agent_name}
          </span>
          <span>·</span>
          <span>{run.model}</span>
          <span>·</span>
          <span>{finishedAgo}</span>
        </div>
        {run.prompt_tokens !== null || run.completion_tokens !== null ? (
          <span className="font-mono">
            {run.prompt_tokens ?? "?"} in · {run.completion_tokens ?? "?"} out
          </span>
        ) : null}
      </div>
      <pre className="whitespace-pre-wrap break-words font-sans text-sm leading-relaxed text-foreground">
        {run.response}
      </pre>
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
