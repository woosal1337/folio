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

import { convertFileSrc, invoke, type InvokeArgs } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { save as platformShowSaveDialog, type SaveDialogOptions } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { openUrl as platformOpenUrl } from "@tauri-apps/plugin-opener";
import {
  getCurrent as platformGetInitialDeepLink,
  onOpenUrl as platformOnDeepLink,
} from "@tauri-apps/plugin-deep-link";

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
import type { GitSyncSummary } from "@/shared/types/GitSyncSummary";
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

/**
 * Probe the Keychain for an OpenAI API key. Used by the recording
 * store to gate auto-summarise / auto-extract-tasks / auto-extract-
 * memories / autoname on the presence of a key without exposing the
 * key value to React state. Phase-3 audit B9 phase 2.
 */
export async function hasOpenAiKey(): Promise<boolean> {
  try {
    const providers = await listProviders();
    return providers.some((p) => p.id === "openai" && p.configured);
  } catch (e) {
    console.error("hasOpenAiKey:", e);
    return false;
  }
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

// ---- Git sync (v2 #070 / GET-72) ---------------------------------------

export function gitSyncVault(): Promise<GitSyncSummary> {
  return call<GitSyncSummary>("git_sync_vault");
}

export function gitVaultIsRepo(): Promise<boolean> {
  return call<boolean>("git_vault_is_repo");
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

// ---- Native share sheet (v2 #010 / GET-34) -----------------------

/**
 * Present the macOS NSSharingServicePicker for one or more files.
 * Anchors to the current key window. AirDrop, Messages, Mail, Notes,
 * Reminders, third-party share extensions all come for free.
 */
export function sharePaths(paths: string[]): Promise<void> {
  return call<void>("share_paths", { paths });
}

// ---- Voice debrief (v2 #027 / GET-53) ----------------------------

/**
 * Save a voice-debrief blob next to an existing recording. `bytes`
 * is the raw container body (typically `audio/webm;codecs=opus`).
 * Returns the final on-disk path.
 */
export function saveDebrief(
  sessionDir: string,
  filename: string,
  bytes: Uint8Array
): Promise<string> {
  return call<string>("save_debrief", {
    sessionDir,
    filename,
    bytes: Array.from(bytes),
  });
}

// ---- Permission walkthrough (v2 #003 / GET-31) -------------------

import type { PermissionRow } from "@/shared/types/PermissionRow";
import type { Permission } from "@/shared/types/Permission";

export function listPermissions(): Promise<PermissionRow[]> {
  return call<PermissionRow[]>("list_permissions");
}

export function openPermissionSettings(permission: Permission): Promise<void> {
  return call<void>("open_permission_settings", { permission });
}

// ---- Menu bar tray bridge (v2 #006 / GET-25) ---------------------

/**
 * Push the current recording state into the menu bar tray icon. Pass
 * `null` when not recording; pass the elapsed seconds while recording
 * so the title updates to "● M:SS".
 */
export function setTrayRecording(elapsedSecs: number | null): Promise<void> {
  return call<void>("set_tray_recording", { elapsedSecs });
}

// ---- Native Preferences NSWindow (v2 #020 / GET-86) -------------

/** Open the dedicated Preferences NSWindow. Replaces the in-app modal. */
export function openPreferencesWindow(): Promise<void> {
  return call<void>("open_preferences_window");
}

// ---- Multi-window (v2 #014 / GET-48) -----------------------------

export function openRecordWindow(): Promise<void> {
  return call<void>("open_record_window");
}

export function openLibraryWindow(): Promise<void> {
  return call<void>("open_library_window");
}

export function openEditorWindow(label: string): Promise<void> {
  return call<void>("open_editor_window", { label });
}

// ---- Transcript backlinks (v2 #038 / GET-41) ---------------------

import type { TranscriptHit } from "@/shared/types/TranscriptHit";

/**
 * Locate an evidence span inside a session's transcript and return
 * the channel / segment / start-second / end-second it lives in.
 * Returns null when the span can't be found.
 */
export function locateTranscriptSpan(
  sessionDir: string,
  span: string
): Promise<TranscriptHit | null> {
  return call<TranscriptHit | null>("locate_transcript_span", { sessionDir, span });
}

/**
 * Centralised wrappers around every `@tauri-apps/*` direct import.
 * `docs/CODE_STYLE.md` §9.4 requires that **only this file** imports
 * `@tauri-apps/api/*` or `@tauri-apps/plugin-*`. Components consume
 * these typed wrappers; an ESLint `no-restricted-imports` rule
 * enforces the boundary.
 */

export const PRIVACY_MODE_CHANGED_EVENT = "privacy-mode-changed";

export function assetUrl(path: string): string {
  return convertFileSrc(path);
}

export async function startWindowDrag(): Promise<void> {
  await getCurrentWindow().startDragging();
}

export async function isWindowMaximized(): Promise<boolean> {
  return getCurrentWindow().isMaximized();
}

export async function toggleWindowMaximize(): Promise<void> {
  const win = getCurrentWindow();
  const maximized = await win.isMaximized();
  if (maximized) await win.unmaximize();
  else await win.maximize();
}

export async function onPrivacyModeChanged(
  handler: (enabled: boolean) => void
): Promise<UnlistenFn> {
  return listen<boolean>(PRIVACY_MODE_CHANGED_EVENT, (event) => handler(event.payload));
}

export async function onWhisperDownloadProgress<T = WhisperDownloadProgress>(
  handler: (payload: T) => void
): Promise<UnlistenFn> {
  return listen<T>(WHISPER_DOWNLOAD_PROGRESS_EVENT, (event) => handler(event.payload));
}

export async function onDeepLink(handler: (urls: string[]) => void): Promise<UnlistenFn> {
  return platformOnDeepLink(handler);
}

export async function getInitialDeepLink(): Promise<string[] | null> {
  return platformGetInitialDeepLink();
}

export async function openExternalUrl(url: string): Promise<void> {
  await platformOpenUrl(url);
}

export async function showSaveDialog(options: SaveDialogOptions): Promise<string | null> {
  return platformShowSaveDialog(options);
}

/**
 * Write a UTF-8 text file to `path`. NOTE: §9.1 says the React layer
 * "MUST NOT perform direct filesystem access via the browser" — this
 * wrapper exists so the single remaining call site (transcript export
 * after a user-picked save dialog) stays on the typed surface, and so
 * the ESLint boundary rule can flag any new direct fs callers. Folding
 * this through a Tauri command is tracked under
 * `docs/refactor/PHASE-3-PUNCH-LIST.md` (C8 follow-up).
 */
export async function writeTextFileFromBrowser(path: string, contents: string): Promise<void> {
  await writeTextFile(path, contents);
}
