//! The Attune main application.
//!
//! State machine:
//!
//!   Idle ──[click Record]──► Recording ──[click Stop]──► Idle
//!     │                          │
//!     │                          └─[start error]──► Idle (with error)
//!     │
//!     └──[refresh]──► re-list devices
//!
//! The cpal stream owned by `CaptureSession` runs on its own thread inside
//! cpal. We hold the session in app state and tick the UI every frame so
//! the duration counter and recording pulse animate smoothly.

use std::path::PathBuf;
use std::time::Instant;

use attune_core::audio::{
    list_input_devices, CaptureArtifacts, CaptureConfig, CaptureSession, DeviceInfo,
};
use egui::{Align, Color32, Layout, RichText, Sense, Vec2};
use serde::{Deserialize, Serialize};

use crate::theme::{RECORD_RED, STRONG, SUBTLE, SUBTLER, SUCCESS_GREEN};

pub struct AttuneApp {
    persisted: PersistedState,

    /// Cached list of input devices. Refreshed on demand.
    devices: Vec<DeviceInfo>,
    /// Most recent error to surface in the UI, cleared on next user action.
    last_error: Option<String>,
    /// Active recording session, if any. `None` when idle.
    session: Option<CaptureSession>,
    /// When the active session started. Used for the duration counter.
    recording_started: Option<Instant>,
    /// Recent recordings (most recent first).
    history: Vec<RecordingSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedState {
    mic_device: Option<String>,
    system_audio_enabled: bool,
    output_dir: PathBuf,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            mic_device: None,
            system_audio_enabled: true,
            output_dir: PathBuf::from("./recordings"),
        }
    }
}

#[derive(Clone, Debug)]
struct RecordingSummary {
    session_dir: PathBuf,
    label: String,
    duration_seconds: i64,
    mic_bytes: Option<u64>,
    system_bytes: Option<u64>,
}

impl AttuneApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let persisted: PersistedState = cc
            .storage
            .and_then(|s| eframe::get_value::<PersistedState>(s, eframe::APP_KEY))
            .unwrap_or_default();
        let mut app = Self {
            persisted,
            devices: Vec::new(),
            last_error: None,
            session: None,
            recording_started: None,
            history: Vec::new(),
        };
        app.refresh_devices();
        app.refresh_history();
        app
    }

    fn refresh_devices(&mut self) {
        match list_input_devices() {
            Ok(d) => {
                // If the persisted device is no longer present, fall back to default.
                if let Some(name) = &self.persisted.mic_device {
                    if !d.iter().any(|x| &x.name == name) {
                        self.persisted.mic_device = None;
                    }
                }
                self.devices = d;
            }
            Err(e) => {
                self.last_error = Some(format!("device enumeration: {e}"));
                self.devices = Vec::new();
            }
        }
    }

    fn refresh_history(&mut self) {
        self.history = scan_history(&self.persisted.output_dir);
    }

    fn start_recording(&mut self) {
        self.last_error = None;
        let config = CaptureConfig {
            mic_enabled: true,
            system_enabled: self.persisted.system_audio_enabled,
            mic_device_name: self.persisted.mic_device.clone(),
            target_sample_rate: 16_000,
            output_dir: self.persisted.output_dir.clone(),
        };

        match CaptureSession::start(config) {
            Ok(session) => {
                let channels = session.channels_active();
                if channels.is_empty() {
                    self.last_error = Some(
                        "No capture channels available. Check microphone permission in System Settings → Privacy."
                            .into(),
                    );
                    return;
                }
                self.recording_started = Some(Instant::now());
                self.session = Some(session);
            }
            Err(e) => {
                self.last_error = Some(format!("Could not start recording: {e}"));
            }
        }
    }

    fn stop_recording(&mut self) {
        let Some(session) = self.session.take() else {
            return;
        };
        match session.stop() {
            Ok(artifacts) => {
                self.history.insert(0, summarize(&artifacts));
                self.history.truncate(20);
            }
            Err(e) => {
                self.last_error = Some(format!("Stop failed: {e}"));
            }
        }
        self.recording_started = None;
    }

    fn pick_output_dir(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.persisted.output_dir = dir;
            self.refresh_history();
        }
    }

    fn is_recording(&self) -> bool {
        self.session.is_some()
    }

    fn elapsed_label(&self) -> String {
        match self.recording_started {
            Some(t) => {
                let secs = t.elapsed().as_secs();
                let m = secs / 60;
                let s = secs % 60;
                format!("{m:02}:{s:02}")
            }
            None => "00:00".into(),
        }
    }
}

