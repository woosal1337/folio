//! Recording status pill: a small "● recording 00:23" / "○ idle" badge for
//! page headers and the menu bar overlay.

use egui::{RichText, Sense, Vec2};

use crate::design::tokens::{Radius, Space};
use crate::design::{palette, TextStyle};

pub fn status_pill(ui: &mut egui::Ui, ctx: &egui::Context, recording: bool, timer_label: &str) {
    let p = palette::current();

    let bg = if recording {
        p.danger_subtle
    } else {
        p.surface_subtle
    };
    let border = if recording {
        p.danger.linear_multiply(0.4)
    } else {
        p.border
    };

    let frame = egui::Frame::default()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(egui::CornerRadius::same(Radius::pill() as u8))
        .inner_margin(egui::Margin::symmetric(
            Space::sm() as i8 + 2,
            Space::xs() as i8 + 1,
        ));

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Animated dot.
            let dot_size = 8.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(dot_size), Sense::hover());
            let painter = ui.painter();
            if recording {
                let t = ctx.input(|i| i.time);
                let pulse = 0.55 + 0.45 * (t * 3.2).sin();
                let alpha = (pulse.clamp(0.0, 1.0) * 255.0) as u8;
                let glow = egui::Color32::from_rgba_unmultiplied(
                    p.danger.r(),
                    p.danger.g(),
                    p.danger.b(),
                    alpha,
                );
                painter.circle_filled(rect.center(), dot_size * 0.85, glow);
                painter.circle_filled(rect.center(), dot_size * 0.42, p.danger);
                ctx.request_repaint_after(std::time::Duration::from_millis(80));
            } else {
                painter.circle_stroke(
                    rect.center(),
                    dot_size * 0.42,
                    egui::Stroke::new(1.4, p.text_subtle),
                );
            }

            ui.add_space(Space::xs() + 2.0);

            // Label
            let (label_color, label_text) = if recording {
                (p.text, "recording")
            } else {
                (p.text_muted, "idle")
            };
            ui.label(
                RichText::new(label_text)
                    .font(TextStyle::Caption.font_id())
                    .color(label_color),
            );

            ui.add_space(Space::sm());

            // Timer mono.
            ui.label(
                RichText::new(timer_label)
                    .font(TextStyle::MonoSmall.font_id())
                    .color(if recording { p.text } else { p.text_subtle }),
            );
        });
    });
}
