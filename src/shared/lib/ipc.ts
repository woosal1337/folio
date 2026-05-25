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

import type { Agent } from "@/shared/types/Agent";
import type { AgentRun } from "@/shared/types/AgentRun";
import type { DeviceInfo } from "@/shared/types/DeviceInfo";
import type { ModelInfo } from "@/shared/types/ModelInfo";
import type { ProviderId } from "@/shared/types/ProviderId";
import type { ProviderStatus } from "@/shared/types/ProviderStatus";
import type { RecordingResult } from "@/shared/types/RecordingResult";
import type { RecordingStatus } from "@/shared/types/RecordingStatus";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";
import type { Memory } from "@/shared/types/Memory";
import type { MemoryKind } from "@/shared/types/MemoryKind";
import type { MemoryQuery } from "@/shared/types/MemoryQuery";
import type { MemoryUpdate } from "@/shared/types/MemoryUpdate";
import type { NewMemory } from "@/shared/types/NewMemory";
import type { NewTask } from "@/shared/types/NewTask";
import type { Settings } from "@/shared/types/Settings";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";
import type { DigestResult } from "@/shared/types/DigestResult";
import type { PurgeSummary } from "@/shared/types/PurgeSummary";
import type { ShareBundleSummary } from "@/shared/types/ShareBundleSummary";
import type { SnapshotSummary } from "@/shared/types/SnapshotSummary";
import type { Task } from "@/shared/types/Task";
import type { WebhookSubscription } from "@/shared/types/WebhookSubscription";
import type { TaskStatus } from "@/shared/types/TaskStatus";
import type { TaskUpdate } from "@/shared/types/TaskUpdate";
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

// ---- LLM providers -----------------------------------------------------

export function listProviders(): Promise<ProviderStatus[]> {
  return call<ProviderStatus[]>("list_providers");
}

export function setProviderKey(provider: ProviderId, apiKey: string): Promise<void> {
  return call<void>("set_provider_key", { provider, apiKey });
}

export function deleteProviderKey(provider: ProviderId): Promise<void> {
  return call<void>("delete_provider_key", { provider });
}

export function testProvider(provider: ProviderId): Promise<void> {
  return call<void>("test_provider", { provider });
}

export function listProviderModels(provider: ProviderId): Promise<ModelInfo[]> {
  return call<ModelInfo[]>("list_provider_models", { provider });
}

// ---- Agents ------------------------------------------------------------

export function listAgents(): Promise<Agent[]> {
  return call<Agent[]>("list_agents");
}

export function runAgent(sessionDir: string, agentId: string): Promise<AgentRun> {
  return call<AgentRun>("run_agent", { sessionDir, agentId });
}

export function listAgentRuns(sessionDir: string): Promise<AgentRun[]> {
  return call<AgentRun[]>("list_agent_runs", { sessionDir });
}

export function deleteAgentRun(sessionDir: string, agentId: string): Promise<void> {
  return call<void>("delete_agent_run", { sessionDir, agentId });
}

// ---- Tasks -------------------------------------------------------------

export function listTasks(): Promise<Task[]> {
  return call<Task[]>("list_tasks");
}

export function createTask(task: NewTask): Promise<Task> {
  return call<Task>("create_task", { task });
}

export function updateTask(id: string, patch: TaskUpdate): Promise<Task> {
  return call<Task>("update_task", { id, patch });
}

export function deleteTask(id: string): Promise<void> {
  return call<void>("delete_task", { id });
}

export function setTaskStatus(id: string, status: TaskStatus): Promise<Task> {
  return call<Task>("set_task_status", { id, status });
}

// ---- Memory ------------------------------------------------------------

export function listMemories(query: MemoryQuery): Promise<Memory[]> {
  return call<Memory[]>("list_memories", { query });
}

export function getMemory(id: string): Promise<Memory | null> {
  return call<Memory | null>("get_memory", { id });
}

