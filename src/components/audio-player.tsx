import * as React from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Pause, Play } from "lucide-react";

import { cn, formatDuration } from "@/lib/utils";

/** Module-level coordination so kicking off one player pauses every other
 *  audio element currently rendered in the app. */
let currentAudio: HTMLAudioElement | null = null;
function takeFocus(el: HTMLAudioElement) {
  if (currentAudio && currentAudio !== el && !currentAudio.paused) {
    currentAudio.pause();
  }
  currentAudio = el;
}

interface AudioPlayerProps {
  /** Absolute path to a local WAV / audio file. */
  filePath: string;
  /** Optional small label rendered to the left of the scrubber. */
  label?: string;
  className?: string;
}

export function AudioPlayer({ filePath, label, className }: AudioPlayerProps) {
  const audioRef = React.useRef<HTMLAudioElement>(null);
  const [playing, setPlaying] = React.useState(false);
  const [duration, setDuration] = React.useState(0);
  const [current, setCurrent] = React.useState(0);
  const [ready, setReady] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  // Convert the OS path to a Tauri asset:// URL the webview can load.
  const src = React.useMemo(() => {
    try {
      return convertFileSrc(filePath);
    } catch (e) {
      console.error("convertFileSrc:", e);
      return "";
    }
  }, [filePath]);

  React.useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    setReady(false);
    setError(null);

    const onLoaded = () => {
      // Some browsers report Infinity until they sniff the WAV further.
      const d = Number.isFinite(audio.duration) ? audio.duration : 0;
      setDuration(d);
      setReady(true);
    };
    const onTime = () => setCurrent(audio.currentTime);
    const onEnd = () => {
      setPlaying(false);
      setCurrent(audio.duration);
    };
    const onPlay = () => setPlaying(true);
    const onPause = () => setPlaying(false);
    const onError = () => {
      const mediaErr = audio.error;
      const msg = mediaErr ? `media error ${mediaErr.code}` : "playback error";
      setError(msg);
      console.error("audio error:", filePath, mediaErr);
    };

    audio.addEventListener("loadedmetadata", onLoaded);
    audio.addEventListener("durationchange", onLoaded);
    audio.addEventListener("timeupdate", onTime);
    audio.addEventListener("ended", onEnd);
    audio.addEventListener("play", onPlay);
    audio.addEventListener("pause", onPause);
    audio.addEventListener("error", onError);
    return () => {
      audio.removeEventListener("loadedmetadata", onLoaded);
      audio.removeEventListener("durationchange", onLoaded);
      audio.removeEventListener("timeupdate", onTime);
      audio.removeEventListener("ended", onEnd);
      audio.removeEventListener("play", onPlay);
      audio.removeEventListener("pause", onPause);
      audio.removeEventListener("error", onError);
    };
  }, [filePath]);

  const toggle = React.useCallback(async () => {
    const audio = audioRef.current;
    if (!audio) return;
    if (audio.paused) {
      takeFocus(audio);
      try {
        await audio.play();
      } catch (e) {
        console.error("audio.play():", e);
        setError(String(e));
      }
    } else {
      audio.pause();
    }
  }, []);

  const seek = React.useCallback(
    (fraction: number) => {
      const audio = audioRef.current;
      if (!audio || !duration) return;
      audio.currentTime = Math.max(0, Math.min(1, fraction)) * duration;
    },
    [duration]
  );

  return (
    <div className={cn("flex items-center gap-3", className)}>
      <audio ref={audioRef} src={src} preload="metadata" />

      <button
        onClick={toggle}
        disabled={!ready && !error}
        aria-label={playing ? "Pause" : "Play"}
        className={cn(
          "inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-border bg-card text-foreground transition-colors",
          "hover:bg-accent hover:text-accent-foreground disabled:opacity-40 disabled:cursor-not-allowed",
          playing && "bg-primary text-primary-foreground border-primary hover:bg-primary/90"
        )}
      >
        {playing ? (
          <Pause className="h-4 w-4 fill-current" />
        ) : (
          <Play className="ml-0.5 h-4 w-4 fill-current" />
        )}
      </button>

      {label && (
        <span className="w-16 shrink-0 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
          {label}
        </span>
      )}

      <Scrubber
        current={current}
        duration={duration}
        onSeek={seek}
        disabled={!ready}
      />

      <span className="w-24 shrink-0 text-right font-mono text-2xs text-muted-foreground tabular-nums">
        {formatDuration(current)} / {formatDuration(duration)}
      </span>

      {error && (
        <span className="text-2xs text-destructive">{error}</span>
      )}
    </div>
  );
}

interface ScrubberProps {
  current: number;
  duration: number;
  onSeek: (fraction: number) => void;
  disabled?: boolean;
}

function Scrubber({ current, duration, onSeek, disabled }: ScrubberProps) {
  const trackRef = React.useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = React.useState(false);
  const [hoverFrac, setHoverFrac] = React.useState<number | null>(null);

  const fraction = duration > 0 ? current / duration : 0;
  const displayFrac = dragging && hoverFrac !== null ? hoverFrac : fraction;

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (disabled) return;
    const f = compute(e, trackRef);
    if (f !== null) {
      onSeek(f);
      setDragging(true);
      setHoverFrac(f);
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    }
  };
  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (disabled) return;
    const f = compute(e, trackRef);
    if (f === null) return;
    if (dragging) {
      onSeek(f);
      setHoverFrac(f);
    }
  };
  const onPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!dragging) return;
    setDragging(false);
    setHoverFrac(null);
    try {
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // ignore
    }
  };

  return (
    <div
      ref={trackRef}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      className={cn(
        "relative h-4 flex-1 cursor-pointer select-none",
        disabled && "cursor-not-allowed opacity-50"
      )}
    >
      {/* Track */}
      <div className="absolute left-0 right-0 top-1/2 h-1 -translate-y-1/2 rounded-full bg-secondary" />
      {/* Fill */}
      <div
        className="absolute left-0 top-1/2 h-1 -translate-y-1/2 rounded-full bg-primary"
        style={{ width: `${displayFrac * 100}%` }}
      />
      {/* Thumb */}
      <div
        className="absolute top-1/2 h-3 w-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-primary bg-card shadow-sm"
        style={{ left: `${displayFrac * 100}%` }}
      />
    </div>
  );
}

function compute(
  e: React.PointerEvent<HTMLDivElement>,
  ref: React.RefObject<HTMLDivElement | null>
): number | null {
  const rect = ref.current?.getBoundingClientRect();
  if (!rect || rect.width === 0) return null;
  const x = e.clientX - rect.left;
  return Math.max(0, Math.min(1, x / rect.width));
}
