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
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default = "default_language")]
    pub transcription_language: String,
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
    /// `transcriber` provider (OpenAI Whisper API requires
    /// `openai_api_key`; Local Whisper needs no key). When false the
    /// user transcribes manually from the Library row.
    #[serde(default = "default_auto_transcribe_enabled")]
    pub auto_transcribe_enabled: bool,
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
fn default_local_whisper_model() -> String {
    "large-v3".into()
}
fn default_voice_processing_enabled() -> bool {
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
            openai_api_key: String::new(),
            transcription_language: default_language(),
            dictionary_terms: Vec::new(),
            local_whisper_model: default_local_whisper_model(),
            voice_processing_enabled: default_voice_processing_enabled(),
            auto_transcribe_enabled: default_auto_transcribe_enabled(),
            auto_summarize_enabled: default_auto_summarize_enabled(),
            auto_extract_tasks_enabled: default_auto_extract_tasks_enabled(),
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
            openai_api_key: "sk-test".into(),
            ..Settings::default()
        };
        store.save(&s).unwrap();

        let loaded = store.load();
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.openai_api_key, "sk-test");
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
