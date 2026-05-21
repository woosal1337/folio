//! Settings screen.
//!
//! Vertical list of cards. Every card fills the available width so right
//! edges line up. Form fields use the standardised `text_input` /
//! `password_input` / `mono_input` components so heights and paddings stay
//! consistent across the app.

use egui::{Align, Layout, RichText};

use crate::components::{
    caption, card, ghost_button_icon, labeled_section, mono_input, password_input,
};
use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle, Theme};
use crate::state::{refresh_devices, Persisted, Runtime};
use crate::transcription::TranscriberKind;

pub fn show(ui: &mut egui::Ui, ctx: &egui::Context, rt: &mut Runtime, persisted: &mut Persisted) {
    let p = palette::current();
    ui.label(
        RichText::new("Settings")
            .font(TextStyle::Title.font_id())
            .color(p.text),
    );
    ui.add_space(Space::sm());
    ui.label(caption(
        "Recording, transcription, storage, appearance. Everything stays on this Mac.",
    ));
    ui.add_space(Space::xl());

    // 1. RECORDING — what we capture
    card(ui, |ui| {
        labeled_section(ui, Some(Icon::Microphone), "RECORDING", |ui| {
            mic_picker(ui, rt, persisted);
            ui.add_space(Space::md());
            ui.checkbox(
                &mut persisted.system_audio_enabled,
                "Capture system audio (ScreenCaptureKit)",
            );
            ui.add_space(Space::x2s());
            ui.label(
                RichText::new(
                    "macOS prompts for Screen Recording permission the first time. No video is captured.",
                )
                .font(TextStyle::Micro.font_id())
                .color(p.text_subtle),
            );
        });
    });

    ui.add_space(Space::lg());

    // 2. TRANSCRIPTION
    card(ui, |ui| {
        labeled_section(ui, Some(Icon::Sparkle), "TRANSCRIPTION", |ui| {
            // Provider picker
            ui.horizontal(|ui| {
                for kind in TranscriberKind::all() {
                    provider_pill(ui, persisted, *kind);
                    ui.add_space(Space::xs());
                }
            });
            ui.add_space(Space::x2s());
            ui.label(match persisted.transcriber {
                TranscriberKind::OpenAi => caption(
                    "Audio is uploaded to OpenAI's Whisper API. ~$0.006/min against your account. Multilingual.",
                ),
                TranscriberKind::LocalWhisper => caption(
                    "Runs whisper.cpp on this Mac. Lands in a future session — switch to OpenAI for now.",
                ),
            });

            if persisted.transcriber == TranscriberKind::OpenAi {
                ui.add_space(Space::md());
                ui.label(
                    RichText::new("OPENAI API KEY")
                        .font(TextStyle::CapsLabel.font_id())
                        .color(p.text_muted),
                );
                ui.add_space(Space::x2s());
                password_input(ui, &mut persisted.openai_api_key, "sk-...");
                ui.add_space(Space::x2s());
                ui.label(
                    RichText::new(
                        "Stored in your local app preferences. Sent only to api.openai.com.",
                    )
                    .font(TextStyle::Micro.font_id())
                    .color(p.text_subtle),
                );
            }

            ui.add_space(Space::md());
            ui.label(
                RichText::new("LANGUAGE")
                    .font(TextStyle::CapsLabel.font_id())
                    .color(p.text_muted),
            );
            ui.add_space(Space::x2s());
            language_picker(ui, &mut persisted.transcription_language);
            ui.add_space(Space::x2s());
            ui.label(
                RichText::new(
                    "\"Auto\" lets Whisper detect each segment. Pick a language if you record predominantly in one.",
                )
                .font(TextStyle::Micro.font_id())
                .color(p.text_subtle),
            );
        });
    });

    ui.add_space(Space::lg());

    // 3. STORAGE
    card(ui, |ui| {
        labeled_section(ui, Some(Icon::Folder), "STORAGE", |ui| {
            folder_setting(ui, "Recordings", &mut persisted.output_dir);
            ui.add_space(Space::sm());
            folder_setting(ui, "Notes", &mut persisted.notes_dir);
            ui.add_space(Space::sm());
            folder_setting(ui, "Transcripts", &mut persisted.transcripts_dir);
        });
    });

    ui.add_space(Space::lg());

    // 4. APPEARANCE
    card(ui, |ui| {
        labeled_section(ui, Some(Icon::Sparkle), "APPEARANCE", |ui| {
            ui.horizontal(|ui| {
                for theme in Theme::all() {
                    theme_pill(ui, ctx, persisted, *theme);
                    ui.add_space(Space::xs());
                }
            });
            ui.add_space(Space::x2s());
            ui.label(caption(
                "Light is the default. Dark stays available for late-night work.",
            ));
        });
    });

    if let Some(err) = rt.last_error.as_deref() {
        ui.add_space(Space::md());
        ui.label(
            RichText::new(err)
                .font(TextStyle::Caption.font_id())
                .color(p.warning),
        );
    }
}

// ---------------------------------------------------------------------------
// Sub-widgets
// ---------------------------------------------------------------------------

