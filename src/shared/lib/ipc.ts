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
import {
  save as platformShowSaveDialog,
  type SaveDialogOptions,
} from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { openUrl as platformOpenUrl } from "@tauri-apps/plugin-opener";
import {
  getCurrent as platformGetInitialDeepLink,
  onOpenUrl as platformOnDeepLink,
} from "@tauri-apps/plugin-deep-link";

import type { Agent } from "@/shared/types/Agent";
import type { AgentRun } from "@/shared/types/AgentRun";
import type { NoteSearchHit } from "@/shared/types/NoteSearchHit";
import type { ChatThread } from "@/shared/types/ChatThread";
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
import type { DiarizationModelStatus } from "@/shared/types/DiarizationModelStatus";
import type { SpeakerLabel } from "@/shared/types/SpeakerLabel";
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

/** Mic level status returned by {@link checkMicLevel} (GET-212). */
export type MicStatus = "ok" | "too_quiet" | "clipping";

/** Brief mic input-level measurement (GET-212). */
export interface MicLevelResult {
  rms_db: number;
  peak_db: number;
  status: MicStatus;
  settings_url: string;
}

/**
 * Sample the default (or named) mic for ~500 ms and return the
 * RMS/peak level in dBFS plus a qualitative status (GET-212).
 */
