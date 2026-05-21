//! Settings screen. Output folder, notes folder, transcription provider +
//! API key, future model selection.

use egui::{Align, Layout, RichText};

use crate::components::{caption, card, ghost_button_icon, labeled_section, mono};
use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle, Theme};
use crate::state::{Persisted, Runtime};
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
        "Storage paths and provider keys. Everything stays on this Mac.",
    ));
    ui.add_space(Space::xl());

    // Appearance card
    card(ui, |ui| {
        labeled_section(ui, Some(Icon::Sparkle), "APPEARANCE", |ui| {
            ui.horizontal(|ui| {
                for theme in Theme::all() {
                    let selected = persisted.theme == *theme;
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
                            egui::RichText::new(theme.label())
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
                        persisted.theme = *theme;
                        crate::design::set_theme_and_apply(ctx, *theme);
                    }
                    ui.add_space(Space::xs());
                }
            });
            ui.add_space(Space::x2s());
            ui.label(caption(
                "Light is the default. Dark stays available for late-night work.",
            ));
        });
    });

    ui.add_space(Space::lg());

    card(ui, |ui| {
        folder_setting(ui, "RECORDINGS FOLDER", &mut persisted.output_dir);
        ui.add_space(Space::md());
        folder_setting(ui, "NOTES FOLDER", &mut persisted.notes_dir);
        ui.add_space(Space::md());
        folder_setting(ui, "TRANSCRIPTS FOLDER", &mut persisted.transcripts_dir);
    });

    ui.add_space(Space::lg());

    card(ui, |ui| {
        labeled_section(ui, Some(Icon::Sparkle), "TRANSCRIPTION PROVIDER", |ui| {
            ui.horizontal(|ui| {
                for kind in TranscriberKind::all() {
                    let selected = persisted.transcriber == *kind;
                    let bg = if selected {
                        p.surface_overlay
                    } else {
                        p.surface_subtle
                    };
                    let resp = ui.add(
                        egui::Button::new(
                            RichText::new(kind.label())
                                .font(TextStyle::Body.font_id())
                                .color(if selected { p.text } else { p.text_muted }),
                        )
                        .corner_radius(egui::CornerRadius::same(Radius::md() as u8))
                        .fill(bg)
                        .stroke(egui::Stroke::new(1.0, p.border)),
                    );
                    if resp.clicked() {
                        persisted.transcriber = *kind;
                    }
                    ui.add_space(Space::xs());
                }
            });

            ui.add_space(Space::xs());
            match persisted.transcriber {
                TranscriberKind::LocalWhisper => {
                    ui.label(caption(
                        "Runs whisper.cpp with Metal acceleration on this Mac. \
                         Wiring lands next session — a model download (~1.5 GB) \
                         happens on the first transcription.",
                    ));
                }
                TranscriberKind::OpenAi => {
                    ui.label(caption(
                        "Uploads audio to OpenAI's transcription API. Charged at \
                         $0.006/minute against your account. Wiring lands next \
                         session once you've pasted a key below.",
                    ));
                }
            }
        });

        if persisted.transcriber == TranscriberKind::OpenAi {
            ui.add_space(Space::md());
            labeled_section(ui, Some(Icon::Cmd), "OPENAI API KEY", |ui| {
                let resp = ui.add_sized(
                    [ui.available_width(), 28.0],
                    egui::TextEdit::singleline(&mut persisted.openai_api_key)
                        .password(true)
                        .hint_text("sk-...")
                        .font(TextStyle::Mono.font_id()),
                );
                let _ = resp;
                ui.add_space(Space::x2s());
                ui.label(
                    RichText::new(
                        "Stored locally in your app preferences. Never logged or transmitted except to OpenAI.",
                    )
                    .font(TextStyle::Micro.font_id())
                    .color(p.text_subtle),
                );
            });
        }
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

fn folder_setting(ui: &mut egui::Ui, label: &str, path: &mut std::path::PathBuf) {
    let p = palette::current();
    labeled_section(ui, Some(Icon::Folder), label, |ui| {
        ui.horizontal(|ui| {
            let raw = path.display().to_string();
            let display = if raw.len() > 56 {
                format!("…{}", &raw[raw.len() - 55..])
            } else {
                raw.clone()
            };
            ui.add_sized(
                [ui.available_width() - 100.0, 28.0],
                egui::Label::new(mono(display).color(p.text_muted)).truncate(),
            )
            .on_hover_text(raw);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ghost_button_icon(ui, Icon::FolderOpen, "change").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        *path = dir;
                    }
                }
            });
        });
    });
}
