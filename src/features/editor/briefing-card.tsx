import * as React from "react";
import { Copy, Clock, Users, FileText, ListChecks, Brain, Sparkles } from "lucide-react";
import { toast } from "sonner";

import { Card, CardContent } from "@/shared/ui/card";
import { Button } from "@/shared/ui/button";
import { Markdown } from "@/shared/ui/markdown";
import { cn } from "@/shared/lib/utils";
import { copyToClipboard } from "@/shared/lib/share";
import type { AgentRun } from "@/shared/types/AgentRun";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

interface Props {
  recording: RecordingSummary;
  summary: AgentRun | null;
  tasks: AgentRun | null;
  memories: AgentRun | null;
}

/**
 * 30-second post-meeting briefing card. v2 finding 026 / GET-39.
 *
 * Sits at the top of the editor on Stop. Title, duration, participant
 * count, and three slots that fill in as summarize / extract-tasks /
 * extract-memories complete. Each slot shows a skeleton until its
 * agent run lands; the Copy button serialises the whole card to
 * plain text — that's the artifact users paste into Slack / Notion.
 */
export function BriefingCard({ recording, summary, tasks, memories }: Props) {
  const onCopy = React.useCallback(async () => {
    const text = serialiseAsPlainText({ recording, summary, tasks, memories });
    try {
      await copyToClipboard(text);
      toast.success("Briefing copied", {
        description: `${text.length} characters on the clipboard.`,
      });
    } catch (e) {
      toast.error("Could not copy briefing", { description: String(e) });
    }
  }, [recording, summary, tasks, memories]);

  return (
    <Card className="overflow-hidden">
      <CardContent className="flex flex-col gap-4 py-5">
        <header className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h2 className="truncate font-serif text-xl font-medium">
              {recording.suggested_title?.trim() || recording.label}
            </h2>
            <dl className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-2xs text-muted-foreground">
              <Stat icon={Clock} label={formatDuration(recording.duration_seconds)} />
              <Stat icon={Users} label={`${participantCount(recording)} channel${participantCount(recording) === 1 ? "" : "s"}`} />
              {recording.suggested_tags?.length ? (
                <span className="font-mono">{recording.suggested_tags.slice(0, 3).join(" · ")}</span>
              ) : null}
            </dl>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onCopy}
            className="shrink-0 gap-1"
            aria-label="Copy briefing to clipboard"
          >
            <Copy className="h-3.5 w-3.5" />
            Copy
          </Button>
        </header>

        <Slot
          icon={FileText}
          label="Summary"
          empty="Summarising…"
          run={summary}
          previewLines={6}
        />
        <Slot
          icon={ListChecks}
          label="Action items"
          empty="Pulling action items…"
          run={tasks}
          previewLines={6}
        />
        <Slot
          icon={Brain}
          label="Memories"
          empty="Capturing memories…"
          run={memories}
          previewLines={6}
        />
      </CardContent>
    </Card>
  );
}

interface SlotProps {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  empty: string;
  run: AgentRun | null;
  previewLines: number;
}

function Slot({ icon: Icon, label, empty, run, previewLines }: SlotProps) {
  const hasRun = run !== null;
  return (
    <section
      className={cn(
        "rounded-md border border-border bg-muted/30 px-3 py-2.5",
        !hasRun && "animate-pulse"
      )}
    >
      <div className="mb-1.5 flex items-center gap-1.5 text-2xs uppercase tracking-wider text-muted-foreground">
        <Icon className="h-3 w-3" />
        {label}
        {hasRun ? (
          <span className="ml-auto inline-flex items-center gap-1 text-emerald-600 dark:text-emerald-400">
            <Sparkles className="h-3 w-3" />
            ready
          </span>
        ) : null}
      </div>
      {hasRun ? (
        <div
          className="prose prose-sm prose-neutral dark:prose-invert max-w-none"
          style={{ maxHeight: `${previewLines * 1.6}em`, overflow: "hidden" }}
        >
          <Markdown>{run.response}</Markdown>
        </div>
      ) : (
        <p className="text-xs italic text-muted-foreground">{empty}</p>
      )}
    </section>
  );
}

function Stat({ icon: Icon, label }: { icon: React.ComponentType<{ className?: string }>; label: string }) {
  return (
    <span className="inline-flex items-center gap-1">
      <Icon className="h-3 w-3" />
      {label}
    </span>
  );
}

function formatDuration(secs: number | bigint | null | undefined): string {
  const n = Number(secs ?? 0);
  if (!Number.isFinite(n) || n <= 0) return "—";
  const h = Math.floor(n / 3600);
  const m = Math.floor((n % 3600) / 60);
  const s = Math.floor(n % 60);
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

function participantCount(recording: RecordingSummary): number {
  let n = 0;
  if (recording.mic_bytes && Number(recording.mic_bytes) > 0) n += 1;
  if (recording.system_bytes && Number(recording.system_bytes) > 0) n += 1;
  return n;
}

interface SerialiseInput {
  recording: RecordingSummary;
  summary: AgentRun | null;
  tasks: AgentRun | null;
  memories: AgentRun | null;
}

/**
 * Serialise the briefing card to plain text — Slack/Notion paste
 * target. Exported for tests; the runtime calls it via onCopy.
 */
export function serialiseAsPlainText({ recording, summary, tasks, memories }: SerialiseInput): string {
  const title = recording.suggested_title?.trim() || recording.label;
  const lines: string[] = [
    title,
    `Duration: ${formatDuration(recording.duration_seconds)} · ${participantCount(recording)} channel(s)`,
    "",
  ];
  if (summary) {
    lines.push("## Summary", "", summary.response.trim(), "");
  }
  if (tasks) {
    lines.push("## Action items", "", tasks.response.trim(), "");
  }
  if (memories) {
    lines.push("## Memories", "", memories.response.trim(), "");
  }
  return lines.join("\n");
}
