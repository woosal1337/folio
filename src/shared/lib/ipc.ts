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
import type { SyncState } from "@/shared/types/SyncState";
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

export function ping(name?: string): Promise<string> {
  return call<string>("ping", { name });
}

export function listInputDevices(): Promise<DeviceInfo[]> {
  return call<DeviceInfo[]>("list_input_devices");
}

export type MicStatus = "ok" | "too_quiet" | "clipping";

export interface MicLevelResult {
  rms_db: number;
  peak_db: number;
  status: MicStatus;
  settings_url: string;
}

export function checkMicLevel(deviceName?: string): Promise<MicLevelResult> {
  return call<MicLevelResult>("check_mic_level", { deviceName });
}

export function startMicMonitor(deviceName?: string): Promise<void> {
  return call<void>("start_mic_monitor", { deviceName });
}

export function stopMicMonitor(): Promise<void> {
  return call<void>("stop_mic_monitor");
}

export function getSettings(): Promise<Settings> {
  return call<Settings>("get_settings");
}

export function saveSettings(settings: Settings): Promise<void> {
  return call<void>("save_settings", { settings });
}

export function recordingStatus(): Promise<RecordingStatus> {
  return call<RecordingStatus>("recording_status");
}

export function createNote(): Promise<RecordingSummary> {
  return call<RecordingSummary>("create_note");
}

export function renameNote(sessionDir: string, title: string): Promise<void> {
  return call<void>("rename_note", { sessionDir, title });
}

export function getEnhancedNotesAccepted(sessionDir: string): Promise<string | null> {
  return call<string | null>("get_enhanced_notes_accepted", { sessionDir });
}

export function setEnhancedNotesAccepted(
  sessionDir: string,
  marker: string
): Promise<void> {
  return call<void>("set_enhanced_notes_accepted", { sessionDir, marker });
}

export function startRecording(sessionDir?: string): Promise<RecordingStatus> {
  return call<RecordingStatus>("start_recording", { sessionDir });
}

export function stopRecording(): Promise<RecordingResult> {
  return call<RecordingResult>("stop_recording");
}

export function pauseRecording(): Promise<RecordingStatus> {
  return call<RecordingStatus>("pause_recording");
}

export function resumeRecording(): Promise<RecordingStatus> {
  return call<RecordingStatus>("resume_recording");
}

export function listRecordings(): Promise<RecordingSummary[]> {
  return call<RecordingSummary[]>("list_recordings");
}

export function getRecording(label: string): Promise<RecordingSummary | null> {
  return call<RecordingSummary | null>("get_recording", { label });
}

export function searchNoteContent(query: string): Promise<NoteSearchHit[]> {
  return call<NoteSearchHit[]>("search_note_content", { query });
}

export function revealInFinder(path: string): Promise<void> {
  return call<void>("reveal_in_finder", { path });
}

export function exportNoteMarkdown(sessionDir: string): Promise<string> {
  return call<string>("export_note_markdown", { sessionDir });
}

export function deleteRecording(sessionDir: string): Promise<void> {
  return call<void>("delete_recording", { sessionDir });
}

export function listFolders(): Promise<string[]> {
  return call<string[]>("list_folders");
}

export function createFolder(name: string): Promise<string[]> {
  return call<string[]>("create_folder", { name });
}

export function renameFolder(from: string, to: string): Promise<string[]> {
  return call<string[]>("rename_folder", { from, to });
}

export function deleteFolder(name: string): Promise<string[]> {
  return call<string[]>("delete_folder", { name });
}

export function setNoteFolder(
  sessionDir: string,
  folder: string | null
): Promise<void> {
  return call<void>("set_note_folder", { sessionDir, folder });
}

export function transcribeRecording(sessionDir: string): Promise<TranscriptionResult> {
  return call<TranscriptionResult>("transcribe_recording", { sessionDir });
}

export function diarizeSession(sessionDir: string): Promise<boolean> {
  return call<boolean>("diarize_session", { sessionDir });
}

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

export function whisperModelStatus(): Promise<WhisperModelStatus> {
  return call<WhisperModelStatus>("whisper_model_status");
}

