//! Library screen. Full list of recorded sessions with inline playback and
//! a one-click transcribe button per recording.

use egui::{Align, Layout, RichText};

use crate::components::{
    audio_player, caption, divider, empty_state, ghost_button_icon, mono, mono_small,
};
use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};
use crate::playback::{PlayerTrack, TrackSource};
use crate::state::{
    format_bytes, format_duration, format_khz, refresh_history, reveal_in_finder, Persisted,
    RecordingSummary, Runtime,
};

pub fn show(ui: &mut egui::Ui, _ctx: &egui::Context, rt: &mut Runtime, persisted: &mut Persisted) {
    let p = palette::current();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Library")
                .font(TextStyle::Title.font_id())
                .color(p.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Refresh, "refresh").clicked() {
                refresh_history(rt, &persisted.output_dir);
                rt.transcripts.reload();
            }
        });
    });
    ui.add_space(Space::sm());
    ui.label(caption(
        "Every session lives on disk. Play either track inline, transcribe with a click.",
    ));
    ui.add_space(Space::xl());

    if rt.history.is_empty() {
        empty_state(
            ui,
            Icon::Library,
            "Library is empty",
            "Start your first recording from the Record screen and it will appear here.",
        );
        return;
    }

    let p_for_redraw = p;
    let _ = p_for_redraw;

    // Snapshot the player and history so we can mutate rt safely while iterating.
    let snap = rt.player.as_ref().map(|p| p.snapshot());
    let history = rt.history.clone();
    let expanded = rt.expanded_recording.clone();

    let mut to_expand: Option<Option<std::path::PathBuf>> = None;
    let mut player_action: Option<PlayerAction> = None;
    let mut transcribe: Option<RecordingSummary> = None;
    let mut reveal: Option<std::path::PathBuf> = None;

    for item in &history {
        recording_row(
            ui,
            item,
            expanded.as_deref() == Some(item.session_dir.as_path()),
            snap.as_ref(),
            &mut to_expand,
            &mut player_action,
            &mut transcribe,
            &mut reveal,
            rt.transcripts.for_session(&item.session_dir).is_some(),
        );
        ui.add_space(Space::sm());
    }

    if let Some(target) = to_expand {
        rt.expanded_recording = target;
    }

    if let Some(action) = player_action {
        if let Some(player) = rt.player.as_ref() {
            match action {
                PlayerAction::Play(track) => player.play(track),
                PlayerAction::Toggle => player.toggle_pause(),
                PlayerAction::Seek(f) => player.seek_fraction(f),
            }
        }
    }

    if let Some(path) = reveal {
        let _ = reveal_in_finder(&path);
    }

    if let Some(rec) = transcribe {
        rt.last_error = Some(format!(
            "Transcription wiring is in place but the {} provider isn't connected yet. \
             Open Settings → Transcription to choose a provider and paste a key.",
            persisted.transcriber.label()
        ));
        let _ = rec;
    }
}

enum PlayerAction {
    Play(PlayerTrack),
    Toggle,
    Seek(f32),
}

