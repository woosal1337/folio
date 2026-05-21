//! Record screen.
//!
//! Stripped-down capture surface. The big "Start recording" CTA sits at the
//! top with the live status pill on the right. Every recording you make is
//! listed underneath. Device, system-audio toggle, and output folder live in
//! Settings → Recording so this screen stays focused on the act of
//! recording rather than configuring it.

use egui::{Align, Layout, RichText, Sense, Vec2};

use crate::components::{
    caption, empty_state, ghost_button_icon, mono, mono_small, record_button, status_pill,
};
use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};
use crate::state::{
    format_bytes, format_duration, format_khz, refresh_history, start_recording, stop_recording,
    Persisted, RecordingSummary, Runtime, Screen,
};

pub fn show(ui: &mut egui::Ui, ctx: &egui::Context, rt: &mut Runtime, persisted: &mut Persisted) {
    let p = palette::current();

    // Header row
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

    // CTA
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

    // Tiny meta strip pointing at Settings.
    ui.horizontal(|ui| {
        let mic_name = persisted
            .mic_device
            .as_deref()
            .unwrap_or("system default mic");
        let sys_state = if persisted.system_audio_enabled {
            "system audio on"
        } else {
            "system audio off"
        };
        ui.label(
            RichText::new(format!("{}  ·  {}", mic_name, sys_state))
                .font(TextStyle::Caption.font_id())
                .color(p.text_subtle),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Settings, "recording settings").clicked() {
                persisted.active_screen = Screen::Settings;
            }
        });
    });
    ui.add_space(Space::xl());

    // Recordings list
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Recent recordings")
                .font(TextStyle::Heading.font_id())
                .color(p.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Library, "open library").clicked() {
                persisted.active_screen = Screen::Library;
            }
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
            "Start a recording to see it land here.",
        );
        return;
    }

    let history = rt.history.clone();
    let mut open_in_library: Option<std::path::PathBuf> = None;
    for item in history.iter().take(6) {
        if recent_row(ui, item).clicked() {
            open_in_library = Some(item.session_dir.clone());
        }
        ui.add_space(Space::sm());
    }
    if history.len() > 6 {
        ui.add_space(Space::xs());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = format!("View all {}", history.len());
            if ghost_button_icon(ui, Icon::Library, label).clicked() {
                persisted.active_screen = Screen::Library;
            }
        });
    }

    if let Some(path) = open_in_library {
        rt.expanded_recording = Some(path);
        persisted.active_screen = Screen::Library;
    }
}

fn recent_row(ui: &mut egui::Ui, item: &RecordingSummary) -> egui::Response {
    let p = palette::current();
    let avail_w = ui.available_width();
    let height = 56.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(avail_w, height), Sense::click());

    let bg = if response.hovered() {
        p.surface_subtle
    } else {
        p.surface
    };
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(Radius::md() as u8), bg);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(Radius::md() as u8),
        egui::Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );

    let pad_x = Space::md();
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(pad_x, Space::sm())))
            .layout(Layout::left_to_right(Align::Center)),
    );

    child.label(
        RichText::new(Icon::FileAudio.glyph())
            .font(TextStyle::Body.font_id())
            .color(p.text_subtle),
    );
    child.add_space(Space::sm());
    child.vertical(|ui| {
        ui.label(mono(item.label.clone()).color(p.text));
        let mut parts = vec![format_duration(item.duration_seconds)];
        let mut total: u64 = 0;
        if let Some(b) = item.mic_bytes {
            total += b;
        }
        if let Some(b) = item.system_bytes {
            total += b;
        }
        if total > 0 {
            parts.push(format_bytes(total));
        }
        if let Some(sr) = item.mic_sample_rate.or(item.system_sample_rate) {
            parts.push(format_khz(sr));
        }
        ui.label(mono_small(parts.join("    ")));
    });
    child.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.label(
            RichText::new("open")
                .font(TextStyle::Caption.font_id())
                .color(p.text_subtle),
        );
    });

    response
}