export function ensureWhisperModel(modelId: WhisperModel): Promise<WhisperModelStatus> {
  return call<WhisperModelStatus>("ensure_whisper_model", { modelId });
}

export interface WhisperDownloadProgress {
  model_id: string;
  downloaded: number;
  total: number | null;
}

export const WHISPER_DOWNLOAD_PROGRESS_EVENT = "whisper:model-download-progress";

export function diarizationModelStatus(): Promise<DiarizationModelStatus[]> {
  return call<DiarizationModelStatus[]>("diarization_model_status");
}

export function ensureDiarizationModels(): Promise<DiarizationModelStatus[]> {
  return call<DiarizationModelStatus[]>("ensure_diarization_models");
}

export interface DiarizationDownloadProgress {
  model_id: string;
  downloaded: number;
  total: number | null;
}

export const DIARIZATION_DOWNLOAD_PROGRESS_EVENT =
  "diarization:model-download-progress";

export function listSessionSpeakers(sessionDir: string): Promise<SpeakerLabel[]> {
  return call<SpeakerLabel[]>("list_session_speakers", { sessionDir });
}

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

export function confirmSessionSpeaker(
  sessionDir: string,
  cluster: number
): Promise<SpeakerLabel[]> {
  return call<SpeakerLabel[]>("confirm_session_speaker", { sessionDir, cluster });
}

export function rejectSessionSpeaker(
  sessionDir: string,
  cluster: number
): Promise<SpeakerLabel[]> {
  return call<SpeakerLabel[]>("reject_session_speaker", { sessionDir, cluster });
}

export function listProviders(): Promise<ProviderStatus[]> {
  return call<ProviderStatus[]>("list_providers");
}

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

export function clearRecordingArtifacts(sessionDir: string): Promise<void> {
  return call<void>("clear_recording_artifacts", { sessionDir });
}

export function exportVaultSnapshot(destination: string): Promise<SnapshotSummary> {
  return call<SnapshotSummary>("export_vault_snapshot", { destination });
}

export function purgeOldWavFiles(olderThanDays: number | null): Promise<PurgeSummary> {
  return call<PurgeSummary>("purge_old_wav_files", {
    olderThanDays,
  });
}

export function generateWeeklyDigest(): Promise<DigestResult> {
  return call<DigestResult>("generate_weekly_digest");
}

export function exportShareBundle(
  sessionDir: string,
  destination: string
): Promise<ShareBundleSummary> {
  return call<ShareBundleSummary>("export_share_bundle", {
    sessionDir,
    destination,
  });
}

export function gitSyncVault(): Promise<GitSyncSummary> {
  return call<GitSyncSummary>("git_sync_vault");
}

export function gitVaultIsRepo(): Promise<boolean> {
  return call<boolean>("git_vault_is_repo");
}

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

export function getRecordingLanguage(sessionDir: string): Promise<string | null> {
  return call<string | null>("get_recording_language", { sessionDir });
}

export function setRecordingLanguage(
  sessionDir: string,
  language: string | null
): Promise<void> {
  return call<void>("set_recording_language", { sessionDir, language });
}

export function sharePaths(paths: string[]): Promise<void> {
  return call<void>("share_paths", { paths });
}

import type { PermissionRow } from "@/shared/types/PermissionRow";
import type { Permission } from "@/shared/types/Permission";

export function listPermissions(): Promise<PermissionRow[]> {
  return call<PermissionRow[]>("list_permissions");
}

export function openPermissionSettings(permission: Permission): Promise<void> {
  return call<void>("open_permission_settings", { permission });
}

export function requestPermission(permission: Permission): Promise<void> {
  return call<void>("request_permission", { permission });
}

export function requestCalendarAccess(): Promise<void> {
  return call<void>("request_calendar_access");
}

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

export function calendarAuthorizationStatus(): Promise<string> {
  return call<string>("calendar_authorization_status");
}

export function nextCalendarEvent(): Promise<CalendarEvent | null> {
  return call<CalendarEvent | null>("next_calendar_event");
}

export function listCalendarEvents(windowDays: number): Promise<CalendarEvent[]> {
  return call<CalendarEvent[]>("list_calendar_events", { windowDays });
}