impl eframe::App for AttuneApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.persisted);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keep ticking while recording so the duration counter + pulse animate.
        if self.is_recording() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            self.header(ui);
            ui.add_space(18.0);
            self.status_section(ui, ctx);
            ui.add_space(12.0);
            self.separator(ui);
            ui.add_space(6.0);
            self.devices_section(ui);
            ui.add_space(6.0);
            self.system_audio_section(ui);
            ui.add_space(6.0);
            self.output_section(ui);
            ui.add_space(20.0);
            self.action_section(ui);
            ui.add_space(20.0);
            self.separator(ui);
            ui.add_space(6.0);
            self.history_section(ui);
            ui.add_space(8.0);
            self.footer(ui);
        });
    }
}

// ---------------------------------------------------------------------------
// UI sections
// ---------------------------------------------------------------------------

impl AttuneApp {
    fn header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("attune").heading().color(STRONG).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .color(SUBTLER),
                );
            });
        });
        ui.label(
            RichText::new("local-first meeting transcription")
                .small()
                .color(SUBTLE),
        );
    }

    fn status_section(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let frame = egui::Frame::default()
            .fill(Color32::from_rgb(22, 22, 22))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .corner_radius(egui::CornerRadius::same(8))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(34, 34, 34)));
        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                let recording = self.is_recording();

                // Indicator dot.
                let dot_size = 10.0;
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(dot_size), Sense::hover());
                let painter = ui.painter();
                if recording {
                    let t = ctx.input(|i| i.time);
                    let pulse = 0.55 + 0.45 * (t * 3.5).sin();
                    let alpha = (pulse.clamp(0.0, 1.0) * 255.0) as u8;
                    let glow = Color32::from_rgba_unmultiplied(232, 76, 76, alpha);
                    painter.circle_filled(rect.center(), dot_size * 0.9, glow);
                    painter.circle_filled(rect.center(), dot_size * 0.45, RECORD_RED);
                } else {
                    painter.circle_stroke(
                        rect.center(),
                        dot_size * 0.45,
                        egui::Stroke::new(1.5, Color32::from_rgb(90, 90, 90)),
                    );
                }

                ui.add_space(6.0);
                if recording {
                    ui.label(RichText::new("recording").size(15.0).color(STRONG));
                } else {
                    ui.label(RichText::new("idle").size(15.0).color(SUBTLE));
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(self.elapsed_label())
                            .monospace()
                            .size(16.0)
                            .color(if recording { STRONG } else { SUBTLER }),
                    );
                });
            });
        });
    }

    fn devices_section(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("input device").small().color(SUBTLE));
        ui.add_space(2.0);

        let recording = self.is_recording();
        let selected_label = match &self.persisted.mic_device {
            Some(name) => name.clone(),
            None => {
                // Show actual default device name if known.
                self.devices
                    .iter()
                    .find(|d| d.is_default)
                    .map(|d| format!("{}  (default)", d.name))
                    .unwrap_or_else(|| "(default)".into())
            }
        };

        ui.horizontal(|ui| {
            ui.add_enabled_ui(!recording, |ui| {
                egui::ComboBox::from_id_salt("mic_device_combo")
                    .selected_text(selected_label)
                    .width(ui.available_width() - 88.0)
                    .show_ui(ui, |ui| {
                        let default_selected = self.persisted.mic_device.is_none();
                        if ui
                            .selectable_label(default_selected, "(system default)")
                            .clicked()
                        {
                            self.persisted.mic_device = None;
                        }
                        ui.separator();
                        for d in &self.devices {
                            let selected = self.persisted.mic_device.as_deref() == Some(&d.name);
                            let label = if d.is_default {
                                format!("{}  ·  default", d.name)
                            } else {
                                d.name.clone()
                            };
                            if ui.selectable_label(selected, label).clicked() {
                                self.persisted.mic_device = Some(d.name.clone());
                            }
                        }
                    });
            });

            ui.add_enabled_ui(!recording, |ui| {
                if ui
                    .add_sized([76.0, 28.0], egui::Button::new("refresh"))
                    .clicked()
                {
                    self.refresh_devices();
                }
            });
        });

        if let Some(d) = self.selected_device_meta() {
            let sr = d
                .default_sample_rate
                .map(|s| format!("{} Hz", s))
                .unwrap_or_else(|| "unknown".into());
            let ch = d
                .default_channels
                .map(|c| {
                    if c == 1 {
                        "mono".to_string()
                    } else if c == 2 {
                        "stereo".to_string()
                    } else {
                        format!("{c} ch")
                    }
                })
                .unwrap_or_else(|| "unknown".into());
            ui.label(
                RichText::new(format!("{sr} · {ch} · resampled to 16 kHz mono"))
                    .small()
                    .color(SUBTLER),
            );
        }
    }

    fn selected_device_meta(&self) -> Option<&DeviceInfo> {
        match &self.persisted.mic_device {
            Some(name) => self.devices.iter().find(|d| &d.name == name),
            None => self.devices.iter().find(|d| d.is_default),
        }
    }

    fn system_audio_section(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("system audio").small().color(SUBTLE));
        ui.add_space(2.0);

        let recording = self.is_recording();
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!recording, |ui| {
                ui.checkbox(
                    &mut self.persisted.system_audio_enabled,
                    "capture system audio",
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new("ScreenCaptureKit — week 2")
                        .small()
                        .color(SUBTLER),
                );
            });
        });
        ui.label(
            RichText::new(
                "System audio capture is stubbed in this build. Mic recording is unaffected.",
            )
            .small()
            .color(SUBTLER),
        );
    }

    fn output_section(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("output directory").small().color(SUBTLE));
        ui.add_space(2.0);

        let recording = self.is_recording();
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!recording, |ui| {
                let path_str = self.persisted.output_dir.display().to_string();
                let display = if path_str.len() > 42 {
                    format!("…{}", &path_str[path_str.len() - 41..])
                } else {
                    path_str
                };
                let response = ui.add_sized(
                    [ui.available_width() - 88.0, 28.0],
                    egui::TextEdit::singleline(&mut display.clone()).desired_width(f32::INFINITY),
                );
                // Make it look readonly-ish.
                response.on_hover_text(self.persisted.output_dir.display().to_string());

                if ui
                    .add_sized([76.0, 28.0], egui::Button::new("browse"))
                    .clicked()
                {
                    self.pick_output_dir();
                }
            });
        });
    }

    fn action_section(&mut self, ui: &mut egui::Ui) {
        let recording = self.is_recording();

        ui.vertical_centered(|ui| {
            let label = if recording {
                RichText::new("◼  stop recording").size(16.0).color(STRONG)
            } else {
                RichText::new("●  start recording").size(16.0).color(STRONG)
            };
            let button = egui::Button::new(label)
                .min_size(Vec2::new(280.0, 44.0))
                .corner_radius(egui::CornerRadius::same(8))
                .fill(if recording {
                    Color32::from_rgb(60, 22, 22)
                } else {
                    Color32::from_rgb(34, 34, 34)
                });
            let response = ui.add(button);
            if response.clicked() {
                if recording {
                    self.stop_recording();
                } else {
                    self.start_recording();
                }
            }
        });

        if let Some(err) = &self.last_error {
            ui.add_space(8.0);
            ui.label(RichText::new(err).color(RECORD_RED).small());
        }
    }

    fn history_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("recent recordings").small().color(SUBTLE));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(egui::Button::new(RichText::new("refresh").small()))
                    .clicked()
                {
                    self.refresh_history();
                }
            });
        });
        ui.add_space(4.0);

        if self.history.is_empty() {
            ui.label(
                RichText::new("nothing here yet — your recordings will appear after you stop")
                    .small()
                    .color(SUBTLER),
            );
            return;
        }

        let frame = egui::Frame::default()
            .fill(Color32::from_rgb(18, 18, 18))
            .inner_margin(egui::Margin::same(2))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(30, 30, 30)))
            .corner_radius(egui::CornerRadius::same(6));
        frame.show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for (i, item) in self.history.clone().iter().enumerate() {
                        if i > 0 {
                            ui.separator();
                        }
                        history_row(ui, item);
                    }
                });
        });
    }

    fn separator(&self, ui: &mut egui::Ui) {
        let line_color = Color32::from_rgb(30, 30, 30);
        let rect = ui.available_rect_before_wrap();
        let y = rect.min.y;
        ui.painter().line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
            egui::Stroke::new(1.0, line_color),
        );
        ui.add_space(1.0);
    }

    fn footer(&self, ui: &mut egui::Ui) {
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            ui.label(
                RichText::new("audio stays on this mac")
                    .small()
                    .color(SUBTLER),
            );
        });
    }
}

