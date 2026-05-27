//! User-facing settings: persisted, loaded, and saved as JSON.
//!
//! The shape of [`Settings`] is the contract the React frontend reads via
//! IPC. New fields must have a serde default so that loading an older
//! settings file doesn't fail.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use ts_rs::TS;

use crate::error::{AttuneError, Result};

/// Persisted user settings.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Settings {
    pub mic_device: Option<String>,
    pub system_audio_enabled: bool,
    pub output_dir: PathBuf,
    pub notes_dir: PathBuf,
    pub tasks_path: PathBuf,
    pub transcripts_dir: PathBuf,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_provider")]
    pub transcriber: String,
    #[serde(default = "default_language")]
    pub transcription_language: String,
    /// Language the LLM agents (summarise, extract-tasks, extract-memories,
    /// find-decisions, autoname, Q&A) must reply in regardless of the
    /// transcript's language. BCP-47 tag — `"auto"` keeps the legacy
    /// behaviour (mirror the meeting language); `"en"` etc. forces every
    /// agent output (and tool-call free-text like task titles + memory
    /// content) into that language. Default `"en"` because most users
    /// search + skim their library in English even when meetings are
    /// multilingual.
    #[serde(default = "default_briefing_language")]
    pub briefing_language: String,
    #[serde(default)]
    pub dictionary_terms: Vec<String>,
    /// Identifier of the local Whisper model the user has chosen (e.g.
    /// "large-v3", "small"). Used only when `transcriber == "local_whisper"`.
    #[serde(default = "default_local_whisper_model")]
    pub local_whisper_model: String,
    /// macOS only. When true, mic capture goes through Apple's Voice
    /// Processing IO AudioUnit (AEC + noise suppression + AGC) so the
    /// mic stops picking up speaker bleed when the user is not
    /// wearing headphones. Falls back to the plain cpal path on
    /// VPIO init failure. Ignored on non-macOS targets.
    #[serde(default = "default_voice_processing_enabled")]
    pub voice_processing_enabled: bool,
    /// When true, the app starts transcribing automatically as soon as
    /// a recording is stopped. Honours the currently-selected
    /// `transcriber` provider (OpenAI Whisper API requires a Keychain-
    /// stored key via `KeyStore::set`; Local Whisper needs no key).
    /// When false the user transcribes manually from the Library row.
    #[serde(default = "default_auto_transcribe_enabled")]
    pub auto_transcribe_enabled: bool,
    /// When true, the VAD pre-pass runs as its own job before
    /// transcription. Strips silence from both the mic and system
    /// tracks so the ASR only ever sees speech-bearing audio.
    /// Default ON because the silence-hallucination failure mode is
    /// expensive (the 2026-05-26-11-47-54 mic.wav incident) and the
    /// pre-pass is cheap. Set to false to send the raw recordings
    /// straight to the ASR.
    #[serde(default = "default_auto_vad_enabled")]
    pub auto_vad_enabled: bool,
    /// Root directory for the local memory layer. Defaults to a
    /// subtree of the user's Obsidian vault per the
    /// `ai-chat-multi-provider.md` plan; falls back to
    /// `~/Documents/Attune/Memory/` when the vault parent does not
    /// exist. The `MemoryStore` creates this directory on first
    /// write — callers should not assume it exists.
    #[serde(default = "default_memory_dir")]
    pub memory_dir: PathBuf,
    /// When true, runs the `extract-memories` agent automatically
    /// after every transcription. Mirrors `auto_summarize_enabled`
    /// and `auto_extract_tasks_enabled`. Skipped silently if no AI
    /// key is set.
    #[serde(default = "default_auto_extract_memories_enabled")]
    pub auto_extract_memories_enabled: bool,
    /// When true, the app plays short synthesised tones on the
    /// recording lifecycle (start, stop, agent success, error).
    /// Default off — users opt in via Settings. v2 finding 019.
    #[serde(default = "default_feedback_sounds_enabled")]
    pub feedback_sounds_enabled: bool,
    /// When true and an AI provider key is configured, the app
    /// automatically runs the `summarize` agent immediately after a
    /// transcription completes. Lets the user stop a meeting and walk
    /// away knowing the summary will be on the recording's page when
    /// they come back. Falls back to a no-op when no AI key is set.
    #[serde(default = "default_auto_summarize_enabled")]
    pub auto_summarize_enabled: bool,
    /// When true and an AI provider key is configured, the app
    /// automatically runs the `extract-tasks` agent after a
    /// transcription completes. The agent uses the `create_task`
    /// tool to populate the kanban directly, so the user can stop a
    /// meeting and come back to a populated to-do board. Skipped
    /// silently if no AI key is set.
    #[serde(default = "default_auto_extract_tasks_enabled")]
    pub auto_extract_tasks_enabled: bool,
    /// When true and an AI provider key is configured, fires the
    /// `autoname` agent right after transcription completes so the
    /// library row shows a suggested human-readable title + tags +
    /// subtitle. The result is stored as a normal AgentRun under
    /// `<session_dir>/agent_runs/autoname.json`; the library scan
    /// surfaces the title in RecordingSummary so the UI can render
    /// it without any extra IPC roundtrips. v2 finding 024 / GET-37.
    #[serde(default = "default_auto_name_enabled")]
    pub auto_name_enabled: bool,
    /// Optional retention policy for the source WAV files inside each
    /// session directory. None / 0 leaves WAVs in place forever; a
    /// positive value N tells `purge_old_wavs` to delete mic.wav +
    /// system.wav once a transcript exists AND the session's most
    /// recent modification is older than N days. Audio recordings
    /// grow fast (~87GB/yr for a daily-meeting user); this setting
    /// plus the manual 'Purge now' button in Settings → Storage
    /// lets the user keep transcripts but drop the source audio.
    /// v2 finding 063 / GET-98.
    #[serde(default)]
    pub wav_retention_days: Option<u32>,
    /// Opt-in toggle for the public-aggregate stats counter. v2
    /// finding 095 / GET-110. When true, the app uploads three
    /// numbers (minutes-transcribed-locally, USD-saved-aggregate,
    /// active-install ping) to the public counter; no content, no
    /// identifiers ever cross the wire. Default OFF — the user opts
    /// in from Settings → Privacy. The upload path itself lands in
    /// the follow-up PR; this setting ships the consent surface so
    /// adding the upload later is a zero-UI change.
    #[serde(default)]
    pub share_aggregate_stats: bool,
    /// Attune Pro license key (v2 finding 092 / GET-108). Empty
    /// string = Free tier. Non-empty = Pro — gates auto-record,
    /// multi-window, marketplace install, and other paid features
    /// downstream PRs will wire. The actual signature-verification
    /// of the key is a follow-up; for now the presence of a
    /// non-empty value flips the tier.
    #[serde(default)]
    pub pro_license_key: String,
    /// RFC-3339 timestamp the user started the 14-day Pro trial.
    /// Empty string = trial never started. The UI computes
    /// remaining days client-side and locks Pro features when more
    /// than 14 days have elapsed without a license_key. v2 finding
    /// 094 / GET-109.
    #[serde(default)]
    pub pro_trial_started_at: String,
    /// Apple Reminders two-way sync — when enabled, kanban tasks
    /// publish to the named Reminders list and #attune-tagged
    /// reminders pull into the kanban inbox column. Default OFF;
    /// requires the user to grant Reminders permission on first
    /// run. v2 finding 076 / GET-78.
    #[serde(default)]
    pub reminders_sync_enabled: bool,
    /// Reminders list name to mirror to/from. Defaults to "Attune"
    /// (we create the list on first sync if it doesn't exist).
    #[serde(default = "default_reminders_list")]
    pub reminders_list_name: String,
    /// Privacy Mode / Airgap (v2 finding 048 / GET-42). When true the
    /// CloudGuard blocks every outbound HTTP request except to
    /// localhost. Cloud LLM providers, embedding APIs, model
    /// downloads, and webhook delivery all short-circuit with a clear
    /// "blocked by Privacy Mode" error. The titlebar shows an AIRGAP
    /// badge while this is on. Defaults to false.
    #[serde(default)]
    pub privacy_mode: bool,
    /// Voice-debrief on Stop (v2 finding 027 / GET-53). When true, the
    /// app pops a small sheet right after the user hits Stop that asks
    /// 'anything to capture before this fades?' and records up to 20s
    /// of mic. The clip lands next to the meeting as `debrief.webm`
    /// and the existing extract-tasks / extract-memories agents fire
    /// against its transcript. Default OFF — opt in from Settings.
    #[serde(default)]
    pub voice_debrief_enabled: bool,
    /// One-screen first-run conductor completion flag. v2 finding 001
    /// / GET-24. True after the user has either finished the
    /// onboarding screen or explicitly dismissed it. The Record route
    /// renders the conductor while this is false.
    #[serde(default)]
    pub onboarding_completed: bool,

    // ============================================================
    // Sprint 1 onboarding rebuild (GET-133 / GET-134 / GET-135).
    // ============================================================
    /// GET-133 General. When true, a thin floating indicator pulses
    /// on the right edge of the screen while Attune is transcribing.
    /// The eventual home of the consensus #11 Acoustic Confidence
    /// Strip surface; defaults ON because Tony's Calm Computing
    /// principle says recording state must always be visible.
    #[serde(default = "default_live_meeting_indicator")]
    pub live_meeting_indicator: bool,
    /// GET-133 General. macOS Login Items API binding. When true,
    /// Attune launches on user login via the SMAppService
    /// background-task entitlement. Default OFF — opt in from
    /// Settings.
    #[serde(default)]
    pub open_at_login: bool,
    /// GET-133 General. When a meeting starts (detected via
    /// EventKit or auto-detect), Attune repositions itself out of
    /// the way so the user can keep typing notes alongside their
    /// conferencing app. Default OFF until the implicit-brief
    /// surface lands.
    #[serde(default)]
    pub move_aside_in_meetings: bool,
    /// GET-133 Privacy. Default visibility for shared-meeting links.
    /// `"workspace_only"` (default, stricter than Granola),
    /// `"anyone_with_link"`, or `"disabled"`.
    #[serde(default = "default_link_sharing")]
    pub default_link_sharing: String,
    /// GET-133 Privacy. When the user clicks a shared-meeting link
    /// in their browser, deep-link into the desktop app instead of
    /// the web view. Default ON.
    #[serde(default = "default_always_open_shared_links")]
    pub always_open_shared_links: bool,
    /// GET-133 Privacy. Roundtable consensus #5 — show a coloured
    /// left border on every artefact indicating where it lives
    /// (green = on-device only, amber = Apple PCC / encrypted cloud,
    /// red = third-party cloud). Default ON.
    #[serde(default = "default_privacy_tier_band_enabled")]
    pub privacy_tier_band_enabled: bool,
    /// GET-133 Privacy. Number of days after which transcripts auto-
    /// delete. GDPR Art. 5(1)(c) data minimisation default = 90
    /// days; Granola defaults to Off, which is an Art. 5 violation
    /// in the EU. Range UI: Off / 7 / 30 / 90 / 365.
    #[serde(default = "default_auto_delete_period_days")]
    pub auto_delete_period_days: Option<u32>,

    /// GET-134 Calendar. Show next-meeting + countdown in the macOS
    /// menu bar. Default ON — feeds the consensus #9 Quiet Mode dot.
    #[serde(default = "default_show_upcoming_meetings_in_menubar")]
    pub show_upcoming_meetings_in_menubar: bool,
    /// GET-134 Calendar. Power-user knob hidden behind an Advanced
    /// disclosure. When true, the "Coming up" section includes
    /// events without attendees / video links (e.g. focus blocks).
    /// Default OFF.
    #[serde(default)]
    pub show_events_without_participants: bool,

    /// GET-135 Notifications. Fire a notification 1 minute before
    /// a calendared meeting starts. Default ON.
    #[serde(default = "default_notify_scheduled_meetings")]
    pub notify_scheduled_meetings: bool,
    /// GET-135 Notifications. Detect via NSRunningApplication that
    /// a known conferencing app started a call, then ask the user
    /// to capture. Default ON.
    #[serde(default = "default_notify_auto_detected_meetings")]
    pub notify_auto_detected_meetings: bool,
    /// GET-135 Notifications. Bundle identifiers the user does NOT
    /// want auto-detect notifications for. Per-app mute escape
    /// hatch — Granola pattern.
    #[serde(default)]
    pub notification_muted_apps: Vec<String>,
    /// GET-135 Notifications. Where to surface "a teammate shared a
    /// note with you" events. `"activity_and_email"` (default),
    /// `"activity_only"`, `"email_only"`, or `"none"`.
    #[serde(default = "default_note_shared_notification")]
    pub note_shared_notification: String,
}

