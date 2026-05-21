//! Hairline dividers. One pixel, palette border color.

use crate::design::palette;
use crate::design::tokens::Layout;

pub fn divider(ui: &mut egui::Ui) {
    let p = palette::current();
    let rect = ui.available_rect_before_wrap();
    let y = rect.min.y;
    ui.painter().line_segment(
        [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
        egui::Stroke::new(Layout::hairline(), p.border),
    );
    ui.add_space(Layout::hairline());
}

pub fn vertical_divider(ui: &mut egui::Ui) {
    let p = palette::current();
    let rect = ui.available_rect_before_wrap();
    let x = rect.min.x;
    ui.painter().line_segment(
        [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
        egui::Stroke::new(Layout::hairline(), p.border),
    );
}