fn history_row(ui: &mut egui::Ui, item: &RecordingSummary) {
    egui::Frame::default()
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&item.label).color(STRONG));
                    let mut parts: Vec<String> = Vec::new();
                    let dur = format_duration(item.duration_seconds.max(0));
                    parts.push(dur);
                    if let Some(b) = item.mic_bytes {
                        parts.push(format!("mic {}", human_bytes(b)));
                    }
                    if let Some(b) = item.system_bytes {
                        parts.push(format!("system {}", human_bytes(b)));
                    }
                    ui.label(RichText::new(parts.join("  ·  ")).small().color(SUBTLER));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(RichText::new("reveal").small()).clicked() {
                        let _ = reveal_in_finder(&item.session_dir);
                    }
                });
            });
        });
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

fn summarize(art: &CaptureArtifacts) -> RecordingSummary {
    let label = art
        .session_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "session".into());
    let duration_seconds = (art.stopped_at - art.started_at).num_seconds();
    let mic_bytes = art
        .mic_path
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok().map(|m| m.len()));
    let system_bytes = art
        .system_path
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok().map(|m| m.len()));
    RecordingSummary {
        session_dir: art.session_dir.clone(),
        label,
        duration_seconds,
        mic_bytes,
        system_bytes,
    }
}

