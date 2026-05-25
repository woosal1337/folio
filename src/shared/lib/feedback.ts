/**
 * Subtle audio feedback on recording / agent lifecycle events. Uses
 * Web Audio API to synthesise short tones inline so we don't bundle
 * AIFF files. Default-off — users discover via Settings → General →
 * Feedback. v2 roadmap finding 019.
 *
 * Honours prefers-reduced-motion AND a Settings flag. Plays silently
 * (no-op) when AudioContext isn't available.
 */

import { useSettingsStore } from "@/shared/stores/settings-store";

export type FeedbackKind =
  /** Recording started — quick rising chirp. */
  | "start"
  /** Recording stopped — descending two-note. */
  | "stop"
  /** Background success (agent finished, memory saved) — single soft bell. */
  | "success"
  /** Cancellation / dismissal — quick low bloop. */
  | "dismiss"
  /** Error toast counterpart — minor third dyad. */
  | "error";

interface TonePart {
  freq: number;
  /** Seconds from start of the sequence. */
  start: number;
  /** Seconds the note sustains. */
  duration: number;
  /** Peak gain, 0-1. Default 0.18 (~ -15 dBFS) so we don't startle. */
  gain?: number;
}

const SEQUENCES: Record<FeedbackKind, TonePart[]> = {
  start: [
    { freq: 660, start: 0, duration: 0.08 },
    { freq: 990, start: 0.06, duration: 0.1 },
  ],
  stop: [
    { freq: 880, start: 0, duration: 0.08 },
    { freq: 660, start: 0.08, duration: 0.12 },
  ],
  success: [
    { freq: 1320, start: 0, duration: 0.05, gain: 0.12 },
    { freq: 1760, start: 0.05, duration: 0.18, gain: 0.14 },
  ],
  dismiss: [{ freq: 220, start: 0, duration: 0.08, gain: 0.1 }],
  error: [
    { freq: 440, start: 0, duration: 0.12 },
    { freq: 523, start: 0, duration: 0.12 },
  ],
};

let ctx: AudioContext | null = null;

function context(): AudioContext | null {
  if (typeof window === "undefined") return null;
  if (ctx) return ctx;
  try {
    const Ctor =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext?: typeof AudioContext })
        .webkitAudioContext;
    if (!Ctor) return null;
    ctx = new Ctor();
    return ctx;
  } catch {
    return null;
  }
}

/**
 * Whether the user opted into feedback sounds. Read from settings
 * on every call so toggling in Settings takes effect immediately.
 */
function isEnabled(): boolean {
  const s = useSettingsStore.getState().settings;
  if (!s) return false;
  if (!s.feedback_sounds_enabled) return false;
  if (
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches
  ) {
    // Belt-and-braces: respect the OS-level reduce-motion signal as a
    // proxy for "I prefer a quiet machine."
    return false;
  }
  return true;
}

/**
 * Schedule one feedback sequence on an already-running context.
 * Pulled out so the suspended-context branch can call the same code
 * after the async resume() resolves.
 */
function schedule(audio: AudioContext, kind: FeedbackKind): void {
  const now = audio.currentTime;
  for (const part of SEQUENCES[kind]) {
    const osc = audio.createOscillator();
    const gain = audio.createGain();
    osc.type = "sine";
    osc.frequency.value = part.freq;
    const peak = part.gain ?? 0.18;
    // Tight attack + linear decay = bell-like timbre without ringing.
    gain.gain.setValueAtTime(0.0001, now + part.start);
    gain.gain.exponentialRampToValueAtTime(peak, now + part.start + 0.005);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + part.start + part.duration);
    osc.connect(gain).connect(audio.destination);
    osc.start(now + part.start);
    osc.stop(now + part.start + part.duration + 0.01);
  }
}

export function playFeedback(kind: FeedbackKind): void {
  if (!isEnabled()) return;
  const audio = context();
  if (!audio) return;
  // Safari/WebKit (and Tauri's wkwebview) auto-suspend the
  // AudioContext until a user gesture resumes it. The previous code
  // fired-and-forgot `audio.resume()` and immediately scheduled
  // oscillators against `audio.currentTime` — which was still
  // frozen, so the events landed in the past and produced silence.
  // Wait for resume to actually flip the state, THEN schedule.
  if (audio.state === "suspended") {
    audio
      .resume()
      .then(() => schedule(audio, kind))
      .catch((e) => {
        console.warn("AudioContext resume failed:", e);
      });
    return;
  }
  schedule(audio, kind);
}