fn default_theme() -> String {
    "light".into()
}
fn default_provider() -> String {
    "openai".into()
}
fn default_language() -> String {
    "auto".into()
}
fn default_briefing_language() -> String {
    "en".into()
}
fn default_local_whisper_model() -> String {
    "large-v3".into()
}
fn default_voice_processing_enabled() -> bool {
    true
}
fn default_auto_vad_enabled() -> bool {
    true
}
fn default_auto_transcribe_enabled() -> bool {
    true
}
fn default_auto_summarize_enabled() -> bool {
    true
}
fn default_auto_extract_tasks_enabled() -> bool {
    true
}
fn default_auto_extract_memories_enabled() -> bool {
    true
}
fn default_auto_name_enabled() -> bool {
    true
}
fn default_reminders_list() -> String {
    "Attune".into()
}
fn default_feedback_sounds_enabled() -> bool {
    false
}
fn default_live_meeting_indicator() -> bool {
    true
}
fn default_link_sharing() -> String {
    "workspace_only".into()
}
fn default_always_open_shared_links() -> bool {
    true
}
fn default_privacy_tier_band_enabled() -> bool {
    true
}
fn default_auto_delete_period_days() -> Option<u32> {
    // GDPR Art. 5(1)(c) data minimisation — keep transcripts no
    // longer than the user needs them. 90 days is the consensus
    // default among privacy-aware tools (Signal, ProtonMail).
    Some(90)
}
fn default_show_upcoming_meetings_in_menubar() -> bool {
    true
}
fn default_notify_scheduled_meetings() -> bool {
    true
}
fn default_notify_auto_detected_meetings() -> bool {
    true
}
fn default_note_shared_notification() -> String {
    "activity_and_email".into()
}
/// Resolve the default memory directory.
///
/// We try the Obsidian-vault subtree first (the canonical SSOT per the
/// vault's `ai-chat-multi-provider.md` plan): everything Attune
/// produces — recordings, tasks, memories — should be `git push`-able
/// together. If the vault's parent (`me/`) does not exist, the user
/// either has not cloned the vault or runs Attune standalone; fall
/// back to a vault-free path under their home directory so the app
/// still works.
fn default_memory_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let vault_root = home
        .join("Documents")
        .join("GitHub")
        .join("obsidian.md")
        .join("me");
    if vault_root.is_dir() {
        vault_root.join("meetings").join(".attune").join("memory")
    } else {
        home.join("Documents").join("Attune").join("Memory")
    }
}

