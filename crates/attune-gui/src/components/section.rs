//! Form-section helpers: a small uppercase-style label sitting above a
//! field, optionally with a leading icon. Used pervasively on the record
//! screen and in settings.

use egui::RichText;

use crate::components::text;
use crate::design::tokens::Space;
use crate::design::{palette, Icon, TextStyle};

/// A row with an optional icon and a caption-styled label. Used to head a
/// group of related controls.
pub fn section_header(ui: &mut egui::Ui, icon: Option<Icon>, label: impl Into<String>) {
    let p = palette::current();
    ui.horizontal(|ui| {
        if let Some(i) = icon {
            ui.label(
                RichText::new(i.glyph())
                    .font(TextStyle::Body.font_id())
                    .color(p.text_subtle),
            );
            ui.add_space(Space::xs());
        }
        ui.label(text::caption(label.into()).color(p.text_muted));
    });
    ui.add_space(Space::xs());
}

/// A section: header + a closure that draws the body. Adds bottom spacing
/// so multiple sections compose vertically without callers managing the
/// gap.
pub fn labeled_section<R>(
    ui: &mut egui::Ui,
    icon: Option<Icon>,
    label: impl Into<String>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    section_header(ui, icon, label);
    let r = body(ui);
    ui.add_space(Space::md());
    r
}
