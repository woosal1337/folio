import { create } from "zustand";

export type JobKind =
  | "finalize"
  | "vad"
  | "transcribe"
  | "diarize"
  | "agent"
  | "download"
  | "sync";

export interface Job {
  id: string;
  kind: JobKind;

  label: string;

  detail?: string;

  sessionDir?: string;

  recordingLabel?: string;

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
