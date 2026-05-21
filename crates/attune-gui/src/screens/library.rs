//! Library — the one place where a recording lives with its transcript.
//!
//! Each row is collapsible. Expanded, you can play either audio track, kick
//! off a transcription, watch the progress live, and read the transcript
//! once it's saved.

use std::path::PathBuf;
use std::time::Duration;

use egui::{Align, Color32, Layout, RichText, Vec2};

use crate::components::{
    audio_player, caption, divider, empty_state, ghost_button_icon, mono, mono_small,
};
use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};
use crate::playback::{PlayerSnapshot, PlayerTrack, TrackSource};
use crate::state::{
    format_bytes, format_duration, format_khz, poll_transcription_jobs, refresh_history,
    reveal_in_finder, start_transcription, Persisted, RecordingSummary, Runtime,
};
use crate::transcription::{Transcript, TranscriptSegment};

pub fn show(ui: &mut egui::Ui, _ctx: &egui::Context, rt: &mut Runtime, persisted: &mut Persisted) {
    let p = palette::current();
    poll_transcription_jobs(rt);

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
        "Every recording lives here with its transcript. Click a row to expand, play, or transcribe.",
    ));
    ui.add_space(Space::xl());

    if rt.history.is_empty() {
        empty_state(
            ui,
            Icon::Library,
            "Library is empty",
            "Start your first recording from the Record screen.",
        );
        return;
    }

    let snap = rt.player.as_ref().map(|p| p.snapshot());
    let history = rt.history.clone();
    let expanded = rt.expanded_recording.clone();
    let transcripts = rt.transcripts.transcripts.clone();
    let jobs: std::collections::HashMap<PathBuf, String> = rt
        .jobs
        .iter()
        .map(|(k, v)| (k.clone(), v.status.clone()))
        .collect();

    let mut new_expansion: Option<Option<PathBuf>> = None;
    let mut player_action: Option<PlayerAction> = None;
    let mut transcribe_target: Option<RecordingSummary> = None;
    let mut reveal: Option<PathBuf> = None;

    for item in &history {
        let is_open = expanded.as_deref() == Some(item.session_dir.as_path());
        let transcript = transcripts
            .iter()
            .find(|t| t.session_dir == item.session_dir);
        let job_status = jobs.get(&item.session_dir).cloned();

        recording_row(
            ui,
            item,
            is_open,
            transcript,
            job_status,
            snap.as_ref(),
            &mut new_expansion,
            &mut player_action,
            &mut transcribe_target,
            &mut reveal,
        );
        ui.add_space(Space::sm());
    }

    if let Some(target) = new_expansion {
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
    if let Some(item) = transcribe_target {
        start_transcription(rt, persisted, &item);
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
    transcript: Option<&Transcript>,
    job_status: Option<String>,
    snap: Option<&PlayerSnapshot>,
    expansion: &mut Option<Option<PathBuf>>,
    player_action: &mut Option<PlayerAction>,
    transcribe: &mut Option<RecordingSummary>,
    reveal: &mut Option<PathBuf>,
) {
    let p = palette::current();
    let avail_w = ui.available_width();

    let frame = egui::Frame::default()
        .fill(p.surface_raised)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::lg() as u8))
        .inner_margin(egui::Margin::symmetric(
            Space::lg() as i8,
            Space::md() as i8,
        ));
    frame.show(ui, |ui| {
        let inner_w = (avail_w - Space::lg() * 2.0).max(0.0);
        ui.set_min_width(inner_w);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Icon::FileAudio.glyph())
                    .font(TextStyle::Body.font_id())
                    .color(p.text_subtle),
            );
            ui.add_space(Space::sm());
            ui.vertical(|ui| {
                ui.label(mono(item.label.clone()).color(p.text));
                ui.label(mono_small(metadata_string(item)));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Status chip
                if let Some(status) = &job_status {
                    chip(
                        ui,
                        format!("transcribing · {}", status),
                        p.accent_subtle,
                        p.accent_strong,
                    );
                } else if transcript.is_some() {
                    chip(
                        ui,
                        "transcribed".to_string(),
                        p.accent_subtle,
                        p.accent_strong,
                    );
                } else {
                    chip(
                        ui,
                        "no transcript".to_string(),
                        p.surface_subtle,
                        p.text_subtle,
                    );
                }
                ui.add_space(Space::xs());

                let label = if expanded { "close" } else { "open" };
                let icon = if expanded { Icon::X } else { Icon::Play };
                if ghost_button_icon(ui, icon, label).clicked() {
                    *expansion = Some(if expanded {
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

        // Audio players
        let mic_path = item
            .mic_bytes
            .map(|_| item.session_dir.join("mic.wav"))
            .filter(|p| p.exists());
        let sys_path = item
            .system_bytes
            .map(|_| item.session_dir.join("system.wav"))
            .filter(|p| p.exists());

        let blank_snapshot = PlayerSnapshot {
            track: None,
            position: Duration::ZERO,
            duration: None,
            paused: false,
            finished: true,
        };

        if let Some(path) = &mic_path {
            source_label(ui, "Microphone", p);
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
            source_label(ui, "System audio", p);
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

        divider(ui);
        ui.add_space(Space::md());

        // Transcript section
        if let Some(t) = transcript {
            transcript_block(ui, t);
        } else if let Some(status) = &job_status {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(Icon::Sparkle.glyph())
                        .font(TextStyle::Body.font_id())
                        .color(p.accent),
                );
                ui.add_space(Space::sm());
                ui.label(
                    RichText::new(format!("Transcribing · {}", status))
                        .font(TextStyle::Body.font_id())
                        .color(p.text_muted),
                );
            });
        } else {
            ui.horizontal(|ui| {
                if ghost_button_icon(ui, Icon::Sparkle, "transcribe with OpenAI").clicked() {
                    *transcribe = Some(item.clone());
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ghost_button_icon(ui, Icon::Reveal, "reveal in finder").clicked() {
                        *reveal = Some(item.session_dir.clone());
                    }
                });
            });
        }

        if transcript.is_some() {
            ui.add_space(Space::md());
            ui.horizontal(|ui| {
                if ghost_button_icon(ui, Icon::Reveal, "reveal in finder").clicked() {
                    *reveal = Some(item.session_dir.clone());
                }
                if ghost_button_icon(ui, Icon::Sparkle, "re-transcribe").clicked() {
                    *transcribe = Some(item.clone());
                }
            });
        }
    });
}