impl Default for Settings {
    fn default() -> Self {
        let attune = default_home_dir();
        Self {
            mic_device: None,
            system_audio_enabled: true,
            output_dir: attune.join("Recordings"),
            notes_dir: attune.join("Notes"),
            tasks_path: attune.join("Tasks").join("tasks.json"),
            transcripts_dir: attune.join("Transcripts"),
            theme: default_theme(),
            transcriber: default_provider(),
            transcription_language: default_language(),
            briefing_language: default_briefing_language(),
            dictionary_terms: Vec::new(),
            local_whisper_model: default_local_whisper_model(),
            voice_processing_enabled: default_voice_processing_enabled(),
            auto_transcribe_enabled: default_auto_transcribe_enabled(),
            auto_vad_enabled: default_auto_vad_enabled(),
            memory_dir: default_memory_dir(),
            auto_extract_memories_enabled: default_auto_extract_memories_enabled(),
            feedback_sounds_enabled: default_feedback_sounds_enabled(),
            auto_summarize_enabled: default_auto_summarize_enabled(),
            auto_extract_tasks_enabled: default_auto_extract_tasks_enabled(),
            auto_name_enabled: default_auto_name_enabled(),
            wav_retention_days: None,
            share_aggregate_stats: false,
            pro_license_key: String::new(),
            pro_trial_started_at: String::new(),
            reminders_sync_enabled: false,
            reminders_list_name: default_reminders_list(),
            privacy_mode: false,
            voice_debrief_enabled: false,
            onboarding_completed: false,
            live_meeting_indicator: default_live_meeting_indicator(),
            open_at_login: false,
            move_aside_in_meetings: false,
            default_link_sharing: default_link_sharing(),
            always_open_shared_links: default_always_open_shared_links(),
            privacy_tier_band_enabled: default_privacy_tier_band_enabled(),
            auto_delete_period_days: default_auto_delete_period_days(),
            show_upcoming_meetings_in_menubar: default_show_upcoming_meetings_in_menubar(),
            show_events_without_participants: false,
            notify_scheduled_meetings: default_notify_scheduled_meetings(),
            notify_auto_detected_meetings: default_notify_auto_detected_meetings(),
            notification_muted_apps: Vec::new(),
            note_shared_notification: default_note_shared_notification(),
        }
    }
}

