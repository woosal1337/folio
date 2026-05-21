//! Sidebar navigation item. Icon + label, full-row click target, subtle
//! background on hover and a stronger one when active.

use egui::{RichText, Sense, Vec2};

use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};

pub fn nav_item(ui: &mut egui::Ui, icon: Icon, label: &str, active: bool) -> egui::Response {
    let p = palette::current();
    let height = 32.0;
    let width = ui.available_width();

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());

    let bg = if active {
        p.surface_overlay
    } else if response.hovered() {
        p.surface_subtle
    } else {
        egui::Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(Radius::sm() as u8), bg);

    let fg = if active { p.text } else { p.text_muted };

    let icon_font = TextStyle::Body.font_id();
    let label_font = if active {
        TextStyle::BodyStrong.font_id()
    } else {
        TextStyle::Body.font_id()
    };

    let icon_galley = painter.layout_no_wrap(icon.glyph().to_string(), icon_font, fg);
    let label_galley = painter.layout_no_wrap(label.to_string(), label_font, fg);

    let x = rect.min.x + Space::md();
    let icon_pos = egui::pos2(x, rect.center().y - icon_galley.rect.height() * 0.5);
    painter.galley(icon_pos, icon_galley.clone(), fg);

    let label_x = x + icon_galley.rect.width() + Space::sm() + 2.0;
    let label_pos = egui::pos2(label_x, rect.center().y - label_galley.rect.height() * 0.5);
    painter.galley(label_pos, label_galley, fg);

    // Active indicator: thin vertical bar on the left.
    if active {
        let bar_x = rect.min.x + 2.0;
        painter.line_segment(
            [
                egui::pos2(bar_x, rect.center().y - 8.0),
                egui::pos2(bar_x, rect.center().y + 8.0),
            ],
            egui::Stroke::new(2.0, p.text),
        );
    }

    let _ = RichText::new("");
    response
}
