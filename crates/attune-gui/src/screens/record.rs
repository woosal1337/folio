//! Record screen. Device picker, system audio toggle, output directory,
//! main capture toggle, recent recordings.

use egui::{Align, Layout, RichText};

use crate::components::{
    body_strong, caption, card, divider, empty_state, ghost_button_icon, labeled_section, micro,
    mono, mono_small, record_button, status_pill,
};
use crate::design::tokens::{Layout as L, Radius, Space};
use crate::design::{palette, Icon, TextStyle};
use crate::state::{
    format_bytes, format_duration, format_khz, refresh_devices, refresh_history, reveal_in_finder,
    start_recording, stop_recording, Persisted, RecordingSummary, Runtime,
};

pub fn show(ui: &mut egui::Ui, ctx: &egui::Context, rt: &mut Runtime, persisted: &mut Persisted) {
    let p = palette::current();

    // Page header
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Record")
                .font(TextStyle::Title.font_id())
                .color(p.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            status_pill(ui, ctx, rt.is_recording(), &rt.elapsed_label());
        });
    });
    ui.add_space(Space::sm());
    ui.label(caption(
        "Capture system audio and microphone independently. Audio stays on this Mac.",
    ));
    ui.add_space(Space::xl());

    // Controls card
    card(ui, |ui| {
        device_section(ui, rt, persisted);
        ui.add_space(Space::md());
        divider(ui);
        ui.add_space(Space::md());
        system_section(ui, rt, persisted);
        ui.add_space(Space::md());
        divider(ui);
        ui.add_space(Space::md());
        output_section(ui, rt, persisted);
    });

    ui.add_space(Space::xl());

    // Big record button
    let response = record_button(ui, rt.is_recording());
    if response.clicked() {
        if rt.is_recording() {
            stop_recording(rt);
        } else {
            start_recording(rt, persisted);
        }
        refresh_history(rt, &persisted.output_dir);
    }

    if let Some(err) = rt.last_error.as_deref() {
        ui.add_space(Space::sm());
        ui.label(
            RichText::new(err)
                .font(TextStyle::Caption.font_id())
                .color(p.danger),
        );
    }

    ui.add_space(Space::xl());

    // Recent recordings
    ui.horizontal(|ui| {
        ui.label(body_strong("Recent recordings").color(p.text));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Refresh, "refresh").clicked() {
                refresh_history(rt, &persisted.output_dir);
            }
        });
    });
    ui.add_space(Space::sm());

    if rt.history.is_empty() {
        empty_state(
            ui,
            Icon::FileAudio,
            "No recordings yet",
            "Recordings will appear here after you stop a session.",
        );
    } else {
        recordings_list(ui, &rt.history);
    }

    let _ = L::header_height();
}

fn device_section(ui: &mut egui::Ui, rt: &mut Runtime, persisted: &mut Persisted) {
    let p = palette::current();
    let recording = rt.is_recording();

    labeled_section(ui, Some(Icon::Microphone), "INPUT DEVICE", |ui| {
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!recording, |ui| {
                let selected_label = match &persisted.mic_device {
                    Some(name) => name.clone(),
                    None => rt
                        .devices
                        .iter()
                        .find(|d| d.is_default)
                        .map(|d| format!("{}  ·  default", d.name))
                        .unwrap_or_else(|| "(system default)".into()),
                };
                let mut combo = egui::ComboBox::from_id_salt("mic_device")
                    .selected_text(selected_label)
                    .width(ui.available_width() - 90.0);
                combo = combo.height(260.0);
                combo.show_ui(ui, |ui| {
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
            });
            ui.add_enabled_ui(!recording, |ui| {
                if ghost_button_icon(ui, Icon::Refresh, "refresh").clicked() {
                    refresh_devices(rt, persisted);
                }
            });
        });

        if let Some(d) = selected_device_meta(rt, persisted) {
            let sr = d
                .default_sample_rate
                .map(format_khz)
                .unwrap_or_else(|| "unknown".into());
            let ch = d
                .default_channels
                .map(|c| match c {
                    1 => "mono".to_string(),
                    2 => "stereo".to_string(),
                    n => format!("{n} ch"),
                })
                .unwrap_or_else(|| "?".into());
            ui.add_space(Space::xs());
            ui.label(micro(format!("{sr} · {ch} · saved at native rate")).color(p.text_subtle));
        }
    });
}

