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
  recordingStatus as fetchStatus,
  startRecording as ipcStart,
  stopRecording as ipcStop,
  transcribeRecording as ipcTranscribe,
} from "@/shared/lib/ipc";
import { useSettingsStore } from "@/shared/stores/settings-store";

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

  // Self-contained transcription routine so both `stop` (auto) and an
  // explicit `transcribe(...)` call route through the same lifecycle.
  const runTranscription = async (sessionDir: string) => {
    set({
      transcribing: true,
      transcribingDir: sessionDir,
      transcribeError: null,
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
      toast.success("Transcription complete", {
        description: `${segments} segments across ${channelCount} channel${channelCount === 1 ? "" : "s"} saved.`,
      });
    } catch (e) {
      const message = String(e);
      set({
        transcribing: false,
        transcribingDir: null,
        transcribeError: message,
      });
      toast.error("Transcription failed", { description: message });
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
      } catch (e) {
        set({ error: String(e) });
      } finally {
        set({ busy: false });
      }
    },

    stop: async () => {
      set({ busy: true, error: null });
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
      } catch (e) {
        set({ error: String(e) });
      } finally {
        set({ busy: false });
      }

      if (!sessionDir) return;

      // Decide whether to auto-transcribe. We read settings from the
      // settings store rather than re-fetching them on every stop —
      // the store is loaded once on app startup.
      const settings = useSettingsStore.getState().settings;
      const shouldTranscribe =
        settings?.transcriber === "openai" && settings.openai_api_key.trim().length > 0;
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