fn scan_history(output_dir: &std::path::Path) -> Vec<RecordingSummary> {
    let Ok(entries) = std::fs::read_dir(output_dir) else {
        return Vec::new();
    };
    let mut sessions: Vec<RecordingSummary> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "session".into());
        let mic_path = path.join("mic.wav");
        let sys_path = path.join("system.wav");
        let mic_bytes = std::fs::metadata(&mic_path).ok().map(|m| m.len());
        let system_bytes = std::fs::metadata(&sys_path).ok().map(|m| m.len());
        if mic_bytes.is_none() && system_bytes.is_none() {
            continue;
        }
        let duration_seconds = mic_bytes
            .map(|b| ((b.saturating_sub(44)) / (16_000 * 2)) as i64)
            .unwrap_or(0);
        sessions.push(RecordingSummary {
            session_dir: path,
            label,
            duration_seconds,
            mic_bytes,
            system_bytes,
        });
    }
    sessions.sort_by(|a, b| b.label.cmp(&a.label));
    sessions.truncate(20);
    sessions
}

fn format_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        format!("{m}m {s:02}s")
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{h}h {m:02}m")
    }
}

fn human_bytes(b: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if b >= GB {
        format!("{:.1} GB", b as f64 / GB as f64)
    } else if b >= MB {
        format!("{:.1} MB", b as f64 / MB as f64)
    } else if b >= KB {
        format!("{:.1} kB", b as f64 / KB as f64)
    } else {
        format!("{b} B")
    }
}

#[cfg(target_os = "macos")]
fn reveal_in_finder(path: &std::path::Path) -> std::io::Result<()> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
}

#[cfg(not(target_os = "macos"))]
fn reveal_in_finder(_path: &std::path::Path) -> std::io::Result<()> {
    Ok(())
}

// Unused on this platform but keeping for parity with non-mac warnings.
#[allow(dead_code)]
const _: fn() = || {
    let _ = SUCCESS_GREEN;
};
