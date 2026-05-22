/**
 * Cross-route recording session state.
 *
 * Replaces the previous `useRecording` hook so the recording controller
 * survives route changes (the Record page can unmount without losing the
 * timer or the in-flight session). The store mirrors the backend's
 * `RecordingStatus` and adds UI-only state: a high-resolution local
 * timer, busy/error flags, and the last saved session directory.
 */

import { create } from "zustand";

import {
  recordingStatus as fetchStatus,
  startRecording as ipcStart,
  stopRecording as ipcStop,
} from "@/shared/lib/ipc";

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
  /** Internal: interval handle for the local ticker. */
  _tickerId: number | null;

  /** First-mount: ask the backend whether a session is already running. */
  syncFromBackend: () => Promise<void>;
  /** Start a new recording session. */
  start: () => Promise<void>;
  /** Stop the current recording session. */
  stop: () => Promise<void>;
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

  return {
    recording: false,
    startedAt: null,
    elapsed: 0,
    channels: [],
    error: null,
    busy: false,
    lastSavedDir: null,
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
      set({ busy: true, error: null });
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
      try {
        const result = await ipcStop();
        clearTicker();
        set({
          recording: false,
          startedAt: null,
          elapsed: 0,
          channels: [],
          lastSavedDir: result.artifacts.session_dir,
        });
      } catch (e) {
        set({ error: String(e) });
      } finally {
        set({ busy: false });
      }
    },
  };
});
