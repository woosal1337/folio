//! Library screen. Full list of recorded sessions. v0 stub uses the
//! existing history scan; future versions add filtering, tagging, search.

use egui::{Align, Layout, RichText};

use crate::components::{caption, empty_state, ghost_button_icon};
use crate::design::tokens::Space;
use crate::design::{palette, Icon, TextStyle};
use crate::state::{self, refresh_history, Persisted, Runtime};

pub fn show(ui: &mut egui::Ui, _ctx: &egui::Context, rt: &mut Runtime, persisted: &mut Persisted) {
    let p = palette::current();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Library")
                .font(TextStyle::Title.font_id())
                .color(p.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Refresh, "refresh").clicked() {
                refresh_history(rt, &persisted.output_dir);
            }
        });
    });
    ui.add_space(Space::sm());
    ui.label(caption("All recorded sessions from your output folder."));
    ui.add_space(Space::xl());

    if rt.history.is_empty() {
        empty_state(
            ui,
            Icon::Library,
            "Library is empty",
            "Start your first recording from the Record screen.",
        );
    } else {
        super::record::recordings_list(ui, &rt.history);
    }

    let _ = state::format_bytes(0);
}
