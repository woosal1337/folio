//! Editor screen. Built-in markdown editor for meeting notes that live
//! alongside the audio and transcript. Stub for now.

use egui::RichText;

use crate::components::{caption, empty_state};
use crate::design::tokens::Space;
use crate::design::{palette, Icon, TextStyle};
use crate::state::{Persisted, Runtime};

pub fn show(
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
    _rt: &mut Runtime,
    _persisted: &mut Persisted,
) {
    let p = palette::current();
    ui.label(
        RichText::new("Editor")
            .font(TextStyle::Title.font_id())
            .color(p.text),
    );
    ui.add_space(Space::sm());
    ui.label(caption("Write meeting notes alongside the recording."));
    ui.add_space(Space::xl());
    empty_state(
        ui,
        Icon::Editor,
        "Markdown editor coming soon",
        "A focused, distraction-free writing surface with live preview and side-by-side transcript will live here.",
    );
}