export function checkMicLevel(deviceName?: string): Promise<MicLevelResult> {
  return call<MicLevelResult>("check_mic_level", { deviceName });
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

/** Create an empty note (GET-155) the user can write in before/without
 *  recording. Returns its summary. */
export function createNote(): Promise<RecordingSummary> {
  return call<RecordingSummary>("create_note");
}

/** Set or clear a note's user title (GET-163). Persists `title.txt` in
 *  the session dir; an empty title clears it so the UI falls back to the
 *  autoname suggestion or the label. */
export function renameNote(sessionDir: string, title: string): Promise<void> {
  return call<void>("rename_note", { sessionDir, title });
}

/** Read which enhanced-notes run the user has kept (its `finished_at`), or null. */
export function getEnhancedNotesAccepted(sessionDir: string): Promise<string | null> {
  return call<string | null>("get_enhanced_notes_accepted", { sessionDir });
}

/** Keep (own) the enhanced-notes summary identified by `marker` (its
 *  `finished_at`); pass "" to clear and revert it to muted/AI. */
export function setEnhancedNotesAccepted(
  sessionDir: string,
  marker: string
): Promise<void> {
  return call<void>("set_enhanced_notes_accepted", { sessionDir, marker });
}

/** Start capture. With `sessionDir` (GET-155) it records into that
 *  existing note's directory instead of a fresh one. */
export function startRecording(sessionDir?: string): Promise<RecordingStatus> {
  return call<RecordingStatus>("start_recording", { sessionDir });
}

export function stopRecording(): Promise<RecordingResult> {
  return call<RecordingResult>("stop_recording");
}

/** Pause capture, keeping the note open for a Resume (GET-149). */
export function pauseRecording(): Promise<RecordingStatus> {
  return call<RecordingStatus>("pause_recording");
}

/** Resume a paused note, continuing capture into the same note (GET-149). */
export function resumeRecording(): Promise<RecordingStatus> {
  return call<RecordingStatus>("resume_recording");
}

// ---- Library ------------------------------------------------------------

export function listRecordings(): Promise<RecordingSummary[]> {
  return call<RecordingSummary[]>("list_recordings");
}

export function getRecording(label: string): Promise<RecordingSummary | null> {
  return call<RecordingSummary | null>("get_recording", { label });
}

/** Full-text search across note content — title, summary, live notes,
 *  transcript (GET-165). Returns matching notes with a snippet. */
export function searchNoteContent(query: string): Promise<NoteSearchHit[]> {
  return call<NoteSearchHit[]>("search_note_content", { query });
}

export function revealInFinder(path: string): Promise<void> {
  return call<void>("reveal_in_finder", { path });
}

/** Export a note as a self-contained Markdown file (GET-166). Returns
 *  the written path — hand it to `sharePaths` or `revealInFinder`. */
export function exportNoteMarkdown(sessionDir: string): Promise<string> {
  return call<string>("export_note_markdown", { sessionDir });
}

export function deleteRecording(sessionDir: string): Promise<void> {
  return call<void>("delete_recording", { sessionDir });
}

// ---- Folders / Spaces (GET-162) -----------------------------------------

/** Every folder name (registry order, then in-use orphans). */
export function listFolders(): Promise<string[]> {
  return call<string[]>("list_folders");
}

/** Create a folder; idempotent on a case-insensitive name match.
 *  Returns the updated folder list. */
export function createFolder(name: string): Promise<string[]> {
  return call<string[]>("create_folder", { name });
}

/** Rename a folder, rewriting every member note. Returns the new list. */
export function renameFolder(from: string, to: string): Promise<string[]> {
  return call<string[]>("rename_folder", { from, to });
}

/** Delete a folder, clearing its notes' assignments. Returns the new list. */
export function deleteFolder(name: string): Promise<string[]> {
  return call<string[]>("delete_folder", { name });
}

/** Assign a note to a folder, or clear it with `folder = null`. */
export function setNoteFolder(
  sessionDir: string,
  folder: string | null
): Promise<void> {
  return call<void>("set_note_folder", { sessionDir, folder });
}

// ---- Transcription ------------------------------------------------------

export function transcribeRecording(sessionDir: string): Promise<TranscriptionResult> {
  return call<TranscriptionResult>("transcribe_recording", { sessionDir });
}

/**
 * Per-channel result of the VAD pre-pass over a session. Mirrors the
 * Rust-side `ChannelVadResult` so the recording-store can log
 * stripped-silence telemetry without an extra IPC roundtrip. The
 * sidecar JSON is opaque to JS — only the totals matter here.
 */
export interface VadChannelResult {
  channel: string;
  speech_wav_path: string;
  sidecar_path: string;
  sidecar: {
    sample_rate: number;
    original_samples: number;
    kept_samples: number;
    silence_stripped_seconds: number;
    active_ratio: number;
  };
}

export interface VadRunResult {
  session_dir: string;
  channels: VadChannelResult[];
  channel_errors: string[];
}

export function runVad(sessionDir: string): Promise<VadRunResult> {
  return call<VadRunResult>("run_vad", { sessionDir });
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

// ---- Diarization models -------------------------------------------------

/** On-disk status of every speaker-diarization model. */
export function diarizationModelStatus(): Promise<DiarizationModelStatus[]> {
  return call<DiarizationModelStatus[]>("diarization_model_status");
}

/** Download whichever diarization models are missing; returns final status. */
export function ensureDiarizationModels(): Promise<DiarizationModelStatus[]> {
  return call<DiarizationModelStatus[]>("ensure_diarization_models");
}

/** Live progress emitted while a diarization model download is in flight. */
export interface DiarizationDownloadProgress {
  model_id: string;
  downloaded: number;
  total: number | null;
}

/** Channel name for the Tauri event. Exported so listeners stay in sync. */
export const DIARIZATION_DOWNLOAD_PROGRESS_EVENT =
  "diarization:model-download-progress";

// ---- Session speakers (rename + cross-recording memory) ----------------

/** The diarized speakers of a recording (cluster id, name, provenance). */
export function listSessionSpeakers(sessionDir: string): Promise<SpeakerLabel[]> {
  return call<SpeakerLabel[]>("list_session_speakers", { sessionDir });
}

/**
 * Rename a diarized speaker. Persists to this recording and teaches the
 * cross-recording registry the voice → name link (when the cluster has a
 * voice embedding), so future recordings auto-detect the speaker. Returns
 * the updated label set.
 */
export function renameSessionSpeaker(
  sessionDir: string,
  cluster: number,
  name: string
): Promise<SpeakerLabel[]> {
  return call<SpeakerLabel[]>("rename_session_speaker", {
    sessionDir,
    cluster,
    name,
  });
}

/**
 * Confirm a medium-confidence speaker suggestion ("yes, this is <name>").
 * Adds this recording's voice as an exemplar of the suggested identity so
 * future recordings recognise it. Returns the updated label set.
 */
export function confirmSessionSpeaker(
  sessionDir: string,
  cluster: number
): Promise<SpeakerLabel[]> {
  return call<SpeakerLabel[]>("confirm_session_speaker", { sessionDir, cluster });
}

/**
 * Reject a medium-confidence speaker suggestion ("no, not <name>"). Records
 * a negative exemplar so that identity stops matching this voice. Returns
 * the updated label set.
 */
export function rejectSessionSpeaker(
  sessionDir: string,
  cluster: number
): Promise<SpeakerLabel[]> {
  return call<SpeakerLabel[]>("reject_session_speaker", { sessionDir, cluster });
}

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

// ---- Permission walkthrough (v2 #003 / GET-31) -------------------

import type { PermissionRow } from "@/shared/types/PermissionRow";
import type { Permission } from "@/shared/types/Permission";

export function listPermissions(): Promise<PermissionRow[]> {
  return call<PermissionRow[]>("list_permissions");
}

export function openPermissionSettings(permission: Permission): Promise<void> {
  return call<void>("open_permission_settings", { permission });
}

export function requestCalendarAccess(): Promise<void> {
  return call<void>("request_calendar_access");
}

// ---- Calendar-derived suggestions (GET-132) ----------------------

import type { AttendeeSuggestion } from "@/shared/types/AttendeeSuggestion";
import type { CalendarEvent } from "@/shared/types/CalendarEvent";

export function listAttendeeSuggestions(
  userEmail: string,
  domainFilter: string,
  windowDays: number,
  minCount: number
): Promise<AttendeeSuggestion[]> {
  return call<AttendeeSuggestion[]>("list_attendee_suggestions", {
    userEmail,
    domainFilter,
    windowDays,
    minCount,
  });
}

// ---- Coming-up calendar (GET-161) --------------------------------

/** Calendar authorization: "authorized" | "denied" | "restricted" |
 *  "not_determined". Drives the Home "Coming up" permission state. */
export function calendarAuthorizationStatus(): Promise<string> {
  return call<string>("calendar_authorization_status");
}

/** The next upcoming meeting from Apple Calendar, or null. */
export function nextCalendarEvent(): Promise<CalendarEvent | null> {
  return call<CalendarEvent | null>("next_calendar_event");
}

// ---- Backend auth ------------------------------------------------

import type { AuthStatus } from "@/shared/types/AuthStatus";
import type { UserIdentity } from "@/shared/types/UserIdentity";

export function authRequestSigninCode(email: string): Promise<void> {
  return call<void>("auth_request_signin_code", { email });
}

export function authVerifySigninCode(
  email: string,
  code: string,
  deviceId: string,
  deviceName: string
): Promise<UserIdentity> {
  return call<UserIdentity>("auth_verify_signin_code", {
    email,
    code,
    deviceId,
    deviceName,
  });
}

export function authStatus(): Promise<AuthStatus> {
  return call<AuthStatus>("auth_status");
}

export function authLogout(): Promise<void> {
  return call<void>("auth_logout");
}

// ---- Backend account ---------------------------------------------

import type { UserDoc } from "@/shared/types/UserDoc";
import type { DeviceDoc } from "@/shared/types/DeviceDoc";
import type { ReferralTokenResponse } from "@/shared/types/ReferralTokenResponse";
import type { ReferralStats } from "@/shared/types/ReferralStats";

export function accountGet(): Promise<UserDoc> {
  return call<UserDoc>("account_get");
}

export function accountUpdate(displayName: string | null): Promise<UserDoc> {
  return call<UserDoc>("account_update", { displayName });
}

export function accountDevices(): Promise<DeviceDoc[]> {
  return call<DeviceDoc[]>("account_devices");
}

export function accountRevokeDevice(deviceId: string): Promise<void> {
  return call<void>("account_revoke_device", { deviceId });
}

export function accountSoftDelete(): Promise<void> {
  return call<void>("account_soft_delete");
}

// ---- Backend referrals (GET-141) ---------------------------------

export function referralsGenerate(): Promise<ReferralTokenResponse> {
  return call<ReferralTokenResponse>("referrals_generate");
}

export function referralsMe(): Promise<ReferralStats> {
  return call<ReferralStats>("referrals_me");
}

export function referralsRedeem(
  token: string,
  newUserId: string,
  newUserEmail: string
): Promise<void> {
  return call<void>("referrals_redeem", { token, newUserId, newUserEmail });
}

// ---- Backend settings sync ---------------------------------------

export interface SettingsSyncSnapshot {
  settings: unknown | null;
  updated_at: string | null;
}

export function settingsSyncPull(): Promise<SettingsSyncSnapshot> {
  return call<SettingsSyncSnapshot>("settings_sync_pull");
}

export function settingsSyncPush(
  settings: unknown,
  updatedAt: string
): Promise<SettingsSyncSnapshot> {
  return call<SettingsSyncSnapshot>("settings_sync_push", {
    settings,
    updatedAt,
  });
}

// ---- Menu bar tray bridge (v2 #006 / GET-25) ---------------------

/**
 * Push the current recording state into the menu bar tray icon (GET-201).
 * - `elapsedSecs=null`            → idle
 * - `elapsedSecs=N, paused=false` → recording (red circle)
 * - `elapsedSecs=N, paused=true`  → paused (pause bars)
 * - `airgapped=true`              → Privacy Mode (lock)
 */
export function setTrayRecording(
  elapsedSecs: number | null,
  paused?: boolean,
  airgapped?: boolean
): Promise<void> {
  return call<void>("set_tray_recording", { elapsedSecs, paused, airgapped });
}

// ---- Native Preferences NSWindow (v2 #020 / GET-86) -------------

/** Open the dedicated Preferences NSWindow. Replaces the in-app modal. */
export function openPreferencesWindow(): Promise<void> {
  return call<void>("open_preferences_window");
}

// ---- Meeting-detection HUD (GET-143) -----------------------------

/** Tauri window label of the frameless meeting-detection HUD popover. */
export const MEETING_HUD_WINDOW_LABEL = "meeting-hud";
/** Event the watcher emits when a conferencing app is detected. */
export const MEETING_DETECTED_EVENT = "meeting-detected";
/** Event the HUD asks the main window to start the one-click flow with. */
export const MEETING_TAKE_NOTES_EVENT = "meeting:take-notes";

/** Payload of {@link MEETING_DETECTED_EVENT} + {@link getPendingMeeting}. */
export interface DetectedMeeting {
  bundle_id: string;
  app_label: string;
  detected_at_ms: number;
}

/** The detection awaiting a decision in the HUD, read on HUD mount. */
export function getPendingMeeting(): Promise<DetectedMeeting | null> {
  return call<DetectedMeeting | null>("get_pending_meeting");
}

/** Take Notes: focus the main window, start capture, close the HUD. */
export function meetingTakeNotes(): Promise<void> {
  return call<void>("meeting_take_notes");
}

/** Dismiss the HUD without muting the app. */
export function dismissMeetingHud(): Promise<void> {
  return call<void>("dismiss_meeting_hud");
}

/** Don't ask for this app again: mute its bundle id and close the HUD. */
export function suppressMeetingApp(bundleId: string): Promise<void> {
  return call<void>("suppress_meeting_app", { bundleId });
}

/** Subscribe to fresh meeting detections (HUD refresh while open). */
export async function onMeetingDetected(
  handler: (meeting: DetectedMeeting) => void
): Promise<UnlistenFn> {
  return listen<DetectedMeeting>(MEETING_DETECTED_EVENT, (event) =>
    handler(event.payload)
  );
}

/** One bullet in a pre-meeting brief (GET-197). */
export interface BriefBullet {
  text: string;
  source_label?: string | null;
}

/** A generated pre-meeting brief (GET-197). */
export interface MeetingBrief {
  bullets: BriefBullet[];
  sources_count: number;
  attendees_searched: string[];
}

/**
 * Generate a pre-meeting brief from local context (GET-197).
 * Pass the attendee list from the next calendar event.
 * Returns null when attendees is empty, no API key set, privacy mode on,
 * or no relevant local context exists.
 */
export function getMeetingBrief(attendees: string[]): Promise<MeetingBrief | null> {
  return call<MeetingBrief | null>("get_meeting_brief", { attendees });
}

/** Subscribe to the HUD's Take-Notes request (main window only). */
export async function onMeetingTakeNotes(handler: () => void): Promise<UnlistenFn> {
  return listen(MEETING_TAKE_NOTES_EVENT, () => handler());
}

// ---- Floating recording bar --------------------------------------

/** Tauri window label of the frameless floating recording-control bar. */
export const RECORDING_BAR_WINDOW_LABEL = "recording-bar";
/** Event the bar's Stop button asks the main window to stop with. */
export const RECORDING_BAR_STOP_EVENT = "recording-bar:stop";
/** Events the bar's pause/resume control asks the main window with. */
export const RECORDING_BAR_PAUSE_EVENT = "recording-bar:pause";
export const RECORDING_BAR_RESUME_EVENT = "recording-bar:resume";

/** Open (or reveal) the floating recording bar. Called on capture start. */
export function showRecordingBar(): Promise<void> {
  return call<void>("show_recording_bar");
}

/** Close the floating recording bar. Called on capture stop. */
export function hideRecordingBar(): Promise<void> {
  return call<void>("hide_recording_bar");
}

/** Stop from the bar — routes through the main window's stop flow. */
export function recordingBarStop(): Promise<void> {
  return call<void>("recording_bar_stop");
}

/** Pause from the bar — routes through the main window's pause flow. */
export function recordingBarPause(): Promise<void> {
  return call<void>("recording_bar_pause");
}

/** Resume from the bar — routes through the main window's resume flow. */
export function recordingBarResume(): Promise<void> {
  return call<void>("recording_bar_resume");
}

/** Subscribe to the bar's Stop request (main window only). */
export async function onRecordingBarStop(handler: () => void): Promise<UnlistenFn> {
  return listen(RECORDING_BAR_STOP_EVENT, () => handler());
}

/** Subscribe to the bar's Pause request (main window only). */
export async function onRecordingBarPause(handler: () => void): Promise<UnlistenFn> {
  return listen(RECORDING_BAR_PAUSE_EVENT, () => handler());
}

/** Subscribe to the bar's Resume request (main window only). */
export async function onRecordingBarResume(handler: () => void): Promise<UnlistenFn> {
  return listen(RECORDING_BAR_RESUME_EVENT, () => handler());
}

// ---- Live transcript preview (GET-160) ---------------------------

/** Rolling-window live transcript preview emitted while recording. */
export interface LiveTranscript {
  session_dir: string;
  text: string;
}

/** Subscribe to live-transcript previews. The handler fires with the
 *  latest rolling-window text for the capturing note. */
export async function onLiveTranscript(
  handler: (preview: LiveTranscript) => void
): Promise<UnlistenFn> {
  return listen<LiveTranscript>("live-transcript", (event) => handler(event.payload));
}

// ---- Menu bar tray events (GET-144) ------------------------------
// The tray menu (GET-25) emits these; GET-144 wires the main window to
// them so every entry point — tray, Cmd-R, and the meeting HUD —
// converges on one take-notes flow.

export type TrayEvent =
  | "tray:start-recording"
  | "tray:stop-recording"
  | "tray:open-library"
  | "tray:open-inbox";

/** Subscribe to a menu-bar tray event. */
export async function onTrayEvent(
  event: TrayEvent,
  handler: () => void
): Promise<UnlistenFn> {
  return listen(event, () => handler());
}

// ---- Live notes (GET-145) ----------------------------------------

import type { RawNoteLine } from "@/shared/types/RawNoteLine";

/** Persist the anchored live-notes buffer for a session (atomic). */
export function saveLiveNotes(sessionDir: string, lines: RawNoteLine[]): Promise<void> {
  return call<void>("save_live_notes", { sessionDir, lines });
}

/** Load a session's raw live-note lines (empty when none yet). */
export function loadLiveNotes(sessionDir: string): Promise<RawNoteLine[]> {
  return call<RawNoteLine[]>("load_live_notes", { sessionDir });
}

// ---- Per-note scoped chat (GET-150) ------------------------------

/** One prior turn in a per-note conversation. */
export interface ChatTurn {
  role: "user" | "assistant";
  content: string;
}

/** Ask a question scoped to a single note's transcript + notes + runs. */
export function askNote(
  sessionDir: string,
  question: string,
  history: ChatTurn[]
): Promise<{ answer: string }> {
  return call<{ answer: string }>("ask_note", { sessionDir, question, history });
}

/** A user-authored chat recipe loaded from .attune/recipes/*.toml (GET-194). */
export interface UserRecipe {
  label: string;
  prompt: string;
  icon?: string | null;
}

/** Load user-authored recipes from the vault (GET-194). */
export function listRecipes(): Promise<UserRecipe[]> {
  return call<UserRecipe[]>("list_recipes");
}

/** Coverage metadata for a cross-library Ask Attune answer (GET-193). */
export interface CoverageNote {
  notes_total: number;
  notes_read: number;
  capped: boolean;
  date_oldest: string | null;
  date_newest: string | null;
  memories: number;
  tasks: number;
}

/**
 * Ask a question scoped to a specific folder (GET-205).
 * Returns the same shape as `askLibrary`.
 */
export function askFolder(
  folderName: string,
  question: string,
  history: ChatTurn[],
  model?: string
): Promise<{ answer: string; coverage: CoverageNote }> {
  return call<{ answer: string; coverage: CoverageNote }>("ask_folder", {
    folderName,
    question,
    history,
    model,
  });
}

/** Ask a question across the whole library (GET-152). Optional model id. */
export function askLibrary(
  question: string,
  history: ChatTurn[],
  model?: string
): Promise<{ answer: string; coverage: CoverageNote }> {
  return call<{ answer: string; coverage: CoverageNote }>("ask_library", {
    question,
    history,
    model,
  });
}

// ---- Chat history + Recents (GET-167) ----------------------------------

/** Persisted chat threads, newest first. `scope` is "library" or "note";
 *  `sessionDir` narrows note threads to one note. */
export function listChatThreads(
  scope?: "library" | "note",
  sessionDir?: string
): Promise<ChatThread[]> {
  return call<ChatThread[]>("list_chat_threads", { scope, sessionDir });
}

/** Save (upsert) a chat thread. */
export function saveChatThread(thread: ChatThread): Promise<void> {
  return call<void>("save_chat_thread", { thread });
}

/** Delete a chat thread by id. */
export function deleteChatThread(id: string): Promise<void> {
  return call<void>("delete_chat_thread", { id });
}

/** Label of the webview window this code is running in. Falls back to
 *  "main" outside a Tauri context (e.g. unit tests). */
export function currentWindowLabel(): string {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
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
 * Fuzzily locate the transcript segment behind a paraphrased enhanced-note
 * line (GET-198). Returns null when the line can't be pinned to a moment.
 */
export function locateNoteEvidence(
  sessionDir: string,
  line: string
): Promise<TranscriptHit | null> {
  return call<TranscriptHit | null>("locate_note_evidence", { sessionDir, line });
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

export async function onDiarizationDownloadProgress<T = DiarizationDownloadProgress>(
  handler: (payload: T) => void
): Promise<UnlistenFn> {
  return listen<T>(DIARIZATION_DOWNLOAD_PROGRESS_EVENT, (event) =>
    handler(event.payload)
  );
}

export async function onDeepLink(
  handler: (urls: string[]) => void
): Promise<UnlistenFn> {
  return platformOnDeepLink(handler);
}

export async function getInitialDeepLink(): Promise<string[] | null> {
  return platformGetInitialDeepLink();
}

export async function openExternalUrl(url: string): Promise<void> {
  await platformOpenUrl(url);
}

export async function showSaveDialog(
  options: SaveDialogOptions
): Promise<string | null> {
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
export async function writeTextFileFromBrowser(
  path: string,
  contents: string
): Promise<void> {
  await writeTextFile(path, contents);
}

// ---- MCP config generator (GET-208) -------------------------------------

export interface McpClient {
  id: string;
  name: string;
  status: "detected" | "not_found";
  config_path: string | null;
  json_snippet: string;
  cli_command: string | null;
}

export interface McpConnectInfo {
  clients: McpClient[];
  binary_path: string | null;
}

/** Detect installed MCP clients and generate ready-to-use config snippets. */
export function generateMcpConfig(): Promise<McpConnectInfo> {
  return call<McpConnectInfo>("generate_mcp_config");
}

/** Write the Attune MCP block into a client's config file. */
export function writeMcpConfig(
  configPath: string,
  binaryPath: string,
  clientId: string
): Promise<string> {
  return call<string>("write_mcp_config", { configPath, binaryPath, clientId });
}

// ---- MCP consent + access ledger (GET-210) --------------------------------

export interface McpClientGrant {
  client_id: string;
  client_name?: string | null;
  allow_reads: boolean;
  granted_at?: string | null;
}

export interface McpAccessEntry {
  ts: string;
  client: string;
  tool: string;
  notes: string[];
  query?: string | null;
}

export function listMcpGrants(): Promise<McpClientGrant[]> {
  return call<McpClientGrant[]>("list_mcp_grants");
}

export function grantMcpClient(clientId: string, clientName?: string): Promise<void> {
  return call<void>("grant_mcp_client", { clientId, clientName });
}

export function revokeMcpClient(clientId: string): Promise<void> {
  return call<void>("revoke_mcp_client", { clientId });
}

export function listMcpAccessLog(): Promise<McpAccessEntry[]> {
  return call<McpAccessEntry[]>("list_mcp_access_log");
}