fn metadata_string(item: &RecordingSummary) -> String {
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
    parts.join("    ")
}

fn chip(ui: &mut egui::Ui, label: String, bg: Color32, fg: Color32) {
    let p = palette::current();
    let frame = egui::Frame::default()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::pill() as u8))
        .inner_margin(egui::Margin::symmetric(Space::sm() as i8, 2));
    frame.show(ui, |ui| {
        ui.label(
            RichText::new(label)
                .font(TextStyle::MonoSmall.font_id())
                .color(fg),
        );
    });
}

fn source_label(ui: &mut egui::Ui, label: &str, p: crate::design::Palette) {
    ui.label(
        RichText::new(label)
            .font(TextStyle::CapsLabel.font_id())
            .color(p.text_muted),
    );
}

fn transcript_block(ui: &mut egui::Ui, t: &Transcript) {
    let p = palette::current();
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("TRANSCRIPT")
                .font(TextStyle::CapsLabel.font_id())
                .color(p.text_muted),
        );
        ui.add_space(Space::sm());
        if let Some(lang) = &t.language {
            chip(ui, lang.to_uppercase(), p.accent_subtle, p.accent_strong);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!(
                    "{} · {} segments",
                    t.provider.label(),
                    t.segments.len()
                ))
                .font(TextStyle::MonoSmall.font_id())
                .color(p.text_subtle),
            );
        });
    });
    ui.add_space(Space::sm());

    let max_h = 320.0_f32.min(ui.available_height());
    egui::ScrollArea::vertical()
        .id_salt(format!("transcript_{}", t.id))
        .max_height(max_h)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for seg in &t.segments {
                segment_row(ui, seg);
                ui.add_space(Space::x2s());
            }
        });
}

fn segment_row(ui: &mut egui::Ui, seg: &TranscriptSegment) {
    let p = palette::current();
    let is_you = seg.speaker.as_deref() == Some("you");
    let speaker_color = if is_you { p.accent } else { p.text_muted };

    ui.horizontal(|ui| {
        // Timestamp column
        ui.add_sized(
            Vec2::new(64.0, 18.0),
            egui::Label::new(
                RichText::new(format_ts(seg.start_ms))
                    .font(TextStyle::MonoSmall.font_id())
                    .color(p.text_subtle),
            ),
        );
        // Speaker column
        if let Some(spk) = seg.speaker.as_deref() {
            ui.add_sized(
                Vec2::new(64.0, 18.0),
                egui::Label::new(
                    RichText::new(spk)
                        .font(TextStyle::CapsLabel.font_id())
                        .color(speaker_color),
                ),
            );
        }
        // Text
        ui.label(
            RichText::new(&seg.text)
                .font(TextStyle::Body.font_id())
                .color(p.text),
        );
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

fn handle_player_action(
    action: crate::components::AudioPlayerAction,
    file: &std::path::Path,
    session_dir: PathBuf,
    source: TrackSource,
    snapshot: &PlayerSnapshot,
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

// Backwards-compat shim — older code may reference recordings_list.
#[allow(dead_code)]
pub fn recordings_list(_ui: &mut egui::Ui, _history: &[RecordingSummary]) {
    // Removed in v2; kept as a no-op to avoid breaking any stale imports.
}
