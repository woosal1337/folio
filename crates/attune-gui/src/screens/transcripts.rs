//! Transcripts screen. Lists every transcribed recording with its
//! generated text alongside. When the transcription pipeline is wired up,
//! finished transcripts land in [`Runtime::transcripts`] automatically.

use egui::{Align, Layout, RichText};

use crate::components::{caption, divider, empty_state, ghost_button_icon, mono, mono_small};
use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};
use crate::state::{Persisted, Runtime};
use crate::transcription::Transcript;

pub fn show(ui: &mut egui::Ui, _ctx: &egui::Context, rt: &mut Runtime, _persisted: &mut Persisted) {
    let p = palette::current();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Transcripts")
                .font(TextStyle::Title.font_id())
                .color(p.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Refresh, "refresh").clicked() {
                rt.transcripts.reload();
            }
        });
    });
    ui.add_space(Space::sm());
    ui.label(caption(
        "Searchable text generated from your recordings. Choose a provider in Settings → Transcription.",
    ));
    ui.add_space(Space::xl());

    if rt.transcripts.transcripts.is_empty() {
        empty_state(
            ui,
            Icon::Transcript,
            "No transcripts yet",
            "Open Library, expand a recording, and click Transcribe. Results appear here.",
        );
        return;
    }

    let transcripts = rt.transcripts.transcripts.clone();
    for (i, t) in transcripts.iter().enumerate() {
        transcript_card(ui, t, i + 1);
        ui.add_space(Space::md());
    }
}

fn transcript_card(ui: &mut egui::Ui, t: &Transcript, n: usize) {
    let p = palette::current();
    let frame = egui::Frame::default()
        .fill(p.surface_raised)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::lg() as u8))
        .inner_margin(egui::Margin::symmetric(
            Space::lg() as i8,
            Space::md() as i8,
        ));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("#{n:03}"))
                    .font(TextStyle::MonoSmall.font_id())
                    .color(p.text_subtle),
            );
            ui.add_space(Space::sm());
            ui.vertical(|ui| {
                ui.label(mono(t.recording_label.clone()).color(p.text));
                let line = format!(
                    "{} · {} · {} segments · {}",
                    t.provider.label(),
                    t.model,
                    t.segments.len(),
                    t.created_label(),
                );
                ui.label(mono_small(line));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = format!("{}s", t.duration_seconds);
                ui.label(
                    RichText::new(label)
                        .font(TextStyle::MonoSmall.font_id())
                        .color(p.text_subtle),
                );
            });
        });

        ui.add_space(Space::sm());
        divider(ui);
        ui.add_space(Space::sm());

        egui::ScrollArea::vertical()
            .id_salt(format!("transcript_{}", t.id))
            .max_height(220.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for seg in &t.segments {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format_ts(seg.start_ms))
                                .font(TextStyle::MonoSmall.font_id())
                                .color(p.text_subtle),
                        );
                        ui.add_space(Space::sm());
                        if let Some(spk) = seg.speaker.as_deref() {
                            ui.label(
                                RichText::new(format!("{spk}:"))
                                    .font(TextStyle::BodyStrong.font_id())
                                    .color(p.text_muted),
                            );
                        }
                        ui.label(
                            RichText::new(&seg.text)
                                .font(TextStyle::Body.font_id())
                                .color(p.text),
                        );
                    });
                    ui.add_space(Space::x2s());
                }
            });
    });
}

fn format_ts(ms: u64) -> String {
    let total = ms / 1000;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}
