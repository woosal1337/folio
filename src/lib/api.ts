/* Typed wrappers around Tauri's `invoke`. Every Rust command exposed via
 * #[tauri::command] gets a named export here so callers see proper types
 * and screens never sprinkle stringly-typed invoke calls. */

import { invoke } from "@tauri-apps/api/core";
import type { DeviceInfo, Settings } from "./types";

export async function ping(name?: string): Promise<string> {
  return invoke<string>("ping", { name });
}

export async function listInputDevices(): Promise<DeviceInfo[]> {
  return invoke<DeviceInfo[]>("list_input_devices");
}

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke<void>("save_settings", { settings });
}
