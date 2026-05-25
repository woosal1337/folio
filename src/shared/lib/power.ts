/**
 * Power-state helpers. Used by the recording-store to defer
 * non-essential AI work when the laptop is on battery + low. v2
 * roadmap finding 065.
 *
 * Tauri exposes the Web Battery API on macOS via the WebView. We
 * gracefully degrade — if the browser blocks the API (Safari WebKit
 * often does), the helpers report "ok to run" and the user is no
 * worse off than today.
 */

export interface PowerSnapshot {
  /** True when the device is plugged in OR battery API unavailable
   *  (the latter is the conservative default — don't skip work just
   *  because we couldn't read the meter). */
  charging: boolean;
  /** 0-1 battery level when reported, null otherwise. */
  level: number | null;
}

/**
 * Default threshold under which we treat the machine as
 * power-constrained. ~30% leaves the user enough runway for a Local
 * Whisper transcription on a typical M1 Air.
 */
export const LOW_BATTERY_THRESHOLD = 0.3;

interface BatteryManager {
  charging: boolean;
  level: number;
}

interface NavigatorWithBattery extends Navigator {
  getBattery?: () => Promise<BatteryManager>;
}

export async function readPower(): Promise<PowerSnapshot> {
  const nav = navigator as NavigatorWithBattery;
  if (typeof nav.getBattery !== "function") {
    // API not exposed — pretend we're plugged in so we don't skip
    // work for the wrong reason.
    return { charging: true, level: null };
  }
  try {
    const battery = await nav.getBattery();
    return { charging: battery.charging, level: battery.level };
  } catch {
    return { charging: true, level: null };
  }
}

/**
 * Should we skip a battery-hungry AI job right now? Returns true
 * only when we are CERTAIN the device is on battery AND below the
 * threshold. Default-off semantics: when in doubt, run the work.
 */
export async function shouldDeferOnPower(): Promise<boolean> {
  const p = await readPower();
  if (p.charging) return false;
  if (p.level === null) return false;
  return p.level < LOW_BATTERY_THRESHOLD;
}

/** Format a battery level as "23%". */
export function formatBatteryPct(level: number | null): string {
  if (level === null) return "";
  return `${Math.round(level * 100)}%`;
}
