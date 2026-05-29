/**
 * Floating recording-control bar.
 *
 * Renders inside the frameless, always-on-top `recording-bar` window the
 * backend opens while a capture is in progress. Gives the user an
 * always-visible recording indicator (pulsing red dot), the live elapsed
 * time, and a Stop button — no matter which app is focused. The whole bar
 * is a drag handle so it can be parked anywhere; the Stop button routes
 * through the main window's stop flow so auto-transcribe + toasts still
 * fire.
 *
 * State comes from polling `recording_status` (the bar is a separate
 * webview with its own JS context, so it can't read the main window's
 * Zustand store). When the backend reports idle the bar self-closes as a
 * safety net in case the main window didn't get to hide it.
 */

import * as React from "react";
import { GripHorizontal, Square } from "lucide-react";

import {
  hideRecordingBar,
  recordingBarStop,
  recordingStatus,
  startWindowDrag,
} from "@/shared/lib/ipc";

/** Poll cadence for the live elapsed/paused state. */
const POLL_MS = 500;

function formatElapsed(secs: number): string {
  const safe = Math.max(0, Math.floor(secs));
  const m = Math.floor(safe / 60);
  const s = safe % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export default function RecordingBar() {
  const [elapsed, setElapsed] = React.useState(0);
  const [paused, setPaused] = React.useState(false);
  const [stopping, setStopping] = React.useState(false);

  // The window is transparent (see show_recording_bar). The app's <body>
  // ships an opaque `bg-background`, which would fill the window's square
  // corners and defeat the rounded pill — so blank out the page chrome in
  // this dedicated window and let only the capsule paint.
  React.useEffect(() => {
    const els = [document.documentElement, document.body];
    const prev = els.map((el) => el.style.background);
    els.forEach((el) => {
      el.style.background = "transparent";
    });
    return () => {
      els.forEach((el, i) => {
        el.style.background = prev[i] ?? "";
      });
    };
  }, []);

  // Poll the backend for the live capture state.
  React.useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const status = await recordingStatus();
        if (cancelled) return;
        setElapsed(Number(status.elapsed_secs));
        setPaused(status.paused);
        // Idle and not paused → the capture is fully over. Close the bar
        // in case the main window never hid it (e.g. it was closed).
        if (!status.recording && !status.paused && !stopping) {
          void hideRecordingBar().catch(() => {});
        }
      } catch {
        // Transient IPC error — keep the last shown value.
      }
    };
    void poll();
    const id = window.setInterval(poll, POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [stopping]);

  const onStop = React.useCallback(() => {
    setStopping(true);
    void recordingBarStop().catch((e) => {
      console.error("recording_bar_stop:", e);
      setStopping(false);
    });
  }, []);

  // Whole-bar drag: start a window drag on press unless the press lands on
  // the Stop button (or another interactive control).
  const onMouseDown = React.useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest("button")) return;
    e.preventDefault();
    void startWindowDrag().catch((err) => console.error("startWindowDrag:", err));
  }, []);

  return (
    // eslint-disable-next-line jsx-a11y/no-static-element-interactions -- frameless-window drag region, same pattern as the main shell.
    <div
      onMouseDown={onMouseDown}
      className="fixed inset-0 flex select-none flex-col items-center justify-between overflow-hidden rounded-[20px] border border-white/10 bg-neutral-900/95 py-2.5 text-white shadow-2xl backdrop-blur"
    >
      {/* Drag grip — the explicit "grab here" affordance, set apart from
          the controls so it reads as a handle (the whole pill drags). */}
      <GripHorizontal
        aria-hidden="true"
        className="h-3.5 w-3.5 shrink-0 text-neutral-500"
      />

      {/* Recording / paused indicator. */}
      <span
        className="relative flex h-4 w-4 shrink-0 items-center justify-center"
        title={paused ? "Paused" : "Recording"}
      >
        {paused ? (
          <span className="h-2 w-2 rounded-full bg-amber-400" />
        ) : (
          <>
            <span className="absolute h-3.5 w-3.5 animate-ping rounded-full bg-red-500/40" />
            <span className="h-2 w-2 rounded-full bg-red-500" />
          </>
        )}
      </span>

      {/* Elapsed time. */}
      <p className="shrink-0 font-mono text-[10px] font-semibold tabular-nums text-neutral-200">
        {formatElapsed(elapsed)}
      </p>

      {/* Stop — icon only, but labelled for screen readers + hover. */}
      <button
        type="button"
        onClick={onStop}
        disabled={stopping}
        aria-label="Stop recording"
        title="Stop recording"
        className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-red-500 text-white transition-colors hover:bg-red-600 disabled:opacity-60"
      >
        <Square className="h-3.5 w-3.5 fill-current" />
      </button>
    </div>
  );
}
