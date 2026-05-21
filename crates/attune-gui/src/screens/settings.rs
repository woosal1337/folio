//! Settings screen. Output folder, mic device, model selection, retention
//! policy. Stub for now; uses the same Persisted state used by Record.

use egui::{Align, Layout, RichText};

use crate::components::{caption, card, ghost_button_icon, labeled_section, mono};
use crate::design::tokens::Space;
use crate::design::{palette, Icon, TextStyle};
use crate::state::{Persisted, Runtime};

pub fn show(ui: &mut egui::Ui, _ctx: &egui::Context, _rt: &mut Runtime, persisted: &mut Persisted) {
    let p = palette::current();
    ui.label(
        RichText::new("Settings")
            .font(TextStyle::Title.font_id())
            .color(p.text),
    );
    ui.add_space(Space::sm());
    ui.label(caption("Behaviour and storage."));
    ui.add_space(Space::xl());

    card(ui, |ui| {
        labeled_section(ui, Some(Icon::Folder), "OUTPUT FOLDER", |ui| {
            ui.horizontal(|ui| {
                let raw = persisted.output_dir.display().to_string();
                let display = if raw.len() > 56 {
                    format!("…{}", &raw[raw.len() - 55..])
                } else {
                    raw.clone()
                };
                ui.add_sized(
                    [ui.available_width() - 90.0, 28.0],
                    egui::Label::new(mono(display).color(p.text_muted)).truncate(),
                )
                .on_hover_text(raw);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ghost_button_icon(ui, Icon::FolderOpen, "change").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            persisted.output_dir = dir;
                        }
                    }
                });
            });
        });
    });
}
