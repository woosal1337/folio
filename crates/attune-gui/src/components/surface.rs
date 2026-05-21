//! Card / surface wrapper. Solid background, subtle border, rounded
//! corners, generous padding, always fills the available width so multiple
//! cards stack with consistent left and right edges.

use crate::design::palette;
use crate::design::tokens::{Radius, Space};

pub fn card<R>(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
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
    frame
        .show(ui, |ui| {
            // Force the inner ui to take the full available width so all
            // cards in a column share the same right edge.
            let inner_w = (avail_w - (Space::lg() * 2.0)).max(0.0);
            ui.set_min_width(inner_w);
            contents(ui)
        })
        .inner
}
