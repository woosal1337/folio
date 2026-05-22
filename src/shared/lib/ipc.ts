/**
 * Typed wrappers around Tauri's `invoke`.
 *
 * Every Rust `#[tauri::command]` gets a named async export here so call
 * sites never sprinkle stringly-typed `invoke` calls.
 *
 * All command argument and return types come from `@/shared/types/`,
 * which is generated from the Rust definitions in `attune-core` via
 * `ts-rs`. Re-running `cargo test` regenerates the bindings; CI catches
 * any drift.
 *
 * Errors thrown by the underlying `invoke` are wrapped in [`IpcError`]
 * so callers can distinguish IPC failures (timeouts, missing commands,
 * panics on the Rust side) from domain errors that bubble up as strings
 * inside the result type.
 */

import { invoke, type InvokeArgs } from "@tauri-apps/api/core";

import type { DeviceInfo } from "@/shared/types/DeviceInfo";
import type { RecordingResult } from "@/shared/types/RecordingResult";
import type { RecordingStatus } from "@/shared/types/RecordingStatus";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";
import type { Settings } from "@/shared/types/Settings";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";
import type { TranscriptionResult } from "@/shared/types/TranscriptionResult";
import type { WhisperModel } from "@/shared/types/WhisperModel";
import type { WhisperModelStatus } from "@/shared/types/WhisperModelStatus";

export class IpcError extends Error {
  constructor(
    public readonly command: string,
    public readonly cause: unknown
  ) {
    const detail =
      typeof cause === "string"
        ? cause
        : cause instanceof Error
          ? cause.message
          : JSON.stringify(cause);
    super(`ipc ${command} failed: ${detail}`);
    this.name = "IpcError";
  }
}

async function call<T>(command: string, args?: InvokeArgs): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (cause) {
    throw new IpcError(command, cause);
  }
}

// ---- Health -------------------------------------------------------------

export function ping(name?: string): Promise<string> {
  return call<string>("ping", { name });
}

// ---- Devices ------------------------------------------------------------

export function listInputDevices(): Promise<DeviceInfo[]> {
  return call<DeviceInfo[]>("list_input_devices");
}

// ---- Settings -----------------------------------------------------------

export function getSettings(): Promise<Settings> {
  return call<Settings>("get_settings");
}

export function saveSettings(settings: Settings): Promise<void> {
  return call<void>("save_settings", { settings });
}

// ---- Recording ----------------------------------------------------------

export function recordingStatus(): Promise<RecordingStatus> {
  return call<RecordingStatus>("recording_status");
}

export function startRecording(): Promise<RecordingStatus> {
  return call<RecordingStatus>("start_recording");
}

export function stopRecording(): Promise<RecordingResult> {
  return call<RecordingResult>("stop_recording");
}

// ---- Library ------------------------------------------------------------

export function listRecordings(): Promise<RecordingSummary[]> {
  return call<RecordingSummary[]>("list_recordings");
}

export function getRecording(label: string): Promise<RecordingSummary | null> {
  return call<RecordingSummary | null>("get_recording", { label });
}

export function revealInFinder(path: string): Promise<void> {
  return call<void>("reveal_in_finder", { path });
}

export function deleteRecording(sessionDir: string): Promise<void> {
  return call<void>("delete_recording", { sessionDir });
}

// ---- Transcription ------------------------------------------------------

export function transcribeRecording(sessionDir: string): Promise<TranscriptionResult> {
  return call<TranscriptionResult>("transcribe_recording", { sessionDir });
}

export function readTranscript(sessionDir: string): Promise<SessionTranscript> {
  return call<SessionTranscript>("read_transcript", { sessionDir });
}

export function saveTranscript(
  sessionDir: string,
  transcript: SessionTranscript
): Promise<string> {
  return call<string>("save_transcript", { sessionDir, transcript });
}

// ---- Local Whisper models -----------------------------------------------

export function whisperModelStatus(): Promise<WhisperModelStatus> {
  return call<WhisperModelStatus>("whisper_model_status");
}

export function ensureWhisperModel(modelId: WhisperModel): Promise<WhisperModelStatus> {
  return call<WhisperModelStatus>("ensure_whisper_model", { modelId });
}

/** Live progress emitted while a model download is in flight. */
export interface WhisperDownloadProgress {
  model_id: string;
  downloaded: number;
  total: number | null;
}

/** Channel name for the Tauri event. Exported so listeners stay in sync. */
export const WHISPER_DOWNLOAD_PROGRESS_EVENT = "whisper:model-download-progress";