export function setTrayRecording(
  elapsedSecs: number | null,
  paused?: boolean,
  airgapped?: boolean
): Promise<void> {
  return call<void>("set_tray_recording", { elapsedSecs, paused, airgapped });
}

export function openPreferencesWindow(): Promise<void> {
  return call<void>("open_preferences_window");
}

export const MEETING_HUD_WINDOW_LABEL = "meeting-hud";

export const MEETING_DETECTED_EVENT = "meeting-detected";

export const MEETING_TAKE_NOTES_EVENT = "meeting:take-notes";

export interface DetectedMeeting {
  bundle_id: string;
  app_label: string;
  detected_at_ms: number;
}

export function getPendingMeeting(): Promise<DetectedMeeting | null> {
  return call<DetectedMeeting | null>("get_pending_meeting");
}

export function meetingTakeNotes(): Promise<void> {
  return call<void>("meeting_take_notes");
}

export function dismissMeetingHud(): Promise<void> {
  return call<void>("dismiss_meeting_hud");
}

export function suppressMeetingApp(bundleId: string): Promise<void> {
  return call<void>("suppress_meeting_app", { bundleId });
}

export async function onMeetingDetected(
  handler: (meeting: DetectedMeeting) => void
): Promise<UnlistenFn> {
  return listen<DetectedMeeting>(MEETING_DETECTED_EVENT, (event) =>
    handler(event.payload)
  );
}

export interface BriefBullet {
  text: string;
  source_label?: string | null;
}

export interface MeetingBrief {
  bullets: BriefBullet[];
  sources_count: number;
  attendees_searched: string[];
}

export function getMeetingBrief(attendees: string[]): Promise<MeetingBrief | null> {
  return call<MeetingBrief | null>("get_meeting_brief", { attendees });
}

export async function onMeetingTakeNotes(handler: () => void): Promise<UnlistenFn> {
  return listen(MEETING_TAKE_NOTES_EVENT, () => handler());
}

export const RECORDING_BAR_WINDOW_LABEL = "recording-bar";

export const RECORDING_BAR_STOP_EVENT = "recording-bar:stop";

export const RECORDING_BAR_PAUSE_EVENT = "recording-bar:pause";
export const RECORDING_BAR_RESUME_EVENT = "recording-bar:resume";

export function showRecordingBar(): Promise<void> {
  return call<void>("show_recording_bar");
}

export function hideRecordingBar(): Promise<void> {
  return call<void>("hide_recording_bar");
}

export function recordingBarStop(): Promise<void> {
  return call<void>("recording_bar_stop");
}

export function recordingBarPause(): Promise<void> {
  return call<void>("recording_bar_pause");
}

export function recordingBarResume(): Promise<void> {
  return call<void>("recording_bar_resume");
}

export async function onRecordingBarStop(handler: () => void): Promise<UnlistenFn> {
  return listen(RECORDING_BAR_STOP_EVENT, () => handler());
}

export async function onRecordingBarPause(handler: () => void): Promise<UnlistenFn> {
  return listen(RECORDING_BAR_PAUSE_EVENT, () => handler());
}

export async function onRecordingBarResume(handler: () => void): Promise<UnlistenFn> {
  return listen(RECORDING_BAR_RESUME_EVENT, () => handler());
}

export interface LiveTranscript {
  session_dir: string;
  text: string;
}

export async function onLiveTranscript(
  handler: (preview: LiveTranscript) => void
): Promise<UnlistenFn> {
  return listen<LiveTranscript>("live-transcript", (event) => handler(event.payload));
}

export interface RemoteSyncProgress {
  session_dir: string;
  remote_status: string;
  transcript_written: boolean;
}

export async function onRemoteSyncProgress(
  handler: (progress: RemoteSyncProgress) => void
): Promise<UnlistenFn> {
  return listen<RemoteSyncProgress>("remote-sync-progress", (event) =>
    handler(event.payload)
  );
}

export type TrayEvent =
  | "tray:start-recording"
  | "tray:stop-recording"
  | "tray:open-library"
  | "tray:open-inbox";

