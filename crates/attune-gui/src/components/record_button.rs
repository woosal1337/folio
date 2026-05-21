//! The main capture toggle. A wide pill that says "start recording" in idle
//! and "stop" while recording, with a leading icon. Has a hover/press tint
//! so the button feels responsive.

use egui::{Sense, Vec2};

use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};

pub fn record_button(ui: &mut egui::Ui, recording: bool) -> egui::Response {
    let p = palette::current();
    let height = 44.0;
    let available_width = ui.available_width();

    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(available_width, height), Sense::click());

    let (mut bg, fg, accent, label_text, icon_glyph) = if recording {
        (
            p.danger_subtle,
            p.text,
            p.danger,
            "stop recording",
            Icon::Square.glyph(),
        )
    } else {
        (
            p.text,
            p.text_inverse,
            p.text_inverse,
            "start recording",
            Icon::CircleFilled.glyph(),
        )
    };
    if response.hovered() {
        bg = lighten(bg, 6);
    }
    if response.is_pointer_button_down_on() {
        bg = lighten(bg, 12);
    }

    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(Radius::lg() as u8), bg);
    let stroke = if recording {
        egui::Stroke::new(1.0, p.danger.linear_multiply(0.35))
    } else {
        egui::Stroke::NONE
    };
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(Radius::lg() as u8),
        stroke,
        egui::StrokeKind::Inside,
    );

    let icon_font = TextStyle::BodyStrong.font_id();
    let label_font = TextStyle::BodyStrong.font_id();

    let icon_galley = painter.layout_no_wrap(icon_glyph.to_string(), icon_font, accent);
    let label_galley = painter.layout_no_wrap(label_text.to_string(), label_font, fg);
    let gap = Space::sm();
    let total_w = icon_galley.rect.width() + gap + label_galley.rect.width();
    let mut x = rect.center().x - total_w * 0.5;
    let icon_pos = egui::pos2(x, rect.center().y - icon_galley.rect.height() * 0.5);
    painter.galley(icon_pos, icon_galley.clone(), accent);
    x += icon_galley.rect.width() + gap;
    let label_pos = egui::pos2(x, rect.center().y - label_galley.rect.height() * 0.5);
    painter.galley(label_pos, label_galley.clone(), fg);

    response
}

fn lighten(c: egui::Color32, amount: i32) -> egui::Color32 {
    let delta = (amount * 4) as u8;
    egui::Color32::from_rgb(
        c.r().saturating_add(delta),
        c.g().saturating_add(delta),
        c.b().saturating_add(delta),
    )
}
