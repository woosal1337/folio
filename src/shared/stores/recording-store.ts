/**
 * Cross-route recording session state.
 *
 * Replaces the previous `useRecording` hook so the recording controller
 * survives route changes (the Record page can unmount without losing the
 * timer or the in-flight session). The store mirrors the backend's
 * `RecordingStatus` and adds UI-only state: a high-resolution local
 * timer, busy/error flags, the last saved session directory, and the
 * post-stop transcription lifecycle.
 */

import { toast } from "sonner";
import { create } from "zustand";

import {
  getRecording as ipcGetRecording,
  recordingStatus as fetchStatus,
  runAgent as ipcRunAgent,
  startRecording as ipcStart,
  stopRecording as ipcStop,
  transcribeRecording as ipcTranscribe,
} from "@/shared/lib/ipc";
import { estimateOpenAITranscribeCost, formatUsd } from "@/shared/lib/cost-estimate";
import { playFeedback } from "@/shared/lib/feedback";
import { formatBatteryPct, readPower, shouldDeferOnPower } from "@/shared/lib/power";
import { useCloudCostConfirmStore } from "@/shared/stores/cloud-cost-confirm-store";
import { useJobsStore } from "@/shared/stores/jobs-store";
import { useMemoriesStore } from "@/shared/stores/memories-store";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { useTasksStore } from "@/shared/stores/tasks-store";

interface RecordingState {
  recording: boolean;
  /** Wall-clock ms when the current session started, or null if idle. */
  startedAt: number | null;
  /** Whole-seconds elapsed in the current session. */
  elapsed: number;
  /** Channels reported by the backend (e.g. ["mic", "system"]). */
  channels: string[];
  /** Last error message surfaced to the UI, or null. */
  error: string | null;
  /** True while an IPC call is in flight. */
  busy: boolean;
  /** Session directory of the most recently stopped recording. */
  lastSavedDir: string | null;

  /** True while an auto-transcription job is in flight. */
  transcribing: boolean;
  /**
   * Session directory of the recording currently being transcribed, or
   * null if no job is active. Lets the UI show a spinner on a specific
   * row without conflating it with `lastSavedDir`.
   */
  transcribingDir: string | null;
  /** Last transcript JSON path written, or null. */
  lastTranscriptPath: string | null;
  /** Last transcription error, or null. */
  transcribeError: string | null;

  /** Internal: interval handle for the local ticker. */
  _tickerId: number | null;

  /** First-mount: ask the backend whether a session is already running. */
  syncFromBackend: () => Promise<void>;
  /** Start a new recording session. */
  start: () => Promise<void>;
  /** Stop the current recording session. Auto-transcribes if configured. */
  stop: () => Promise<void>;
  /** Transcribe an existing session on demand. */
  transcribe: (sessionDir: string) => Promise<void>;
}