export function createMemory(memory: NewMemory): Promise<Memory> {
  return call<Memory>("create_memory", { memory });
}

export function updateMemory(id: string, patch: MemoryUpdate): Promise<Memory> {
  return call<Memory>("update_memory", { id, patch });
}

export function deleteMemory(id: string): Promise<Memory> {
  return call<Memory>("delete_memory", { id });
}

export function purgeMemory(id: string): Promise<void> {
  return call<void>("purge_memory", { id });
}

export function pinMemory(id: string, pinned: boolean): Promise<Memory> {
  return call<Memory>("pin_memory", { id, pinned });
}

export function searchMemories(
  query: string,
  kinds: MemoryKind[],
  limit?: number
): Promise<Memory[]> {
  return call<Memory[]>("search_memories", { query, kinds, limit });
}

export function rebuildMemoryIndex(): Promise<number> {
  return call<number>("rebuild_memory_index");
}

export function memoryFilePath(id: string): Promise<string | null> {
  return call<string | null>("memory_file_path", { id });
}

// ---- Maintenance -------------------------------------------------------

export function clearRecordingArtifacts(sessionDir: string): Promise<void> {
  return call<void>("clear_recording_artifacts", { sessionDir });
}

/**
 * Build a vault-snapshot zip at the chosen destination. Returns a
 * summary describing how many files were bundled and the resulting
 * zip size. v2 finding 057 / GET-92.
 */
export function exportVaultSnapshot(destination: string): Promise<SnapshotSummary> {
  return call<SnapshotSummary>("export_vault_snapshot", { destination });
}

/**
 * Walk every session and delete mic.wav + system.wav from sessions
 * where the transcript exists AND the audio is older than the given
 * threshold. Reads the threshold from settings when `olderThanDays`
 * is null. v2 finding 063 / GET-98.
 */
export function purgeOldWavFiles(olderThanDays: number | null): Promise<PurgeSummary> {
  return call<PurgeSummary>("purge_old_wav_files", {
    olderThanDays,
  });
}

/** Generate a weekly digest markdown file under
 *  `~/Documents/Attune/Digests/YYYY-MM-DD.md`. v2 finding 082 / GET-80. */
export function generateWeeklyDigest(): Promise<DigestResult> {
  return call<DigestResult>("generate_weekly_digest");
}

/** Export a single recording as a sealed .attune-share zip with a
 *  SHA-256 manifest. v2 finding 052 / GET-69. */
export function exportShareBundle(
  sessionDir: string,
  destination: string
): Promise<ShareBundleSummary> {
  return call<ShareBundleSummary>("export_share_bundle", {
    sessionDir,
    destination,
  });
}

// ---- Captions window (v2 #103 / GET-115) -------------------------------

export function openCaptionsWindow(): Promise<void> {
  return call<void>("open_captions_window");
}

export function closeCaptionsWindow(): Promise<void> {
  return call<void>("close_captions_window");
}

// ---- Webhooks ----------------------------------------------------------

export function listWebhooks(): Promise<WebhookSubscription[]> {
  return call<WebhookSubscription[]>("list_webhooks");
}

export function saveWebhook(
  subscription: WebhookSubscription
): Promise<WebhookSubscription> {
  return call<WebhookSubscription>("save_webhook", { subscription });
}

export function deleteWebhook(id: string): Promise<void> {
  return call<void>("delete_webhook", { id });
}

export function testWebhook(id: string): Promise<string> {
  return call<string>("test_webhook", { id });
}

// ---- Per-recording language override (v2 #046 / GET-89) ----------

export function getRecordingLanguage(sessionDir: string): Promise<string | null> {
  return call<string | null>("get_recording_language", { sessionDir });
}

export function setRecordingLanguage(
  sessionDir: string,
  language: string | null
): Promise<void> {
  return call<void>("set_recording_language", { sessionDir, language });
}