export async function onTrayEvent(
  event: TrayEvent,
  handler: () => void
): Promise<UnlistenFn> {
  return listen(event, () => handler());
}

export async function onStitchingStarted(handler: () => void): Promise<UnlistenFn> {
  return listen("recording:stitching-started", () => handler());
}

export async function onStitchingDone(handler: () => void): Promise<UnlistenFn> {
  return listen("recording:stitching-done", () => handler());
}

import type { RawNoteLine } from "@/shared/types/RawNoteLine";

export function saveLiveNotes(sessionDir: string, lines: RawNoteLine[]): Promise<void> {
  return call<void>("save_live_notes", { sessionDir, lines });
}

export function loadLiveNotes(sessionDir: string): Promise<RawNoteLine[]> {
  return call<RawNoteLine[]>("load_live_notes", { sessionDir });
}

export interface ChatTurn {
  role: "user" | "assistant";
  content: string;
}

export function askNote(
  sessionDir: string,
  question: string,
  history: ChatTurn[]
): Promise<{ answer: string }> {
  return call<{ answer: string }>("ask_note", { sessionDir, question, history });
}

export interface UserRecipe {
  label: string;
  prompt: string;
  icon?: string | null;
}

export function listRecipes(): Promise<UserRecipe[]> {
  return call<UserRecipe[]>("list_recipes");
}

export interface CoverageNote {
  notes_total: number;
  notes_read: number;
  capped: boolean;
  date_oldest: string | null;
  date_newest: string | null;
  memories: number;
  tasks: number;
}

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

export function listChatThreads(
  scope?: "library" | "note",
  sessionDir?: string
): Promise<ChatThread[]> {
  return call<ChatThread[]>("list_chat_threads", { scope, sessionDir });
}

export function saveChatThread(thread: ChatThread): Promise<void> {
  return call<void>("save_chat_thread", { thread });
}

export function deleteChatThread(id: string): Promise<void> {
  return call<void>("delete_chat_thread", { id });
}

export function currentWindowLabel(): string {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

import type { TranscriptHit } from "@/shared/types/TranscriptHit";

export function locateTranscriptSpan(
  sessionDir: string,
  span: string
): Promise<TranscriptHit | null> {
  return call<TranscriptHit | null>("locate_transcript_span", { sessionDir, span });
}

export function locateNoteEvidence(
  sessionDir: string,
  line: string
): Promise<TranscriptHit | null> {
  return call<TranscriptHit | null>("locate_note_evidence", { sessionDir, line });
}

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

export async function writeTextFileFromBrowser(
  path: string,
  contents: string
): Promise<void> {
  await writeTextFile(path, contents);
}

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

export function generateMcpConfig(): Promise<McpConnectInfo> {
  return call<McpConnectInfo>("generate_mcp_config");
}

export function writeMcpConfig(
  configPath: string,
  binaryPath: string,
  clientId: string
): Promise<string> {
  return call<string>("write_mcp_config", { configPath, binaryPath, clientId });
}

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

export interface RemoteAccount {
  signed_in: boolean;
  email: string | null;
}

export interface EndpointTest {
  ok: boolean;
  engine: string | null;
  model: string | null;
  gpu: boolean | null;
  message: string;
}

export function remoteRegister(email: string, password: string): Promise<void> {
  return call<void>("remote_register", { email, password });
}

export function remoteLogin(email: string, password: string): Promise<void> {
  return call<void>("remote_login", { email, password });
}

export function remoteLogout(): Promise<void> {
  return call<void>("remote_logout");
}

export function remoteMe(): Promise<RemoteAccount> {
  return call<RemoteAccount>("remote_me");
}

export function testRemoteEndpoint(endpoint: string): Promise<EndpointTest> {
  return call<EndpointTest>("test_remote_endpoint", { endpoint });
}

export function syncRecording(sessionDir: string): Promise<SyncState> {
  return call<SyncState>("sync_recording", { sessionDir });
}

export function getSyncStatus(sessionDir: string): Promise<SyncState | null> {
  return call<SyncState | null>("get_sync_status", { sessionDir });
}
