//! Card / surface wrapper. Solid background, subtle border, rounded
//! corners, generous padding. Use this anywhere you'd otherwise reach for
//! `egui::Frame::default()`.

use crate::design::palette;
use crate::design::tokens::{Radius, Space};

pub fn card<R>(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let p = palette::current();
    let frame = egui::Frame::default()
        .fill(p.surface_raised)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::lg() as u8))
        .inner_margin(egui::Margin::symmetric(
            Space::lg() as i8,
            Space::md() as i8,
        ));
    frame.show(ui, contents).inner
}
