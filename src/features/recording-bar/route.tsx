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
import { GripHorizontal, Pause, Play, Square } from "lucide-react";

import {
  hideRecordingBar,
  recordingBarPause,
  recordingBarResume,
  recordingBarStop,
  recordingStatus,
  startWindowDrag,
} from "@/shared/lib/ipc";

/** Poll cadence for the live elapsed/paused state. */
const POLL_MS = 500;
/** Consecutive fully-idle polls before the bar self-closes as a safety
 *  net. A real stop closes the bar instantly via the store; this only
 *  catches an orphaned bar (app went away without hiding us). Set high
 *  (~10s) because pausing looks idle while the segment WAV finalizes —
 *  which can take a few seconds for a long recording — and that must
 *  never be mistaken for "done". */
const IDLE_HIDE_TICKS = 20;
/** Max polls to hold an optimistic pause/resume state before giving up and
 *  trusting the backend again (safety valve if the action never lands). */
const PENDING_MAX_TICKS = 8;

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
  // While a pause/resume is in flight the backend briefly reports an
  // in-between state (segment tearing down / new segment spinning up). Hold
  // the optimistic value and ignore the poll until the backend matches the
  // target, so the icon doesn't flip back and forth mid-transition.
  const [transitioning, setTransitioning] = React.useState(false);
  const pendingRef = React.useRef<{ target: boolean; ticks: number } | null>(null);

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
    // Self-close is only a safety net for "the app went away without
    // hiding us" — the real teardown is the store's hideRecordingBar() on
    // stop. Pausing briefly looks idle (the segment is torn down a beat
    // before the paused-note is recorded), so require the idle state to
    // *persist* before closing; otherwise a pause would wrongly kill the
    // bar. ~IDLE_HIDE_TICKS × POLL_MS of sustained idle = genuinely over.
    let idleTicks = 0;
    const poll = async () => {
      try {
        const status = await recordingStatus();
        if (cancelled) return;
        setElapsed(Number(status.elapsed_secs));
        // Reconcile the paused indicator, but respect an in-flight
        // pause/resume: keep the optimistic value until the backend
        // reaches the target (or we hit the safety-valve tick count).
        const pending = pendingRef.current;
        if (pending) {
          if (status.paused === pending.target || pending.ticks >= PENDING_MAX_TICKS) {
            pendingRef.current = null;
            setPaused(status.paused);
            setTransitioning(false);
          } else {
            pending.ticks += 1;
          }
        } else {
          setPaused(status.paused);
        }
        if (status.recording || status.paused || pendingRef.current) {
          idleTicks = 0;
          return;
        }
        // Fully idle (neither recording nor paused). Don't count the brief
        // window right after the user hit Stop here — that path closes us
        // explicitly via the store.
        idleTicks += 1;
        if (idleTicks >= IDLE_HIDE_TICKS && !stopping) {
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

  // Pause when recording, resume when paused. Flip optimistically and mark
  // the transition pending so the poll won't bounce the icon back until the
  // backend reaches the target state.
  const onPauseResume = React.useCallback(() => {
    if (transitioning) return;
    const wasPaused = paused;
    const target = !wasPaused;
    pendingRef.current = { target, ticks: 0 };
    setPaused(target);
    setTransitioning(true);
    const action = wasPaused ? recordingBarResume : recordingBarPause;
    void action().catch((e) => {
      console.error("recording_bar_pause/resume:", e);
      pendingRef.current = null;
      setPaused(wasPaused);
      setTransitioning(false);
    });
  }, [paused, transitioning]);

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

      {/* Controls: pause/resume (neutral) above stop (red). */}
      <div className="flex shrink-0 flex-col items-center gap-2">
        <button
          type="button"
          onClick={onPauseResume}
          disabled={stopping}
          aria-label={paused ? "Resume recording" : "Pause recording"}
          title={paused ? "Resume recording" : "Pause recording"}
          className="inline-flex h-8 w-8 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20 disabled:opacity-60"
        >
          {paused ? (
            <Play className="h-3.5 w-3.5 fill-current" />
          ) : (
            <Pause className="h-3.5 w-3.5 fill-current" />
          )}
        </button>

        {/* Stop — icon only, but labelled for screen readers + hover. */}
        <button
          type="button"
          onClick={onStop}
          disabled={stopping}
          aria-label="Stop recording"
          title="Stop recording"
          className="inline-flex h-8 w-8 items-center justify-center rounded-full bg-red-500 text-white transition-colors hover:bg-red-600 disabled:opacity-60"
        >
          <Square className="h-3.5 w-3.5 fill-current" />
        </button>
      </div>
    </div>
  );
}
