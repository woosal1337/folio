import { useSettingsStore } from "@/shared/stores/settings-store";

export type FeedbackKind = "start" | "stop" | "success" | "dismiss" | "error";

interface TonePart {
  freq: number;

  start: number;

  duration: number;

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

function isEnabled(): boolean {
  const s = useSettingsStore.getState().settings;
  if (!s) return false;
  if (!s.feedback_sounds_enabled) return false;
  if (
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches
  ) {
    return false;
  }
  return true;
}

function schedule(audio: AudioContext, kind: FeedbackKind): void {
  const now = audio.currentTime;
  for (const part of SEQUENCES[kind]) {
    const osc = audio.createOscillator();
    const gain = audio.createGain();
    osc.type = "sine";
    osc.frequency.value = part.freq;
    const peak = part.gain ?? 0.18;

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

  if (audio.state === "suspended") {
    void (async () => {
      try {
        await audio.resume();
        schedule(audio, kind);
      } catch (e) {
        console.error("AudioContext resume failed:", e);
      }
    })();
    return;
  }
  schedule(audio, kind);
}
