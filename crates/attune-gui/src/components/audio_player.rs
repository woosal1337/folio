//! Inline audio player widget. Used in the Library recording details to
//! play either the mic or system track without leaving the app.
//!
//! Visual layout, left to right:
//!
//!   ┌──┐  ━━━━━━━━━●━━━━━━━━━━━━━━━━━━━━  01:23 / 04:50
//!   │▶ │  scrubbable progress bar               mono current/total
//!   └──┘

use std::path::Path;

use egui::{Color32, Response, Sense, Vec2};

use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};
use crate::playback::{format_time, PlayerSnapshot};

pub struct AudioPlayerAction {
    pub toggle: bool,
    pub seek_fraction: Option<f32>,
}

/// Render the player row. Returns the action the user took (toggle or
/// scrub). Caller drives the underlying [`Player`].
pub fn audio_player(
    ui: &mut egui::Ui,
    file: &Path,
    snapshot: &PlayerSnapshot,
) -> AudioPlayerAction {
    let p = palette::current();
    let mut action = AudioPlayerAction {
        toggle: false,
        seek_fraction: None,
    };

    let playing = snapshot.is_playing(file) && !snapshot.paused;
    let active = snapshot
        .track
        .as_ref()
        .map(|t| t.file == file)
        .unwrap_or(false);

    ui.horizontal(|ui| {
        // Play / pause button (circular).
        let button_size = 32.0;
        let (btn_rect, btn_resp) = ui.allocate_exact_size(Vec2::splat(button_size), Sense::click());
        let painter = ui.painter();

        let bg = if btn_resp.hovered() {
            p.surface_overlay_strong
        } else {
            p.surface_overlay
        };
        painter.circle_filled(btn_rect.center(), button_size * 0.5, bg);
        painter.circle_stroke(
            btn_rect.center(),
            button_size * 0.5,
            egui::Stroke::new(1.0, p.border),
        );

        let glyph = if playing {
            Icon::Pause.glyph()
        } else {
            Icon::Play.glyph()
        };
        let galley =
            painter.layout_no_wrap(glyph.to_string(), TextStyle::BodyStrong.font_id(), p.text);
        let pos = egui::pos2(
            btn_rect.center().x - galley.rect.width() * 0.5,
            btn_rect.center().y - galley.rect.height() * 0.5,
        );
        painter.galley(pos, galley, p.text);

        if btn_resp.clicked() {
            action.toggle = true;
        }

        ui.add_space(Space::sm());

        // Progress bar + time labels share the remaining width.
        let height = 18.0;
        let time_label_width = 90.0;
        let bar_width = (ui.available_width() - time_label_width - Space::sm()).max(80.0);

        let (bar_rect, bar_resp) =
            ui.allocate_exact_size(Vec2::new(bar_width, height), Sense::click_and_drag());

        // Track line.
        let track_y = bar_rect.center().y;
        let track_rect = egui::Rect::from_min_size(
            egui::pos2(bar_rect.min.x, track_y - 2.0),
            Vec2::new(bar_rect.width(), 4.0),
        );
        ui.painter().rect_filled(
            track_rect,
            egui::CornerRadius::same(Radius::xs() as u8),
            p.surface_overlay,
        );

        // Fill segment.
        let fraction = if active { snapshot.fraction() } else { 0.0 };
        let fill_w = bar_rect.width() * fraction;
        if fill_w > 0.0 {
            let fill_rect = egui::Rect::from_min_size(
                egui::pos2(bar_rect.min.x, track_y - 2.0),
                Vec2::new(fill_w, 4.0),
            );
            ui.painter().rect_filled(
                fill_rect,
                egui::CornerRadius::same(Radius::xs() as u8),
                p.text,
            );
        }

        // Cursor knob.
        let knob_color = if active { p.text } else { p.text_subtle };
        let knob_x = bar_rect.min.x + fill_w;
        ui.painter()
            .circle_filled(egui::pos2(knob_x, track_y), 6.0, knob_color);
        ui.painter().circle_stroke(
            egui::pos2(knob_x, track_y),
            6.0,
            egui::Stroke::new(1.0, p.surface_overlay),
        );

        // Hit testing for scrub.
        if let Some(pos) = bar_resp.interact_pointer_pos() {
            let f = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
            action.seek_fraction = Some(f);
        }

        ui.add_space(Space::sm());

        // Time labels.
        let current = if active {
            format_time(snapshot.position)
        } else {
            "00:00".into()
        };
        let total = snapshot
            .duration
            .map(format_time)
            .unwrap_or_else(|| "--:--".into());
        let label = format!("{current} / {total}");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(label)
                    .font(TextStyle::MonoSmall.font_id())
                    .color(if active { p.text } else { p.text_muted }),
            );
        });

        let _: (Response, Response) = (btn_resp, bar_resp);
    });

    let _ = Color32::TRANSPARENT;
    action
}
