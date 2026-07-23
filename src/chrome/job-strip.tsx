import * as React from "react";
import {
  AudioLines,
  Bot,
  Cloud,
  FileAudio,
  Layers,
  Loader2,
  ScanFace,
  Sparkles,
  type LucideIcon,
} from "lucide-react";
import { Link } from "react-router-dom";

import { cn } from "@/shared/lib/utils";
import { useJobsStore, type Job, type JobKind } from "@/shared/stores/jobs-store";

const KIND_META: Record<JobKind, { icon: LucideIcon; tone: string }> = {
  finalize: { icon: Layers, tone: "text-primary" },
  vad: { icon: AudioLines, tone: "text-primary" },
  transcribe: { icon: FileAudio, tone: "text-primary" },
  diarize: { icon: ScanFace, tone: "text-primary" },
  agent: { icon: Sparkles, tone: "text-primary" },
  download: { icon: Bot, tone: "text-primary" },
  sync: { icon: Cloud, tone: "text-primary" },
};

export function JobStrip() {
  const jobsMap = useJobsStore((s) => s.jobs);
  const jobs = React.useMemo(
    () => Object.values(jobsMap).sort((a, b) => b.startedAt - a.startedAt),
    [jobsMap]
  );

  if (jobs.length === 0) return null;

  return (
    <div
      data-drag=""
      role="status"
      aria-live="polite"
      className="select-none border-b border-border bg-background/85 backdrop-blur supports-[backdrop-filter]:bg-background/65"
    >
      <ul className="container-x flex items-center gap-2 overflow-x-auto px-4 py-1.5">
        {jobs.map((job) => (
          <li key={job.id}>
            <JobPill job={job} />
          </li>
        ))}
      </ul>
    </div>
  );
}

function JobPill({ job }: { job: Job }) {
  const { icon: Icon, tone } = KIND_META[job.kind];
  const className = cn(
    "group inline-flex max-w-[280px] items-center gap-2 rounded-full border border-border bg-card px-2.5 py-1 text-xs transition-colors hover:border-primary/40 hover:bg-secondary"
  );
  const inner = (
    <>
      <Loader2
        className={cn("h-3 w-3 shrink-0 animate-spin", tone)}
        aria-hidden="true"
      />
      <Icon className="h-3 w-3 shrink-0 text-muted-foreground" aria-hidden="true" />
      <span className="min-w-0 truncate font-medium text-foreground">{job.label}</span>
      {job.detail && (
        <span className="hidden truncate font-mono text-2xs text-muted-foreground md:inline">
          {job.detail}
        </span>
      )}
    </>
  );

  if (job.recordingLabel) {
    return (
      <Link
        to={`/editor/${encodeURIComponent(job.recordingLabel)}`}
        className={className}
      >
        {inner}
      </Link>
    );
  }

  return <span className={className}>{inner}</span>;
}
