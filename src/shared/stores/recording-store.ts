import { toast } from "sonner";
import { create } from "zustand";

import {
  getRecording as ipcGetRecording,
  hasOpenAiKey as ipcHasOpenAiKey,
  recordingStatus as fetchStatus,
  runAgent as ipcRunAgent,
  setTrayRecording as ipcSetTrayRecording,
  showRecordingBar as ipcShowRecordingBar,
  hideRecordingBar as ipcHideRecordingBar,
  startRecording as ipcStart,
  stopRecording as ipcStop,
  pauseRecording as ipcPause,
  resumeRecording as ipcResume,
  transcribeRecording as ipcTranscribe,
  diarizeSession as ipcDiarize,
  runVad as ipcRunVad,
  whisperModelStatus as ipcWhisperModelStatus,
  syncRecording as ipcSyncRecording,
  currentWindowLabel,
  RECORDING_BAR_WINDOW_LABEL,
} from "@/shared/lib/ipc";
import { estimateOpenAITranscribeCost, formatUsd } from "@/shared/lib/cost-estimate";
import { humanizeError } from "@/shared/lib/errors";
import { bridgeNavigate } from "@/shared/lib/navigate-bridge";
import { playFeedback } from "@/shared/lib/feedback";
import { formatBatteryPct, readPower, shouldDeferOnPower } from "@/shared/lib/power";
import { useCloudCostConfirmStore } from "@/shared/stores/cloud-cost-confirm-store";
import { useJobsStore } from "@/shared/stores/jobs-store";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";
import type { SyncState } from "@/shared/types/SyncState";

export type RemoteSyncStage = "uploading" | "queued" | "processing";

interface RecordingState {
  recording: boolean;

  paused: boolean;

  startedAt: number | null;

  elapsed: number;

  channels: string[];

  error: string | null;

  busy: boolean;

  lastSavedDir: string | null;

  liveSessionDir: string | null;

  transcribing: boolean;

  transcribingDir: string | null;

  lastTranscriptPath: string | null;

  transcribeError: string | null;

  transcribeErrorDir: string | null;

  remoteStage: RemoteSyncStage | null;

  _tickerId: number | null;

  syncFromBackend: () => Promise<void>;

  start: (sessionDir?: string) => Promise<void>;

  stop: () => Promise<void>;

  pause: () => Promise<void>;

  resume: () => Promise<void>;

  transcribe: (sessionDir: string) => Promise<void>;
}

