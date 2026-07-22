use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use ts_rs::TS;

use crate::error::{FolioError, Result};

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

    #[serde(default = "default_briefing_language")]
    pub briefing_language: String,
    #[serde(default)]
    pub dictionary_terms: Vec<String>,

    #[serde(default = "default_local_whisper_model")]
    pub local_whisper_model: String,

    #[serde(default = "default_voice_processing_enabled")]
    pub voice_processing_enabled: bool,

    #[serde(default = "default_auto_transcribe_enabled")]
    pub auto_transcribe_enabled: bool,

    #[serde(default = "default_auto_vad_enabled")]
    pub auto_vad_enabled: bool,

    #[serde(default)]
    pub system_audio_enhancement: SystemAudioEnhancement,

    #[serde(default = "default_true")]
    pub diarization_enabled: bool,

    #[serde(default = "default_live_transcript_enabled")]
    pub live_transcript_enabled: bool,

    #[serde(default = "default_memory_dir")]
    pub memory_dir: PathBuf,

    #[serde(default = "default_auto_extract_memories_enabled")]
    pub auto_extract_memories_enabled: bool,

    #[serde(default = "default_feedback_sounds_enabled")]
    pub feedback_sounds_enabled: bool,

    #[serde(default = "default_auto_summarize_enabled")]
    pub auto_summarize_enabled: bool,

    #[serde(default = "default_auto_extract_tasks_enabled")]
    pub auto_extract_tasks_enabled: bool,

    #[serde(default = "default_auto_name_enabled")]
    pub auto_name_enabled: bool,

    #[serde(default)]
    pub wav_retention_days: Option<u32>,

    #[serde(default)]
    pub share_aggregate_stats: bool,

    #[serde(default)]
    pub pro_license_key: String,

    #[serde(default)]
    pub pro_trial_started_at: String,

    #[serde(default)]
    pub reminders_sync_enabled: bool,

    #[serde(default = "default_reminders_list")]
    pub reminders_list_name: String,

    #[serde(default)]
    pub privacy_mode: bool,

    #[serde(default)]
    pub voice_debrief_enabled: bool,

    #[serde(default)]
    pub onboarding_completed: bool,

    #[serde(default = "default_live_meeting_indicator")]
    pub live_meeting_indicator: bool,

    #[serde(default)]
    pub open_at_login: bool,

    #[serde(default)]
    pub move_aside_in_meetings: bool,

    #[serde(default = "default_link_sharing")]
    pub default_link_sharing: String,

    #[serde(default = "default_always_open_shared_links")]
    pub always_open_shared_links: bool,

    #[serde(default = "default_privacy_tier_band_enabled")]
    pub privacy_tier_band_enabled: bool,

    #[serde(default = "default_auto_delete_period_days")]
    pub auto_delete_period_days: Option<u32>,

    #[serde(default = "default_show_upcoming_meetings_in_menubar")]
    pub show_upcoming_meetings_in_menubar: bool,

    #[serde(default)]
    pub show_events_without_participants: bool,

    #[serde(default = "default_notify_scheduled_meetings")]
    pub notify_scheduled_meetings: bool,

    #[serde(default = "default_notify_auto_detected_meetings")]
    pub notify_auto_detected_meetings: bool,

    #[serde(default)]
    pub notification_muted_apps: Vec<String>,

    #[serde(default = "default_note_shared_notification")]
    pub note_shared_notification: String,

    #[serde(default)]
    pub signin_mode: String,

    #[serde(default)]
    pub workspace_name: String,

    #[serde(default)]
    pub workspace_bucket: String,

    #[serde(default)]
    pub onboarding_calendar_deferred: bool,

    #[serde(default = "default_true")]
    pub workspace_discoverable: bool,

    #[serde(default = "default_true")]
    pub workspace_auto_join: bool,

    #[serde(default)]
    pub workspace_logo_path: String,

    #[serde(default)]
    pub auto_segment_secs: Option<u64>,

    #[serde(default)]
    pub remote_endpoint: String,

    #[serde(default)]
    pub remote_auto_upload: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct SystemAudioEnhancement {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_enhancement_atten_lim_db")]
    pub atten_lim_db: f32,
}

impl Default for SystemAudioEnhancement {
    fn default() -> Self {
        Self {
            enabled: false,
            atten_lim_db: default_enhancement_atten_lim_db(),
        }
    }
}

fn default_enhancement_atten_lim_db() -> f32 {
    -20.0
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "light".into()
}
fn default_provider() -> String {
    "local_whisper".into()
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
    false
}
fn default_auto_vad_enabled() -> bool {
    true
}
fn default_live_transcript_enabled() -> bool {
    false
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
    "Folio".into()
}
fn default_feedback_sounds_enabled() -> bool {
    false
}
fn default_live_meeting_indicator() -> bool {
    true
}
fn default_link_sharing() -> String {
    "disabled".into()
}
fn default_always_open_shared_links() -> bool {
    true
}
fn default_privacy_tier_band_enabled() -> bool {
    true
}
fn default_auto_delete_period_days() -> Option<u32> {
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
        vault_root.join("meetings").join(".folio").join("memory")
    } else {
        home.join("Documents").join("Folio").join("Memory")
    }
}

impl Default for Settings {
    fn default() -> Self {
        let folio = default_home_dir();
        Self {
            mic_device: None,
            system_audio_enabled: true,
            output_dir: folio.join("Recordings"),
            notes_dir: folio.join("Notes"),
            tasks_path: folio.join("Tasks").join("tasks.json"),
            transcripts_dir: folio.join("Transcripts"),
            theme: default_theme(),
            transcriber: default_provider(),
            transcription_language: default_language(),
            briefing_language: default_briefing_language(),
            dictionary_terms: Vec::new(),
            local_whisper_model: default_local_whisper_model(),
            voice_processing_enabled: default_voice_processing_enabled(),
            auto_transcribe_enabled: default_auto_transcribe_enabled(),
            auto_vad_enabled: default_auto_vad_enabled(),
            system_audio_enhancement: SystemAudioEnhancement::default(),
            diarization_enabled: default_true(),
            live_transcript_enabled: default_live_transcript_enabled(),
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
            signin_mode: String::new(),
            workspace_name: String::new(),
            workspace_bucket: String::new(),
            onboarding_calendar_deferred: false,
            workspace_discoverable: true,
            workspace_auto_join: true,
            workspace_logo_path: String::new(),
            auto_segment_secs: None,
            remote_endpoint: String::new(),
            remote_auto_upload: false,
        }
    }
}

fn default_home_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Documents").join("Folio")
}

pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_location() -> Self {
        Self::new(default_settings_path())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

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

    pub fn save(&self, settings: &Settings) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                FolioError::Storage(format!(
                    "could not create settings dir {}: {e}",
                    parent.display()
                ))
            })?;
        }

        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| FolioError::Storage(format!("could not serialize settings: {e}")))?;

        crate::storage::atomic_write::atomic_write(&self.path, json.as_bytes())?;

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
            .join("Folio")
            .join("settings.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".config").join("folio").join("settings.json")
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
