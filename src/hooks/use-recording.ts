import * as React from "react";

import {
  recordingStatus,
  startRecording as apiStart,
  stopRecording as apiStop,
} from "@/lib/api";

/** UI-side recording controller. Owns the timer locally so the elapsed
 *  display updates smoothly without polling the backend every tick. On
 *  mount, syncs with the backend in case a session is already in flight
 *  (e.g. after a window reload while recording). */
export function useRecording() {
  const [recording, setRecording] = React.useState(false);
  const [startedAt, setStartedAt] = React.useState<number | null>(null);
  const [elapsed, setElapsed] = React.useState(0);
  const [channels, setChannels] = React.useState<string[]>([]);
  const [error, setError] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const [lastSavedDir, setLastSavedDir] = React.useState<string | null>(null);

  // Local timer ticking each 250 ms while recording.
  React.useEffect(() => {
    if (!recording || startedAt === null) return;
    const id = window.setInterval(() => {
      setElapsed(Math.floor((Date.now() - startedAt) / 1000));
    }, 250);
    return () => window.clearInterval(id);
  }, [recording, startedAt]);

  // First-mount sync with the backend.
  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const status = await recordingStatus();
        if (cancelled) return;
        if (status.recording) {
          setRecording(true);
          setStartedAt(Date.now() - status.elapsed_secs * 1000);
          setChannels(status.channels);
        }
      } catch (e) {
        console.error("initial recording_status:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const start = React.useCallback(async () => {
    setError(null);
    setBusy(true);
    try {
      const status = await apiStart();
      setRecording(true);
      setStartedAt(Date.now());
      setElapsed(0);
      setChannels(status.channels);
      setLastSavedDir(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  const stop = React.useCallback(async () => {
    setError(null);
    setBusy(true);
    try {
      const result = await apiStop();
      setRecording(false);
      setStartedAt(null);
      setElapsed(0);
      setChannels([]);
      setLastSavedDir(result.artifacts.session_dir);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  return {
    recording,
    elapsed,
    channels,
    error,
    busy,
    lastSavedDir,
    start,
    stop,
  };
}