fn selected_device_meta<'a>(
    rt: &'a Runtime,
    persisted: &Persisted,
) -> Option<&'a attune_core::audio::DeviceInfo> {
    match &persisted.mic_device {
        Some(name) => rt.devices.iter().find(|d| &d.name == name),
        None => rt.devices.iter().find(|d| d.is_default),
    }
}

fn system_section(ui: &mut egui::Ui, rt: &Runtime, persisted: &mut Persisted) {
    let recording = rt.is_recording();
    labeled_section(ui, Some(Icon::SpeakerSimple), "SYSTEM AUDIO", |ui| {
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!recording, |ui| {
                ui.checkbox(&mut persisted.system_audio_enabled, "Capture system audio");
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(micro("ScreenCaptureKit · audio only"));
            });
        });
        ui.add_space(Space::x2s());
        ui.label(micro(
            "macOS prompts for Screen Recording permission the first time. \
                 No video is captured.",
        ));
    });
}

fn output_section(ui: &mut egui::Ui, rt: &Runtime, persisted: &mut Persisted) {
    let recording = rt.is_recording();
    labeled_section(ui, Some(Icon::Folder), "OUTPUT FOLDER", |ui| {
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!recording, |ui| {
                let raw = persisted.output_dir.display().to_string();
                let display = if raw.len() > 56 {
                    format!("…{}", &raw[raw.len() - 55..])
                } else {
                    raw.clone()
                };
                let label = mono(display).color(palette::current().text_muted);
                ui.add_sized(
                    [ui.available_width() - 90.0, 28.0],
                    egui::Label::new(label).truncate(),
                )
                .on_hover_text(raw);

                if ghost_button_icon(ui, Icon::FolderOpen, "browse").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        persisted.output_dir = dir;
                    }
                }
            });
        });
        let _ = rt;
    });
}

pub fn recordings_list(ui: &mut egui::Ui, history: &[RecordingSummary]) {
    let p = palette::current();
    let frame = egui::Frame::default()
        .fill(p.surface)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::lg() as u8));
    frame.show(ui, |ui| {
        egui::ScrollArea::vertical()
            .max_height(260.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (i, item) in history.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(0.5);
                        divider(ui);
                    }
                    history_row(ui, item);
                }
            });
    });
}

fn history_row(ui: &mut egui::Ui, item: &RecordingSummary) {
    let p = palette::current();
    egui::Frame::default()
        .inner_margin(egui::Margin::symmetric(
            Space::md() as i8,
            Space::sm() as i8 + 2,
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Left icon column.
                ui.label(
                    RichText::new(Icon::FileAudio.glyph())
                        .font(TextStyle::Body.font_id())
                        .color(p.text_subtle),
                );
                ui.add_space(Space::sm());

                ui.vertical(|ui| {
                    ui.label(mono(item.label.clone()));
                    let mut parts = vec![format_duration(item.duration_seconds)];
                    if let Some(b) = item.mic_bytes {
                        let sr = item
                            .mic_sample_rate
                            .map(|s| format!(" {}", format_khz(s)))
                            .unwrap_or_default();
                        parts.push(format!("mic {}{}", format_bytes(b), sr));
                    }
                    if let Some(b) = item.system_bytes {
                        let sr = item
                            .system_sample_rate
                            .map(|s| format!(" {}", format_khz(s)))
                            .unwrap_or_default();
                        parts.push(format!("system {}{}", format_bytes(b), sr));
                    }
                    ui.label(mono_small(parts.join("    ")));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ghost_button_icon(ui, Icon::Reveal, "reveal").clicked() {
                        let _ = reveal_in_finder(&item.session_dir);
                    }
                });
            });
        });
}
