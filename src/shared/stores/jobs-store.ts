/**
 * Cross-cutting "what's running right now" store.
 *
 * Anything async that takes more than a couple of seconds (transcribing
 * a recording, running an agent, downloading a Whisper model) pushes a
 * job in here when it starts and pops it when it finishes. The global
 * <JobStrip /> renders the active jobs as pills under the drag strip,
 * giving the user one place to see all in-flight work without hunting
 * for spinners on individual rows.
 *
 * Job IDs are caller-supplied strings — typically `${kind}:${sessionDir}`
 * or `${kind}:${sessionDir}:${agentId}` — so the same job can be popped
 * by the caller without keeping a token. Pushing the same id twice is a
 * no-op (the latest payload wins on label/kind), so re-pushes from
 * idempotent retries don't multiply pills.
 */

import { create } from "zustand";

export type JobKind = "transcribe" | "agent" | "download";

export interface Job {
  id: string;
  kind: JobKind;
  /** Human-readable line for the strip ("Transcribing 2026-05-24-…"). */
  label: string;
  /** Secondary line shown smaller below the label. */
  detail?: string;
  /** Recording session dir the job belongs to, if applicable. */
  sessionDir?: string;
  /** Recording label (used to build the editor link). */
  recordingLabel?: string;
  /** Wall-clock ms when the job was pushed. */
  startedAt: number;
}

interface JobsState {
  jobs: Record<string, Job>;
  push: (job: Omit<Job, "startedAt"> & { startedAt?: number }) => void;
  pop: (id: string) => void;
  clearAll: () => void;
}

export const useJobsStore = create<JobsState>((set) => ({
  jobs: {},
  push: (job) =>
    set((s) => ({
      jobs: {
        ...s.jobs,
        [job.id]: { ...job, startedAt: job.startedAt ?? Date.now() },
      },
    })),
  pop: (id) =>
    set((s) => {
      if (!(id in s.jobs)) return s;
      const next = { ...s.jobs };
      delete next[id];
      return { jobs: next };
    }),
  clearAll: () => set({ jobs: {} }),
}));

/**
 * Do NOT subscribe via `useJobsStore((s) => Object.values(s.jobs).sort(...))`
 * — that returns a fresh array on every state read and triggers React's
 * "Maximum update depth exceeded" guard. Subscribe to `s.jobs` directly
 * (stable reference) and `useMemo` the sorted array inside the consumer.
 */
