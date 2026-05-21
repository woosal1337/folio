//! Empty state. Used on screens that don't have content yet (Library,
//! Transcripts, Editor) and on the recordings list when there are no
//! recordings.

use egui::RichText;

use crate::components::text;
use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};

pub fn empty_state(
    ui: &mut egui::Ui,
    icon: Icon,
    title: impl Into<String>,
    description: impl Into<String>,
) {
    let p = palette::current();
    let frame = egui::Frame::default()
        .fill(p.surface)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::lg() as u8))
        .inner_margin(egui::Margin::symmetric(
            Space::xl() as i8,
            Space::xl() as i8,
        ));
    frame.show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(Space::md());
            ui.label(
                RichText::new(icon.glyph())
                    .font(egui::FontId::new(28.0, egui::FontFamily::Proportional))
                    .color(p.text_subtle),
            );
            ui.add_space(Space::sm());
            ui.label(text::body_strong(title.into()).color(p.text));
            ui.add_space(Space::x2s());
            let _ = TextStyle::Caption;
            ui.label(text::caption(description.into()).color(p.text_muted));
            ui.add_space(Space::md());
        });
    });
}
