import type { Page } from "@playwright/test";

import { installTauriStub, ipcLog } from "./tauri-ipc";

export interface MockSettings {
  mic_device: string | null;
  system_audio_enabled: boolean;
  output_dir: string;
  tasks_path: string;
  theme: string;
  transcriber: string;
  transcription_language: string;
  briefing_language: string;
  local_whisper_model: string;
  voice_processing_enabled: boolean;
  auto_transcribe_enabled: boolean;
  auto_vad_enabled: boolean;
  live_transcript_enabled: boolean;
  memory_dir: string;
  auto_extract_memories_enabled: boolean;
  feedback_sounds_enabled: boolean;
  auto_summarize_enabled: boolean;
  auto_extract_tasks_enabled: boolean;
  auto_name_enabled: boolean;
  wav_retention_days: number | null;
  privacy_mode: boolean;
  onboarding_completed: boolean;
  remote_endpoint: string;
  remote_auto_upload: boolean;
}

export function freshSettings(overrides: Partial<MockSettings> = {}): MockSettings {
  return {
    mic_device: null,
    system_audio_enabled: true,
    output_dir: "/tmp/Folio",
    tasks_path: "/tmp/Folio/Tasks.json",
    theme: "dark",
    transcriber: "local_whisper",
    transcription_language: "auto",
    briefing_language: "en",
    local_whisper_model: "large-v3",
    voice_processing_enabled: true,
    auto_transcribe_enabled: true,
    auto_vad_enabled: true,
    live_transcript_enabled: false,
    memory_dir: "/tmp/Folio/Memory",
    auto_extract_memories_enabled: false,
    feedback_sounds_enabled: false,
    auto_summarize_enabled: false,
    auto_extract_tasks_enabled: false,
    auto_name_enabled: false,
    wav_retention_days: null,
    privacy_mode: false,
    onboarding_completed: false,
    remote_endpoint: "",
    remote_auto_upload: false,
    ...overrides,
  };
}

export interface ScenarioOptions {
  initialSettings?: Partial<MockSettings>;

  startSignedIn?: boolean;

  passthroughUnknown?: boolean;

  recordings?: RecordingSummaryStub[];

  tasks?: TaskStub[];

  memories?: MemoryStub[];

  webhooks?: unknown[];

  providers?: ProviderStub[];
}

export type RecordingSummaryStub = Record<string, unknown>;
export type TaskStub = Record<string, unknown>;
export type MemoryStub = Record<string, unknown>;

export interface ProviderStub {
  id: string;
  name: string;
  has_key: boolean;
  redacted_key: string | null;
}