fn mic_picker(ui: &mut egui::Ui, rt: &mut Runtime, persisted: &mut Persisted) {
    let p = palette::current();
    ui.label(
        RichText::new("INPUT DEVICE")
            .font(TextStyle::CapsLabel.font_id())
            .color(p.text_muted),
    );
    ui.add_space(Space::x2s());

    ui.horizontal(|ui| {
        let selected_label = match &persisted.mic_device {
            Some(name) => name.clone(),
            None => rt
                .devices
                .iter()
                .find(|d| d.is_default)
                .map(|d| format!("{}  ·  default", d.name))
                .unwrap_or_else(|| "(system default)".into()),
        };
        egui::ComboBox::from_id_salt("settings_mic_device")
            .selected_text(selected_label)
            .width(ui.available_width() - 96.0)
            .height(260.0)
            .show_ui(ui, |ui| {
                let default_selected = persisted.mic_device.is_none();
                if ui
                    .selectable_label(default_selected, "(system default)")
                    .clicked()
                {
                    persisted.mic_device = None;
                }
                ui.separator();
                for d in &rt.devices {
                    let selected = persisted.mic_device.as_deref() == Some(&d.name);
                    let label = if d.is_default {
                        format!("{}  ·  default", d.name)
                    } else {
                        d.name.clone()
                    };
                    if ui.selectable_label(selected, label).clicked() {
                        persisted.mic_device = Some(d.name.clone());
                    }
                }
            });
        if ghost_button_icon(ui, Icon::Refresh, "refresh").clicked() {
            refresh_devices(rt, persisted);
        }
    });
}

fn provider_pill(ui: &mut egui::Ui, persisted: &mut Persisted, kind: TranscriberKind) {
    let p = palette::current();
    let selected = persisted.transcriber == kind;
    let bg = if selected {
        p.accent_subtle
    } else {
        p.surface_subtle
    };
    let text_color = if selected {
        p.accent_strong
    } else {
        p.text_muted
    };
    let resp = ui.add(
        egui::Button::new(
            RichText::new(kind.label())
                .font(TextStyle::Body.font_id())
                .color(text_color),
        )
        .corner_radius(egui::CornerRadius::same(Radius::md() as u8))
        .fill(bg)
        .stroke(egui::Stroke::new(
            1.0,
            if selected { p.accent } else { p.border },
        ))
        .min_size(egui::vec2(150.0, 32.0)),
    );
    if resp.clicked() && !selected {
        persisted.transcriber = kind;
    }
}

fn theme_pill(ui: &mut egui::Ui, ctx: &egui::Context, persisted: &mut Persisted, theme: Theme) {
    let p = palette::current();
    let selected = persisted.theme == theme;
    let bg = if selected {
        p.accent_subtle
    } else {
        p.surface_subtle
    };
    let text_color = if selected {
        p.accent_strong
    } else {
        p.text_muted
    };
    let resp = ui.add(
        egui::Button::new(
            RichText::new(theme.label())
                .font(TextStyle::Body.font_id())
                .color(text_color),
        )
        .corner_radius(egui::CornerRadius::same(Radius::md() as u8))
        .fill(bg)
        .stroke(egui::Stroke::new(
            1.0,
            if selected { p.accent } else { p.border },
        ))
        .min_size(egui::vec2(96.0, 32.0)),
    );
    if resp.clicked() && !selected {
        persisted.theme = theme;
        crate::design::set_theme_and_apply(ctx, theme);
    }
}

fn language_picker(ui: &mut egui::Ui, language: &mut String) {
    let options: &[(&str, &str)] = &[
        ("auto", "Auto-detect"),
        ("en", "English"),
        ("tr", "Turkish"),
        ("az", "Azerbaijani"),
        ("ru", "Russian"),
        ("de", "German"),
        ("es", "Spanish"),
        ("fr", "French"),
        ("it", "Italian"),
        ("pt", "Portuguese"),
        ("ar", "Arabic"),
        ("ja", "Japanese"),
        ("zh", "Chinese"),
    ];
    let selected_label = options
        .iter()
        .find(|(code, _)| code == language)
        .map(|(_, label)| label.to_string())
        .unwrap_or_else(|| "Auto-detect".into());

    egui::ComboBox::from_id_salt("settings_language")
        .selected_text(selected_label)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for (code, label) in options {
                let selected = language == code;
                if ui.selectable_label(selected, *label).clicked() {
                    *language = code.to_string();
                }
            }
        });
}

fn folder_setting(ui: &mut egui::Ui, label: &str, path: &mut std::path::PathBuf) {
    let p = palette::current();
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(120.0, 36.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.label(
                    RichText::new(label)
                        .font(TextStyle::Body.font_id())
                        .color(p.text),
                );
            },
        );
        ui.add_space(Space::sm());
        let mut path_str = path.display().to_string();
        let raw = path_str.clone();
        let resp = mono_input(ui, &mut path_str, "(path)");
        // The mono_input owns the path string by reference, but folder paths
        // are picked through the dialog rather than typed. Restore the path
        // if the user edited it inadvertently.
        if resp.changed() {
            *path = path_str.into();
        }
        let _ = raw;
    });
    ui.add_space(Space::x2s());
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(path.display().to_string())
                .font(TextStyle::Micro.font_id())
                .color(p.text_subtle),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::FolderOpen, "change").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    *path = dir;
                }
            }
        });
    });
}
