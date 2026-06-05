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

const POLL_MS = 500;

const IDLE_HIDE_TICKS = 20;

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

  const [micSilent, setMicSilent] = React.useState(false);

  const [transitioning, setTransitioning] = React.useState(false);
  const pendingRef = React.useRef<{ target: boolean; ticks: number } | null>(null);

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

  React.useEffect(() => {
    let cancelled = false;

    let idleTicks = 0;
    const poll = async () => {
      try {
        const status = await recordingStatus();
        if (cancelled) return;
        setElapsed(Number(status.elapsed_secs));
        setMicSilent(status.mic_silent ?? false);

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

        idleTicks += 1;
        if (idleTicks >= IDLE_HIDE_TICKS && !stopping) {
          void hideRecordingBar().catch(() => {});
        }
      } catch {}
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
      <GripHorizontal
        aria-hidden="true"
        className="h-3.5 w-3.5 shrink-0 text-neutral-500"
      />

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

      <p className="shrink-0 font-mono text-[10px] font-semibold tabular-nums text-neutral-200">
        {formatElapsed(elapsed)}
      </p>

      {micSilent ? (
        <span
          className="h-2 w-2 shrink-0 rounded-full bg-amber-400"
          title="Mic level is very low — check that your microphone isn't muted and its input gain is up in System Settings → Sound → Input."
          aria-label="Mic silent warning"
        />
      ) : null}

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
