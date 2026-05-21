//! Transcripts screen. Lists Whisper transcripts of recorded sessions.
//! Stub until the transcription pipeline lands.

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
        RichText::new("Transcripts")
            .font(TextStyle::Title.font_id())
            .color(p.text),
    );
    ui.add_space(Space::sm());
    ui.label(caption("Searchable text from every recording."));
    ui.add_space(Space::xl());
    empty_state(
        ui,
        Icon::Transcript,
        "Coming soon",
        "Whisper-based transcription with speaker labels lands in the next milestone.",
    );
}