fn default_home_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Documents").join("Attune")
}

/// Loads and saves [`Settings`] to a JSON file on disk.
///
/// The store owns the path so callers do not have to thread it around.
/// A missing file is treated as "use defaults"; a malformed file falls
/// back to defaults with a warning rather than failing the app launch.
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    /// Construct a store backed by `path`. The file does not need to exist
    /// yet; [`SettingsStore::load`] returns [`Settings::default`] in that
    /// case.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Default store rooted at the platform's standard config directory.
    /// On macOS this is `~/Library/Application Support/Attune/settings.json`.
    pub fn default_location() -> Self {
        Self::new(default_settings_path())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load settings from disk. Missing files yield defaults; malformed
    /// files log a warning and yield defaults rather than aborting.
    pub fn load(&self) -> Settings {
        match fs::read_to_string(&self.path) {
            Ok(contents) => match serde_json::from_str::<Settings>(&contents) {
                Ok(settings) => {
                    debug!(path = %self.path.display(), "settings loaded");
                    settings
                }
                Err(e) => {
                    warn!(
                        path = %self.path.display(),
                        error = %e,
                        "settings file is malformed; falling back to defaults",
                    );
                    Settings::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!(path = %self.path.display(), "no settings file; using defaults");
                Settings::default()
            }
            Err(e) => {
                warn!(
                    path = %self.path.display(),
                    error = %e,
                    "could not read settings file; falling back to defaults",
                );
                Settings::default()
            }
        }
    }

    /// Atomically write settings to disk. Creates the parent directory if
    /// it does not yet exist. Writes to a sibling temp file then renames,
    /// so a crash mid-write cannot corrupt the on-disk file.
    pub fn save(&self, settings: &Settings) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AttuneError::Storage(format!(
                    "could not create settings dir {}: {e}",
                    parent.display()
                ))
            })?;
        }

        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| AttuneError::Storage(format!("could not serialize settings: {e}")))?;

        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| {
            AttuneError::Storage(format!(
                "could not write settings temp file {}: {e}",
                tmp.display()
            ))
        })?;
        fs::rename(&tmp, &self.path).map_err(|e| {
            AttuneError::Storage(format!(
                "could not finalize settings file {}: {e}",
                self.path.display()
            ))
        })?;

        info!(path = %self.path.display(), "settings saved");
        Ok(())
    }
}

fn default_settings_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Application Support")
            .join("Attune")
            .join("settings.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".config").join("attune").join("settings.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_returns_defaults_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let store = SettingsStore::new(dir.path().join("settings.json"));
        let s = store.load();
        assert_eq!(s.theme, "light");
        assert!(s.system_audio_enabled);
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = TempDir::new().unwrap();
        let store = SettingsStore::new(dir.path().join("settings.json"));
        let s = Settings {
            theme: "dark".into(),
            transcription_language: "tr".into(),
            ..Settings::default()
        };
        store.save(&s).unwrap();

        let loaded = store.load();
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.transcription_language, "tr");
    }

    #[test]
    fn malformed_file_falls_back_to_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "not valid json {{").unwrap();
        let store = SettingsStore::new(path);
        let s = store.load();
        assert_eq!(s.theme, "light");
    }

    #[test]
    fn save_creates_missing_parent_dir() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("a").join("b").join("settings.json");
        let store = SettingsStore::new(&nested);
        store.save(&Settings::default()).unwrap();
        assert!(nested.exists());
    }
}