export const useRecording = create<RecordingState>((set, get) => {
  const tick = () => {
    const { startedAt, recording } = get();
    if (!recording || startedAt === null) return;
    set({ elapsed: Math.floor((Date.now() - startedAt) / 1000) });
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
  };

  // Pull the trailing component of a path, cross-platform. Used by the
  // toast descriptions so we surface "2026-05-23-19-15-22" rather than
  // the full /Users/…/Recordings/2026-05-23-19-15-22 mouthful.
  const basename = (path: string): string => {
    const trimmed = path.replace(/[\\/]+$/, "");
    const idx = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
    return idx === -1 ? trimmed : trimmed.slice(idx + 1);
  };

  // Format seconds as "M:SS" for toast descriptions on stop. Mirrors
  // the formatter used elsewhere in the UI but kept local so the
  // store has no UI-layer dep.
  const formatDurationSeconds = (s: number): string => {
    const safe = Math.max(0, Math.floor(s));
    const m = Math.floor(safe / 60);
    const sec = safe % 60;
    return `${m}:${sec.toString().padStart(2, "0")}`;
  };

  // Try to auto-run the Summarize agent after a transcription completes.
  // Silently no-ops when auto-summarize is off or no OpenAI key is set —
  // we don't want to nag the user with toast errors for opt-out behaviour.
  // Pushes its own pill into the jobs store so the user sees what's
  // running.
  const maybeAutoSummarize = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;
    if (!settings) return;
    if (!settings.auto_summarize_enabled) return;
    if (!settings.openai_api_key || settings.openai_api_key.trim().length === 0) {
      // No AI key configured — the user can still summarize manually
      // from the editor; the AgentPanel's hint links straight to
      // Settings → AI for them.
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
      // Non-fatal — the manual button in the editor still works.
      console.error("auto-summarize failed:", e);
      toast.error("Auto-summary failed", { description: String(e) });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  // Try to auto-run the Extract Memories agent after a transcription
  // completes. Same gating + job-pill pattern as auto-summarize/tasks.
  // Memories the agent writes via the `remember` tool land on the
  // Memory page; on success we refresh the memories store so the UI
  // updates immediately if the user has /memory open.
  const maybeAutoExtractMemories = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;
    if (!settings) return;
    if (!settings.auto_extract_memories_enabled) return;
    if (!settings.openai_api_key || settings.openai_api_key.trim().length === 0) {
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
      void useMemoriesStore.getState().refresh();
      toast.success("Memories captured", { description: basename(sessionDir) });
    } catch (e) {
      console.error("auto-extract-memories failed:", e);
      toast.error("Auto-extract memories failed", { description: String(e) });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  // Try to auto-run the Auto-name agent after a transcription
  // completes. Same gating + job-pill pattern as the other auto-fires.
  // The agent's response is a JSON object with title / tags /
  // subtitle; we don't surface a toast on success because the
  // suggestion shows up directly in the Library list — that's the UX
  // the v2 finding 024 calls for. Errors are still toasted so the
  // user can see when the auto-fire silently broke.
  const maybeAutoName = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;
    if (!settings) return;
    if (!settings.auto_name_enabled) return;
    if (!settings.openai_api_key || settings.openai_api_key.trim().length === 0) {
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
      // No success toast — the suggestion appears in the Library row
      // on the next list refresh, which is the UX the v2 finding
      // explicitly asks for ("apply silently").
    } catch (e) {
      console.error("auto-name failed:", e);
      toast.error("Auto-name failed", { description: String(e) });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  // Try to auto-run the Extract Tasks agent after a transcription
  // completes. Same gating as auto-summarize: opt-in via settings, no
  // toast on the skipped path. The agent writes via the `create_task`
  // tool, so success here means new cards appear on the kanban —
  // refresh the tasks store so they show up wherever the user is.
  const maybeAutoExtractTasks = async (sessionDir: string) => {
    const settings = useSettingsStore.getState().settings;
    if (!settings) return;
    if (!settings.auto_extract_tasks_enabled) return;
    if (!settings.openai_api_key || settings.openai_api_key.trim().length === 0) {
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
      // Refresh so the Tasks page picks up the new cards immediately
      // if the user happens to be looking at it.
      void useTasksStore.getState().refresh();
      toast.success("Tasks ready", { description: basename(sessionDir) });
    } catch (e) {
      console.error("auto-extract-tasks failed:", e);
      toast.error("Auto-extract tasks failed", { description: String(e) });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  // Self-contained transcription routine so both `stop` (auto) and an
  // explicit `transcribe(...)` call route through the same lifecycle.
  const runTranscription = async (sessionDir: string) => {
    // Before kicking off, ask for confirmation if the OpenAI Whisper
    // path is about to send a sizeable WAV upstream. Below the
    // threshold (small / cheap meetings) we don't bother the user.
    // Local Whisper has no cost, so it bypasses this check.
    const settings = useSettingsStore.getState().settings;
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
    });
    useJobsStore.getState().push({
      id: jobId,
      kind: "transcribe",
      label: `Transcribing ${basename(sessionDir)}`,
      sessionDir,
      recordingLabel: basename(sessionDir),
    });
    // Inform the user the async job kicked off. Useful when they
    // navigate away from the row that shows the spinner — the toast
    // is the only persistent signal that work is happening.
    toast.info("Transcribing…", {
      description: basename(sessionDir),
    });
    try {
      const result = await ipcTranscribe(sessionDir);
      set({
        transcribing: false,
        transcribingDir: null,
        lastTranscriptPath: result.transcript_path,
      });
      const segments = result.session_transcript.channels.reduce(
        (acc, channel) => acc + channel.segments.length,
        0
      );
      const channelCount = result.session_transcript.channels.length;

      // For Local Whisper transcriptions: surface "Local Whisper
      // saved you $X" so the user feels the dollar value of the
      // local path. We reuse the same cost estimator the cloud
      // confirm modal uses (#055). v2 finding 093.
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
      // Chain into auto-summarize, auto-extract-tasks, and
      // auto-extract-memories once the transcript has landed. We
      // don't await — `runTranscription`'s caller doesn't care about
      // the post-processing outcome; the jobs strip + the editor
      // page surface progress and results. All three run in
      // parallel: they hit the same provider, but the requests are
      // independent and we'd rather they finish sooner.
      //
      // v2 finding 065: skip the three when the laptop is on
      // battery + below the low threshold. Surface a toast with a
      // Run-anyway action so the user can override per-recording.
      // The check is best-effort (Web Battery API only); on any
      // ambiguity we run the work.
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
      const message = String(e);
      set({
        transcribing: false,
        transcribingDir: null,
        transcribeError: message,
      });
      toast.error("Transcription failed", { description: message });
    } finally {
      useJobsStore.getState().pop(jobId);
    }
  };

  return {
    recording: false,
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
    _tickerId: null,

    syncFromBackend: async () => {
      try {
        const status = await fetchStatus();
        if (!status.recording) return;
        set({
          recording: true,
          startedAt: Date.now() - Number(status.elapsed_secs) * 1000,
          elapsed: Number(status.elapsed_secs),
          channels: status.channels,
        });
        installTicker();
      } catch (e) {
        console.error("recording_store: initial sync failed", e);
      }
    },

    start: async () => {
      set({
        busy: true,
        error: null,
        // Reset transcription state from any prior session so the loader
        // doesn't bleed across recordings.
        transcribeError: null,
        lastTranscriptPath: null,
      });
      try {
        const status = await ipcStart();
        set({
          recording: true,
          startedAt: Date.now(),
          elapsed: 0,
          channels: status.channels,
          lastSavedDir: null,
        });
        installTicker();
        const count = status.channels.length;
        playFeedback("start");
        toast.success("Recording started", {
          description:
            count === 0
              ? "No channels active yet"
              : `${count} channel${count === 1 ? "" : "s"} active: ${status.channels.join(", ")}`,
        });
      } catch (e) {
        const message = String(e);
        set({ error: message });
        playFeedback("error");
        toast.error("Could not start recording", { description: message });
      } finally {
        set({ busy: false });
      }
    },

    stop: async () => {
      set({ busy: true, error: null });
      // Snapshot duration before we reset elapsed, so the toast can
      // surface "0:42" instead of always saying "0:00".
      const elapsedAtStop = get().elapsed;
      let sessionDir: string | null = null;
      try {
        const result = await ipcStop();
        sessionDir = result.artifacts.session_dir;
        clearTicker();
        set({
          recording: false,
          startedAt: null,
          elapsed: 0,
          channels: [],
          lastSavedDir: sessionDir,
        });
        playFeedback("stop");
        toast.success("Recording saved", {
          description: `${formatDurationSeconds(elapsedAtStop)} · ${basename(sessionDir)}`,
        });
      } catch (e) {
        const message = String(e);
        set({ error: message });
        playFeedback("error");
        toast.error("Could not stop recording", { description: message });
      } finally {
        set({ busy: false });
      }

      if (!sessionDir) return;

      // Decide whether to auto-transcribe. We read settings from the
      // settings store rather than re-fetching them on every stop —
      // the store is loaded once on app startup. Honours the
      // `auto_transcribe_enabled` toggle and falls back to manual
      // transcription when the selected provider isn't usable
      // (OpenAI without a key, anything else just runs).
      const settings = useSettingsStore.getState().settings;
      const providerUsable =
        settings?.transcriber === "local_whisper" ||
        (settings?.transcriber === "openai" &&
          settings.openai_api_key.trim().length > 0);
      const shouldTranscribe =
        (settings?.auto_transcribe_enabled ?? true) && providerUsable;
      if (shouldTranscribe) {
        // Fire-and-forget. `runTranscription` flips `transcribing` so
        // the UI shows a spinner on the row; we deliberately do not
        // await so the Stop click resolves as soon as the WAV is
        // saved and the user can keep using the app.
        void runTranscription(sessionDir);
      }
    },

    transcribe: async (sessionDir: string) => {
      await runTranscription(sessionDir);
    },
  };
});
