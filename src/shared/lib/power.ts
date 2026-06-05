export interface PowerSnapshot {
  charging: boolean;

  level: number | null;
}

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
    return { charging: true, level: null };
  }
  try {
    const battery = await nav.getBattery();
    return { charging: battery.charging, level: battery.level };
  } catch {
    return { charging: true, level: null };
  }
}

export async function shouldDeferOnPower(): Promise<boolean> {
  const p = await readPower();
  if (p.charging) return false;
  if (p.level === null) return false;
  return p.level < LOW_BATTERY_THRESHOLD;
}

export function formatBatteryPct(level: number | null): string {
  if (level === null) return "";
  return `${Math.round(level * 100)}%`;
}