export const useRecording = create<RecordingState>((set, get) => {
  let segmentRolling = false;

  const tick = () => {
    const { startedAt, recording, paused } = get();
    if (!recording || startedAt === null) return;
    const next = Math.floor((Date.now() - startedAt) / 1000);
    set({ elapsed: next });

    void ipcSetTrayRecording(next, paused);
  };

  const autoSegment = async () => {
    if (segmentRolling) return;
    segmentRolling = true;
    try {
      console.info("[marathon] auto-segmenting recording");
      await ipcPause();

      await new Promise((r) => setTimeout(r, 300));
      await ipcResume();
    } catch (e) {
      console.error("[marathon] auto-segment failed:", e);
    } finally {
      segmentRolling = false;
    }
  };

  const installTicker = () => {
    const existing = get()._tickerId;
    if (existing !== null) window.clearInterval(existing);
    const id = window.setInterval(tick, 250);
    set({ _tickerId: id });
  };

  const clearTicker = () => {
    const existing = get()._tickerId;
    if (existing !== null) {
      window.clearInterval(existing);
      set({ _tickerId: null });
    }
    void ipcSetTrayRecording(null);
  };

  const basename = (path: string): string => {
    const trimmed = path.replace(/[\\/]+$/, "");
    const idx = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
    return idx === -1 ? trimmed : trimmed.slice(idx + 1);
  };

  const formatDurationSeconds = (s: number): string => {
    const safe = Math.max(0, Math.floor(s));
    const m = Math.floor(safe / 60);
    const sec = safe % 60;
    return `${m}:${sec.toString().padStart(2, "0")}`;
  };

  const navigateToNote = (sessionDir: string) => {
    try {
      const win = currentWindowLabel();
      if (win === RECORDING_BAR_WINDOW_LABEL) {
        return;
      }
    } catch {
      return;
    }
    const target = `/editor/${encodeURIComponent(basename(sessionDir))}`;
    if (window.location.hash === `#${target}`) return;
    bridgeNavigate(target);
  };

  const maybeAutoSummarize = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;
    if (!settings) return;
    if (!settings.auto_summarize_enabled) return;
    if (!(await ipcHasOpenAiKey())) {
      return;
    }
    const jobId = `agent:summarize:${sessionDir}`;
    useJobsStore.getState().push({
      id: jobId,
      kind: "agent",
      label: `Summarizing ${basename(sessionDir)}`,
      detail: "auto",
      sessionDir,
      recordingLabel: basename(sessionDir),
    });
    try {
      await ipcRunAgent(sessionDir, "summarize");
      toast.success("Summary ready", { description: basename(sessionDir) });
    } catch (e) {
      console.error("auto-summarize failed:", e);
      toast.error("Auto-summary failed", { description: humanizeError(e) });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  const maybeAutoExtractMemories = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;
    if (!settings) return;
    if (!settings.auto_extract_memories_enabled) return;
    if (!(await ipcHasOpenAiKey())) {
      return;
    }
    const jobId = `agent:extract-memories:${sessionDir}`;
    useJobsStore.getState().push({
      id: jobId,
      kind: "agent",
      label: `Capturing memories from ${basename(sessionDir)}`,
      detail: "auto",
      sessionDir,
      recordingLabel: basename(sessionDir),
    });
    try {
      await ipcRunAgent(sessionDir, "extract-memories");
      toast.success("Memories captured", { description: basename(sessionDir) });
    } catch (e) {
      console.error("auto-extract-memories failed:", e);
      toast.error("Auto-extract memories failed", { description: humanizeError(e) });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  const maybeAutoName = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;
    if (!settings) return;
    if (!settings.auto_name_enabled) return;
    if (!(await ipcHasOpenAiKey())) {
      return;
    }
    const jobId = `agent:autoname:${sessionDir}`;
    useJobsStore.getState().push({
      id: jobId,
      kind: "agent",
      label: `Naming ${basename(sessionDir)}`,
      detail: "auto",
      sessionDir,
      recordingLabel: basename(sessionDir),
    });
    try {
      await ipcRunAgent(sessionDir, "autoname");
    } catch (e) {
      console.error("auto-name failed:", e);
      toast.error("Auto-name failed", { description: humanizeError(e) });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  const maybeAutoExtractTasks = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;
    if (!settings) return;
    if (!settings.auto_extract_tasks_enabled) return;
    if (!(await ipcHasOpenAiKey())) {
      return;
    }
    const jobId = `agent:extract-tasks:${sessionDir}`;
    useJobsStore.getState().push({
      id: jobId,
      kind: "agent",
      label: `Extracting tasks from ${basename(sessionDir)}`,
      detail: "auto",
      sessionDir,
      recordingLabel: basename(sessionDir),
    });
    try {
      await ipcRunAgent(sessionDir, "extract-tasks");
      toast.success("Tasks ready", { description: basename(sessionDir) });
    } catch (e) {
      console.error("auto-extract-tasks failed:", e);
      toast.error("Auto-extract tasks failed", { description: humanizeError(e) });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  const openTranscriptionSettings = () =>
    useSettingsUiStore.getState().openAt("transcription");

  const runRemoteSync = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;
    if (!settings?.remote_endpoint || settings.remote_endpoint.trim().length === 0) {
      playFeedback("error");
      toast.error("Remote server not configured", {
        description: "Set a server endpoint in Settings → Transcription.",
        action: { label: "Open Settings", onClick: openTranscriptionSettings },
      });
      return;
    }
    const jobId = `transcribe:${sessionDir}`;
    const startedAt = Date.now();
    const stageOf = (state: SyncState): RemoteSyncStage =>
      state.upload_state !== "complete"
        ? "uploading"
        : state.remote_status === "running"
          ? "processing"
          : "queued";
    const stageDetail: Record<RemoteSyncStage, string> = {
      uploading: "Uploading",
      queued: "Queued",
      processing: "On GPU",
    };
    const pushJob = (stage: RemoteSyncStage) =>
      useJobsStore.getState().push({
        id: jobId,
        kind: "sync",
        label: `Syncing ${basename(sessionDir)} to server`,
        detail: stageDetail[stage],
        sessionDir,
        recordingLabel: basename(sessionDir),
        startedAt,
      });
    set({
      transcribing: true,
      transcribingDir: sessionDir,
      transcribeError: null,
      transcribeErrorDir: null,
      remoteStage: "uploading",
      lastTranscriptPath: null,
    });
    pushJob("uploading");
    toast.info("Uploading to server…", { description: basename(sessionDir) });
    try {
      let state = await ipcSyncRecording(sessionDir);
      let guard = 0;
      while (
        state.remote_status !== "succeeded" &&
        state.remote_status !== "failed" &&
        guard < 600
      ) {
        const stage = stageOf(state);
        if (stage !== get().remoteStage) {
          set({ remoteStage: stage });
          pushJob(stage);
        }
        await new Promise((resolve) => setTimeout(resolve, 3000));
        state = await ipcSyncRecording(sessionDir);
        guard += 1;
      }
      if (state.remote_status === "succeeded") {
        set({
          transcribing: false,
          transcribingDir: null,
          remoteStage: null,
          lastTranscriptPath: `${sessionDir}/transcript.json`,
        });
        playFeedback("success");
        toast.success("Transcript synced from server", {
          description: basename(sessionDir),
        });
        void maybeAutoSummarize(sessionDir);
        void maybeAutoExtractTasks(sessionDir);
        void maybeAutoExtractMemories(sessionDir);
        void maybeAutoName(sessionDir);
      } else {
        const message = state.error ?? "remote transcription failed";
        set({
          transcribing: false,
          transcribingDir: null,
          remoteStage: null,
          transcribeError: message,
          transcribeErrorDir: sessionDir,
        });
        playFeedback("error");
        toast.error("Remote transcription failed", { description: message });
      }
    } catch (e) {
      const message = humanizeError(e);
      set({
        transcribing: false,
        transcribingDir: null,
        remoteStage: null,
        transcribeError: message,
        transcribeErrorDir: sessionDir,
      });
      playFeedback("error");
      toast.error("Sync failed", { description: message });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  const runTranscription = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;

    if (settings?.transcriber === "remote_server") {
      await runRemoteSync(sessionDir);
      return;
    }

    if ((settings?.transcriber ?? "local_whisper") === "local_whisper") {
      const modelStatus = await ipcWhisperModelStatus().catch(() => null);
      if (modelStatus && !modelStatus.present) {
        playFeedback("error");
        toast.error("Whisper model not downloaded", {
          description:
            "Download the local transcription model to transcribe on this Mac.",
          action: {
            label: "Open Settings",
            onClick: openTranscriptionSettings,
          },
        });
        return;
      }
    }

    if (settings?.transcriber === "openai") {
      const label = basename(sessionDir);
      const summary = await ipcGetRecording(label).catch(() => null);
      if (summary) {
        const estimate = estimateOpenAITranscribeCost({
          durationSeconds: Number(summary.duration_seconds ?? 0),
          micBytes: Number(summary.mic_bytes ?? 0),
          systemBytes: Number(summary.system_bytes ?? 0),
        });
        if (estimate.exceedsThreshold) {
          const proceed = await useCloudCostConfirmStore.getState().confirm({
            recordingLabel: label,
            estimate,
          });
          if (!proceed) {
            toast.info("Transcription cancelled", { description: label });
            return;
          }
        }
      }
    }

    const jobId = `transcribe:${sessionDir}`;
    set({
      transcribing: true,
      transcribingDir: sessionDir,
      transcribeError: null,
      transcribeErrorDir: null,
      lastTranscriptPath: null,
    });
    useJobsStore.getState().push({
      id: jobId,
      kind: "transcribe",
      label: `Transcribing ${basename(sessionDir)}`,
      sessionDir,
      recordingLabel: basename(sessionDir),
    });

    toast.info("Transcribing…", {
      description: basename(sessionDir),
    });
    try {
      if (settings?.auto_vad_enabled ?? true) {
        const vadJobId = `vad:${sessionDir}`;
        useJobsStore.getState().push({
          id: vadJobId,
          kind: "vad",
          label: `Detecting speech in ${basename(sessionDir)}`,
          sessionDir,
          recordingLabel: basename(sessionDir),
        });
        try {
          const vadResult = await ipcRunVad(sessionDir);
          const totalStripped = vadResult.channels.reduce(
            (acc, c) => acc + c.sidecar.silence_stripped_seconds,
            0
          );
          if (totalStripped >= 1) {
            const mins = Math.floor(totalStripped / 60);
            const secs = Math.round(totalStripped % 60);
            const human = mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
            toast.info("Speech detected", {
              description: `Stripped ${human} of silence before transcription.`,
            });
          }
        } catch (e) {
          console.error("run_vad failed (falling back to raw WAVs):", e);
        } finally {
          useJobsStore.getState().pop(vadJobId);
        }
      }
      const result = await ipcTranscribe(sessionDir);
      set({
        transcribing: false,
        transcribingDir: null,
        lastTranscriptPath: result.transcript_path,
      });

      if (settings?.diarization_enabled ?? true) {
        const diarizeJobId = `diarize:${sessionDir}`;
        useJobsStore.getState().push({
          id: diarizeJobId,
          kind: "diarize",
          label: `Identifying speakers in ${basename(sessionDir)}`,
          sessionDir,
          recordingLabel: basename(sessionDir),
        });
        try {
          const labeled = await ipcDiarize(sessionDir);
          if (labeled) {
            toast.info("Speakers identified", {
              description: basename(sessionDir),
            });
          }
        } catch (e) {
          console.error("diarize_session failed:", e);
        } finally {
          useJobsStore.getState().pop(diarizeJobId);
        }
      }
      const segments = result.session_transcript.channels.reduce(
        (acc, channel) => acc + channel.segments.length,
        0
      );
      const channelCount = result.session_transcript.channels.length;

      let savedHint = "";
      if (settings?.transcriber === "local_whisper") {
        const label = basename(sessionDir);
        const summary = await ipcGetRecording(label).catch(() => null);
        if (summary) {
          const est = estimateOpenAITranscribeCost({
            durationSeconds: Number(summary.duration_seconds ?? 0),
            micBytes: Number(summary.mic_bytes ?? 0),
            systemBytes: Number(summary.system_bytes ?? 0),
          });
          if (est.estimatedUsd > 0) {
            savedHint = ` · Local Whisper saved you ${formatUsd(est.estimatedUsd)}.`;
          }
        }
      }

      playFeedback("success");
      toast.success("Transcription complete", {
        description: `${segments} segments across ${channelCount} channel${channelCount === 1 ? "" : "s"} saved.${savedHint}`,
      });

      if (await shouldDeferOnPower()) {
        const power = await readPower();
        toast.info("Auto-AI deferred", {
          description: `Battery is ${formatBatteryPct(power.level)} and unplugged. Plug in to enable, or run manually.`,
          action: {
            label: "Run anyway",
            onClick: () => {
              void maybeAutoSummarize(sessionDir);
              void maybeAutoExtractTasks(sessionDir);
              void maybeAutoExtractMemories(sessionDir);
              void maybeAutoName(sessionDir);
            },
          },
        });
      } else {
        void maybeAutoSummarize(sessionDir);
        void maybeAutoExtractTasks(sessionDir);
        void maybeAutoExtractMemories(sessionDir);
        void maybeAutoName(sessionDir);
      }
    } catch (e) {
      const message = humanizeError(e);
      set({
        transcribing: false,
        transcribingDir: null,
        transcribeError: message,
        transcribeErrorDir: sessionDir,
      });
      const pointsToTranscriptionSettings = message
        .toLowerCase()
        .includes("settings → transcription");
      toast.error("Transcription failed", {
        description: message,
        ...(pointsToTranscriptionSettings
          ? { action: { label: "Open Settings", onClick: openTranscriptionSettings } }
          : {}),
      });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  return {
    recording: false,
    paused: false,
    startedAt: null,
    elapsed: 0,
    channels: [],
    error: null,
    busy: false,
    lastSavedDir: null,
    transcribing: false,
    transcribingDir: null,
    lastTranscriptPath: null,
    transcribeError: null,
    transcribeErrorDir: null,
    remoteStage: null,
    liveSessionDir: null,
    _tickerId: null,

    syncFromBackend: async () => {
      try {
        const status = await fetchStatus();

        if (!status.recording) {
          if (status.paused) {
            set({
              recording: false,
              paused: true,
              startedAt: null,
              elapsed: Number(status.elapsed_secs),
              channels: [],
              liveSessionDir: status.session_dir,
            });
          }
          return;
        }
        set({
          recording: true,
          paused: false,
          startedAt: Date.now() - Number(status.elapsed_secs) * 1000,
          elapsed: Number(status.elapsed_secs),
          channels: status.channels,
          liveSessionDir: status.session_dir,
        });
        installTicker();

        void ipcShowRecordingBar().catch(() => {});

        if (status.needs_segment) {
          void autoSegment();
        }
      } catch (e) {
        console.error("recording_store: initial sync failed", e);
      }
    },

    start: async (sessionDir?: string) => {
      set({
        busy: true,
        error: null,

        transcribeError: null,
        transcribeErrorDir: null,
        lastTranscriptPath: null,
      });
      try {
        const status = await ipcStart(sessionDir);
        set({
          recording: true,
          paused: false,
          startedAt: Date.now(),
          elapsed: 0,
          channels: status.channels,
          lastSavedDir: null,
          liveSessionDir: status.session_dir,
        });
        installTicker();

        void ipcShowRecordingBar().catch(() => {});
        const count = status.channels.length;
        playFeedback("start");
        toast.success("Recording started", {
          description:
            count === 0
              ? "No channels active yet"
              : `${count} channel${count === 1 ? "" : "s"} active: ${status.channels.join(", ")}`,
        });
      } catch (e) {
        const message = humanizeError(e);
        set({ error: message });
        playFeedback("error");
        toast.error("Could not start recording", { description: message });
      } finally {
        set({ busy: false });
      }
    },

    stop: async () => {
      const s = get();
      if (s.busy || (!s.recording && !s.paused)) return;

      const elapsedAtStop = s.elapsed;

      clearTicker();
      void ipcHideRecordingBar().catch(() => {});
      set({
        busy: true,
        error: null,
        recording: false,
        paused: false,
        startedAt: null,
        elapsed: 0,
        channels: [],
      });
      let sessionDir: string | null = null;
      try {
        const result = await ipcStop();
        sessionDir = result.artifacts.session_dir;
        set({ lastSavedDir: sessionDir, liveSessionDir: null });
        playFeedback("stop");
        toast.success("Recording saved", {
          description: `${formatDurationSeconds(elapsedAtStop)} · ${basename(sessionDir)}`,
        });
        navigateToNote(sessionDir);
      } catch (e) {
        const message = humanizeError(e);
        set({ error: message });
        playFeedback("error");
        toast.error("Could not stop recording", { description: message });

        void get().syncFromBackend();
      } finally {
        set({ busy: false });
      }

      if (!sessionDir) return;

      const settings = useSettingsStore.getState().settings;
      const autoEnabled = settings?.auto_transcribe_enabled ?? true;
      if (!autoEnabled) {
        return;
      }
      if (settings?.transcriber === "local_whisper") {
        void runTranscription(sessionDir);
        return;
      }
      if (settings?.transcriber === "openai") {
        void (async () => {
          if (await ipcHasOpenAiKey()) {
            void runTranscription(sessionDir);
          }
        })();
      }
      if (
        settings?.transcriber === "remote_server" &&
        (settings?.remote_auto_upload ?? false) &&
        !(settings?.privacy_mode ?? false)
      ) {
        void runTranscription(sessionDir);
      }
    },

    pause: async () => {
      const s = get();
      if (s.busy || !s.recording) return;

      clearTicker();
      void ipcSetTrayRecording(null);
      set({ busy: true, error: null, recording: false, paused: true, startedAt: null });
      try {
        const status = await ipcPause();
        set({
          elapsed: Number(status.elapsed_secs),
          channels: [],
          liveSessionDir: status.session_dir,
        });
        playFeedback("stop");
      } catch (e) {
        const message = humanizeError(e);
        set({ error: message });
        toast.error("Could not pause", { description: message });
        void get().syncFromBackend();
      } finally {
        set({ busy: false });
      }
    },

    resume: async () => {
      const s = get();
      if (s.busy || !s.paused) return;
      const resumeFrom = s.elapsed;

      set({
        busy: true,
        error: null,
        recording: true,
        paused: false,
        startedAt: Date.now() - resumeFrom * 1000,
      });
      installTicker();
      void ipcShowRecordingBar().catch(() => {});
      try {
        const status = await ipcResume();
        const elapsed = Number(status.elapsed_secs);
        set({
          startedAt: Date.now() - elapsed * 1000,
          elapsed,
          channels: status.channels,
          liveSessionDir: status.session_dir,
        });
        playFeedback("start");
      } catch (e) {
        const message = humanizeError(e);
        clearTicker();
        set({ error: message, recording: false, paused: true, startedAt: null });
        playFeedback("error");
        toast.error("Could not resume", { description: message });
        void get().syncFromBackend();
      } finally {
        set({ busy: false });
      }
    },

    transcribe: async (sessionDir: string) => {
      await runTranscription(sessionDir);
    },
  };
});