#[allow(clippy::too_many_arguments)]
fn recording_row(
    ui: &mut egui::Ui,
    item: &RecordingSummary,
    expanded: bool,
    snap: Option<&crate::playback::PlayerSnapshot>,
    to_expand: &mut Option<Option<std::path::PathBuf>>,
    player_action: &mut Option<PlayerAction>,
    transcribe: &mut Option<RecordingSummary>,
    reveal: &mut Option<std::path::PathBuf>,
    has_transcript: bool,
) {
    let p = palette::current();

    let frame_fill = if expanded {
        p.surface_raised
    } else {
        p.surface_subtle
    };
    let frame = egui::Frame::default()
        .fill(frame_fill)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::lg() as u8))
        .inner_margin(egui::Margin::symmetric(
            Space::md() as i8,
            Space::md() as i8,
        ));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Icon::FileAudio.glyph())
                    .font(TextStyle::Body.font_id())
                    .color(p.text_subtle),
            );
            ui.add_space(Space::sm());
            ui.vertical(|ui| {
                ui.label(mono(item.label.clone()).color(p.text));

                let mut parts: Vec<String> = vec![format_duration(item.duration_seconds)];
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
                if has_transcript {
                    ui.label(
                        RichText::new("transcript")
                            .font(TextStyle::MonoSmall.font_id())
                            .color(p.success),
                    );
                    ui.add_space(Space::xs());
                }

                let chevron = if expanded { Icon::X } else { Icon::Play };
                if ghost_button_icon(ui, chevron, if expanded { "close" } else { "open" }).clicked()
                {
                    *to_expand = Some(if expanded {
                        None
                    } else {
                        Some(item.session_dir.clone())
                    });
                }
            });
        });

        if !expanded {
            return;
        }

        ui.add_space(Space::md());
        divider(ui);
        ui.add_space(Space::md());

        // Audio players (mic + system if present)
        let mic_path = item
            .mic_bytes
            .map(|_| item.session_dir.join("mic.wav"))
            .filter(|p| p.exists());
        let sys_path = item
            .system_bytes
            .map(|_| item.session_dir.join("system.wav"))
            .filter(|p| p.exists());

        let blank_snapshot = crate::playback::PlayerSnapshot {
            track: None,
            position: std::time::Duration::ZERO,
            duration: None,
            paused: false,
            finished: true,
        };

        if let Some(path) = &mic_path {
            ui.horizontal(|ui| {
                source_badge(ui, "mic", p);
            });
            ui.add_space(Space::x2s());
            let snapshot = snap.cloned().unwrap_or_else(|| blank_snapshot.clone());
            let action = audio_player(ui, path, &snapshot);
            handle_player_action(
                action,
                path,
                item.session_dir.clone(),
                TrackSource::Mic,
                &snapshot,
                player_action,
            );
            ui.add_space(Space::md());
        }

        if let Some(path) = &sys_path {
            ui.horizontal(|ui| {
                source_badge(ui, "system", p);
            });
            ui.add_space(Space::x2s());
            let snapshot = snap.cloned().unwrap_or_else(|| blank_snapshot.clone());
            let action = audio_player(ui, path, &snapshot);
            handle_player_action(
                action,
                path,
                item.session_dir.clone(),
                TrackSource::System,
                &snapshot,
                player_action,
            );
            ui.add_space(Space::md());
        }

        ui.add_space(Space::sm());
        divider(ui);
        ui.add_space(Space::md());

        ui.horizontal(|ui| {
            if ghost_button_icon(ui, Icon::Sparkle, "transcribe").clicked() {
                *transcribe = Some(item.clone());
            }
            if ghost_button_icon(ui, Icon::Reveal, "reveal in finder").clicked() {
                *reveal = Some(item.session_dir.clone());
            }
        });
    });
}

fn handle_player_action(
    action: crate::components::AudioPlayerAction,
    file: &std::path::Path,
    session_dir: std::path::PathBuf,
    source: TrackSource,
    snapshot: &crate::playback::PlayerSnapshot,
    out: &mut Option<PlayerAction>,
) {
    if let Some(f) = action.seek_fraction {
        if snapshot
            .track
            .as_ref()
            .map(|t| t.file == file)
            .unwrap_or(false)
        {
            *out = Some(PlayerAction::Seek(f));
            return;
        }
        // Not the active track yet: load + then seek? Easiest: start playing, ignore seek.
        let _ = f;
    }
    if action.toggle {
        let is_current = snapshot
            .track
            .as_ref()
            .map(|t| t.file == file && !snapshot.finished)
            .unwrap_or(false);
        if is_current {
            *out = Some(PlayerAction::Toggle);
        } else {
            *out = Some(PlayerAction::Play(PlayerTrack {
                session_dir,
                file: file.to_path_buf(),
                source,
            }));
        }
    }
}

fn source_badge(ui: &mut egui::Ui, label: &str, p: crate::design::Palette) {
    let frame = egui::Frame::default()
        .fill(p.surface_overlay)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::pill() as u8))
        .inner_margin(egui::Margin::symmetric(Space::sm() as i8, 1));
    frame.show(ui, |ui| {
        ui.label(
            RichText::new(label)
                .font(TextStyle::MonoSmall.font_id())
                .color(p.text_muted),
        );
    });
}

// kept for callers passing in a public function
#[allow(dead_code)]
pub fn recordings_list(ui: &mut egui::Ui, history: &[RecordingSummary]) {
    // Backwards-compat shim: render a non-interactive list (no player) of
    // recordings. Used by the empty-state fallback when desired.
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
                        divider(ui);
                    }
                    egui::Frame::default()
                        .inner_margin(egui::Margin::symmetric(
                            Space::md() as i8,
                            Space::sm() as i8 + 2,
                        ))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(Icon::FileAudio.glyph())
                                        .font(TextStyle::Body.font_id())
                                        .color(p.text_subtle),
                                );
                                ui.add_space(Space::sm());
                                ui.vertical(|ui| {
                                    ui.label(mono(item.label.clone()));
                                    ui.label(mono_small(format_duration(item.duration_seconds)));
                                });
                            });
                        });
                }
            });
    });
}