export async function setupScenario(page: Page, options: ScenarioOptions = {}) {
  const baseSettings = freshSettings({
    onboarding_completed: true,
    ...(options.initialSettings ?? {}),
  });
  const startSignedIn = options.startSignedIn ?? true;
  const passthroughUnknown = options.passthroughUnknown ?? true;

  await page.addInitScript(
    ([seed, signedIn, recordings, tasks, memories, webhooks, providers]) => {
      const w = window as unknown as Record<string, unknown>;
      w.__FOLIO_SETTINGS__ = JSON.parse(seed as string);
      w.__FOLIO_SIGNED_IN__ = signedIn as boolean;
      w.__FOLIO_INPUT_DEVICES__ = [
        { id: "default", name: "MacBook Pro Microphone" },
        { id: "blue-yeti", name: "Blue Yeti" },
      ];
      w.__FOLIO_RECORDINGS__ = JSON.parse(recordings as string);
      w.__FOLIO_FOLDERS__ = [];
      w.__FOLIO_TASKS__ = JSON.parse(tasks as string);
      w.__FOLIO_MEMORIES__ = JSON.parse(memories as string);
      w.__FOLIO_WEBHOOKS__ = JSON.parse(webhooks as string);
      w.__FOLIO_PROVIDERS__ = JSON.parse(providers as string);
      w.__FOLIO_REFERRAL_STATS__ = {
        token: "stub-token-aaa",
        share_url: "https://join.folio.app/t/stub-token-aaa",
        qualified_count: 0,
        pending_count: 0,
        free_months_earned: 0,
        yearly_cap: 100,
        yearly_remaining: 100,
      };
      w.__FOLIO_DEVICES__ = [
        {
          device_id: "this-mac",
          device_name: "MacBook Pro (this Mac)",
          created_at: new Date().toISOString(),
          last_seen_at: new Date().toISOString(),
          user_agent: "Folio e2e",
          ip: "127.0.0.1",
        },
      ];
    },
    [
      JSON.stringify(baseSettings),
      startSignedIn,
      JSON.stringify(options.recordings ?? []),
      JSON.stringify(options.tasks ?? []),
      JSON.stringify(options.memories ?? []),
      JSON.stringify(options.webhooks ?? []),
      JSON.stringify(options.providers ?? []),
    ] as const
  );

  await installTauriStub(page, {
    passthroughUnknown,
    handlers: {
      ping: () => "pong",

      get_settings: () => {
        return (window as unknown as Record<string, unknown>).__FOLIO_SETTINGS__;
      },
      save_settings: (args) => {
        const a = args as { settings: unknown };
        (window as unknown as Record<string, unknown>).__FOLIO_SETTINGS__ = a.settings;
        return null;
      },

      list_permissions: () => [
        {
          permission: "microphone",
          status: "granted",
          rationale: "",
          settings_url: "",
        },
        {
          permission: "screen_recording",
          status: "granted",
          rationale: "",
          settings_url: "",
        },
        { permission: "calendar", status: "unknown", rationale: "", settings_url: "" },
        {
          permission: "notifications",
          status: "unknown",
          rationale: "",
          settings_url: "",
        },
      ],
      open_permission_settings: () => null,
      request_calendar_access: () => null,
      list_attendee_suggestions: () => [],

      calendar_authorization_status: () =>
        (window as unknown as Record<string, unknown>).__FOLIO_CAL_ACCESS__ ??
        "not_determined",
      next_calendar_event: () =>
        (window as unknown as Record<string, unknown>).__FOLIO_NEXT_EVENT__ ?? null,

      auth_status: () => {
        if ((window as unknown as Record<string, unknown>).__FOLIO_SIGNED_IN__) {
          return {
            signed_in: true,
            identity: {
              user_id: "user-1",
              email: "ege@clinora.ai",
              display_name: "Ege Çelebi",
              privacy_tier: "tier1",
            },
          };
        }
        return { signed_in: false, identity: null };
      },
      auth_request_signin_code: () => null,
      auth_verify_signin_code: () => {
        (window as unknown as Record<string, unknown>).__FOLIO_SIGNED_IN__ = true;
        return {
          user_id: "user-1",
          email: "ege@clinora.ai",
          display_name: "Ege Çelebi",
          privacy_tier: "tier1",
        };
      },
      auth_logout: () => {
        (window as unknown as Record<string, unknown>).__FOLIO_SIGNED_IN__ = false;
        return null;
      },

      account_get: () => ({
        user: {
          _id: "user-1",
          email: "ege@clinora.ai",
          display_name: "Ege Çelebi",
          privacy_tier: "tier1",
          subscription_tier: "free",
        },
        device_count: 1,
      }),

      account_update: (args) => {
        const a = args as { displayName: string | null };
        return {
          id: "user-1",
          email: "ege@clinora.ai",
          display_name: a.displayName,
          privacy_tier: "tier1",
          subscription_tier: "free",
        };
      },
      account_devices: () => ({
        devices: (window as unknown as Record<string, unknown>).__FOLIO_DEVICES__,
      }),
      account_revoke_device: () => null,
      account_soft_delete: () => null,

      referrals_generate: () => ({
        token: "stub-token-aaa",
        share_url: "https://join.folio.app/t/stub-token-aaa",
      }),
      referrals_me: () => {
        return (window as unknown as Record<string, unknown>).__FOLIO_REFERRAL_STATS__;
      },
      referrals_redeem: () => null,

      settings_sync_pull: () => ({ settings: null, updated_at: null }),
      settings_sync_push: (args) => {
        const a = args as { settings: unknown; updatedAt: string };
        return { settings: a.settings, updated_at: a.updatedAt };
      },

      list_input_devices: () =>
        (window as unknown as Record<string, unknown>).__FOLIO_INPUT_DEVICES__,
      list_recordings: () =>
        (window as unknown as Record<string, unknown>).__FOLIO_RECORDINGS__,
      get_recording: (args) => {
        const a = args as { label: string };
        const recs = (window as unknown as Record<string, unknown>)
          .__FOLIO_RECORDINGS__ as Array<Record<string, unknown>>;
        return recs.find((r) => r.label === a.label) ?? null;
      },

      search_note_content: (args) => {
        const a = args as { query: string };
        const q = (a.query ?? "").trim().toLowerCase();
        if (q.length === 0) return [];
        const recs = (window as unknown as Record<string, unknown>)
          .__FOLIO_RECORDINGS__ as Array<Record<string, unknown>>;
        const hits: Array<Record<string, unknown>> = [];
        for (const r of recs) {
          const body = String(
            (r.transcript_text as string) ??
              (r.suggested_title as string) ??
              (r.label as string) ??
              ""
          );
          const idx = body.toLowerCase().indexOf(q);
          if (idx < 0) continue;
          const start = Math.max(0, idx - 30);
          const end = Math.min(body.length, idx + q.length + 30);
          const snippet =
            (start > 0 ? "…" : "") +
            body.slice(start, end) +
            (end < body.length ? "…" : "");
          hits.push({
            session_dir: r.session_dir,
            label: r.label,
            title: (r.title as string) ?? (r.suggested_title as string) ?? null,
            snippet,
            matched_in: "transcript",
          });
        }
        return hits;
      },
      delete_recording: () => null,
      reveal_in_finder: () => null,
      share_paths: () => null,

      export_note_markdown: (args) => {
        const a = args as { sessionDir: string };
        return `${a.sessionDir}/note.md`;
      },

      list_folders: () => [
        ...((window as unknown as Record<string, unknown>)
          .__FOLIO_FOLDERS__ as string[]),
      ],
      create_folder: (args) => {
        const a = args as { name: string };
        const w = window as unknown as Record<string, unknown>;
        const list = w.__FOLIO_FOLDERS__ as string[];
        const name = a.name.trim();
        if (name && !list.some((f) => f.toLowerCase() === name.toLowerCase()))
          w.__FOLIO_FOLDERS__ = [...list, name];
        return [...(w.__FOLIO_FOLDERS__ as string[])];
      },
      rename_folder: (args) => {
        const a = args as { from: string; to: string };
        const w = window as unknown as Record<string, unknown>;
        const list = w.__FOLIO_FOLDERS__ as string[];
        w.__FOLIO_FOLDERS__ = list.map((f) => (f === a.from ? a.to : f));
        return [...(w.__FOLIO_FOLDERS__ as string[])];
      },
      delete_folder: (args) => {
        const a = args as { name: string };
        const w = window as unknown as Record<string, unknown>;
        const list = w.__FOLIO_FOLDERS__ as string[];
        w.__FOLIO_FOLDERS__ = list.filter((f) => f !== a.name);
        return [...(w.__FOLIO_FOLDERS__ as string[])];
      },
      set_note_folder: (args) => {
        const a = args as { sessionDir: string; folder: string | null };
        const w = window as unknown as Record<string, unknown>;
        const list = w.__FOLIO_FOLDERS__ as string[];
        if (a.folder && !list.some((f) => f.toLowerCase() === a.folder!.toLowerCase()))
          w.__FOLIO_FOLDERS__ = [...list, a.folder];
        return null;
      },
      recording_status: () => ({
        recording: false,
        elapsed_secs: 0,
        channels: [],
        session_dir: null,
        paused: false,
      }),

      show_recording_bar: () => null,
      hide_recording_bar: () => null,
      recording_bar_stop: () => null,
      recording_bar_pause: () => null,
      recording_bar_resume: () => null,

      create_note: () => ({
        session_dir: "/tmp/Folio/2026-05-28-note",
        label: "2026-05-28-note",
        duration_seconds: 0,
        mic_bytes: null,
        system_bytes: null,
        mic_sample_rate: null,
        system_sample_rate: null,
        created_at: new Date().toISOString(),
        has_transcript: false,
        suggested_tags: [],
        draft_name: "Draft 1",
      }),

      rename_note: () => null,
      start_recording: () => ({
        recording: true,
        elapsed_secs: 0,
        channels: ["mic", "system"],
        session_dir: "/tmp/Folio/2026-05-28-note",
        paused: false,
      }),
      stop_recording: () => {
        const w = window as unknown as Record<string, unknown>;
        const recs = w.__FOLIO_RECORDINGS__ as Array<Record<string, unknown>>;
        if (!recs.some((r) => r.label === "2026-05-28-note")) {
          recs.unshift({
            session_dir: "/tmp/Folio/2026-05-28-note",
            label: "2026-05-28-note",
            duration_seconds: 60,
            mic_bytes: 1048576,
            system_bytes: null,
            mic_sample_rate: 48000,
            system_sample_rate: null,
            created_at: new Date().toISOString(),
            has_transcript: false,
            suggested_tags: [],
            draft_name: "Draft 1",
          });
        }
        return {
          artifacts: {
            session_dir: "/tmp/Folio/2026-05-28-note",
            mic_path: "/tmp/Folio/2026-05-28-note/mic.wav",
            system_path: null,
            started_at: "2026-05-28T14:00:00Z",
            stopped_at: "2026-05-28T14:01:00Z",
          },
          label: "2026-05-28-note",
        };
      },
      pause_recording: () => ({
        recording: false,
        elapsed_secs: 5,
        channels: [],
        session_dir: "/tmp/Folio/2026-05-28-note",
        paused: true,
      }),
      resume_recording: () => ({
        recording: true,
        elapsed_secs: 5,
        channels: ["mic", "system"],
        session_dir: "/tmp/Folio/2026-05-28-note",
        paused: false,
      }),
      save_live_notes: () => null,
      load_live_notes: () => [],
      ask_note: () => ({ answer: "That isn't covered in this meeting." }),
      ask_library: () => ({ answer: "No open action items found." }),
      transcribe_recording: () => ({
        transcript_path: "/tmp/Folio/2026-05-28-note/transcript.json",
        session_transcript: { channels: [] },
      }),
      read_transcript: () => ({ channels: [] }),
      save_transcript: () => "/tmp/Folio/2026-05-28-note/transcript.json",
      run_vad: () => ({ session_dir: "", channels: [], channel_errors: [] }),

      list_providers: () => {
        return (window as unknown as Record<string, unknown>).__FOLIO_PROVIDERS__;
      },
      set_provider_key: () => null,
      delete_provider_key: () => null,
      test_provider: () => ({ ok: true, latency_ms: 120 }),
      list_provider_models: () => [],

      remote_me: () => {
        const signed = (window as unknown as Record<string, unknown>)
          .__FOLIO_REMOTE_SIGNED_IN__;
        return signed
          ? { signed_in: true, email: "you@example.com" }
          : { signed_in: false, email: null };
      },
      test_remote_endpoint: () => ({
        ok: true,
        engine: "faster_whisper",
        model: "large-v3",
        gpu: true,
        message: "Connected to Folio Server v0.1.0",
      }),
      remote_login: () => {
        (window as unknown as Record<string, unknown>).__FOLIO_REMOTE_SIGNED_IN__ =
          true;
        return null;
      },
      remote_register: () => {
        (window as unknown as Record<string, unknown>).__FOLIO_REMOTE_SIGNED_IN__ =
          true;
        return null;
      },
      remote_logout: () => {
        (window as unknown as Record<string, unknown>).__FOLIO_REMOTE_SIGNED_IN__ =
          false;
        return null;
      },
      sync_recording: () => ({
        schema_version: 1,
        recording_id: "client-uuid",
        remote_recording_id: "srv-1",
        remote_job_id: "job-1",
        upload_state: "complete",
        remote_status: "succeeded",
        last_synced_at: new Date().toISOString(),
      }),
      get_sync_status: () => null,

      list_chat_threads: (args) => {
        const a = args as { scope?: string; sessionDir?: string };
        const all = JSON.parse(
          localStorage.getItem("__FOLIO_CHATS__") ?? "[]"
        ) as Array<Record<string, unknown>>;
        return all
          .filter((t) => (a.scope ? t.scope === a.scope : true))
          .filter((t) => (a.sessionDir ? t.session_dir === a.sessionDir : true))
          .sort((x, y) => String(y.updated_at).localeCompare(String(x.updated_at)));
      },
      save_chat_thread: (args) => {
        const a = args as { thread: Record<string, unknown> };
        const all = JSON.parse(
          localStorage.getItem("__FOLIO_CHATS__") ?? "[]"
        ) as Array<Record<string, unknown>>;
        const next = all.filter((t) => t.id !== a.thread.id);
        next.push(a.thread);
        localStorage.setItem("__FOLIO_CHATS__", JSON.stringify(next));
        return null;
      },
      delete_chat_thread: (args) => {
        const a = args as { id: string };
        const all = JSON.parse(
          localStorage.getItem("__FOLIO_CHATS__") ?? "[]"
        ) as Array<Record<string, unknown>>;
        localStorage.setItem(
          "__FOLIO_CHATS__",
          JSON.stringify(all.filter((t) => t.id !== a.id))
        );
        return null;
      },

      list_agents: () => [],
      run_agent: () => null,
      list_agent_runs: () => [],
      delete_agent_run: () => null,

      list_tasks: () => {
        return (window as unknown as Record<string, unknown>).__FOLIO_TASKS__;
      },
      create_task: (args) => {
        const a = args as { task?: { title?: string } };
        const title = a?.task?.title ?? "(untitled)";
        const next = {
          id: `t-${Date.now()}`,
          title,
          status: "todo",
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          due_at: null,
          source_session: null,
        };
        const list = (window as unknown as Record<string, unknown>)
          .__FOLIO_TASKS__ as unknown[];
        list.push(next);
        return next;
      },
      update_task: () => null,
      delete_task: () => null,
      set_task_status: () => null,

      list_memories: () => {
        return (window as unknown as Record<string, unknown>).__FOLIO_MEMORIES__;
      },
      get_memory: () => null,
      create_memory: () => null,
      update_memory: () => null,
      delete_memory: () => null,
      purge_memory: () => null,
      pin_memory: () => null,
      search_memories: () => [],
      memory_file_path: () => "",
      rebuild_memory_index: () => null,

      list_webhooks: () => {
        return (window as unknown as Record<string, unknown>).__FOLIO_WEBHOOKS__;
      },
      save_webhook: () => null,
      delete_webhook: () => null,
      test_webhook: () => ({ ok: true, status: 200 }),

      set_tray_recording: () => null,
      open_preferences_window: () => null,

      clear_recording_artifacts: () => null,
      export_vault_snapshot: () => null,
      purge_old_wav_files: () => null,
      generate_weekly_digest: () => null,
      export_share_bundle: () => null,
      git_sync_vault: () => null,
      git_vault_is_repo: () => false,
      list_inbox_entries: () => [],
      archive_inbox_entry: () => null,
      get_showcase: () => null,
      save_showcase: () => null,
    },
  });
}

export async function readSettings(page: Page): Promise<MockSettings> {
  return await page.evaluate(
    () =>
      (window as unknown as Record<string, unknown>).__FOLIO_SETTINGS__ as MockSettings
  );
}

export async function ipcCalls(page: Page, cmd: string) {
  const log = await ipcLog(page);
  return log.filter((e) => e.cmd === cmd);
}

export { ipcLog };
