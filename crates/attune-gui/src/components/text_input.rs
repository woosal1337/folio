//! Standardised text input. One height, one padding, one border style.
//! Every form field across the app uses this so vertical rhythm, baselines,
//! and right-edge alignment stay consistent.

use egui::{Margin, Sense, Vec2};

use crate::design::palette;
use crate::design::tokens::{Radius, Space};
use crate::design::TextStyle;

const INPUT_HEIGHT: f32 = 36.0;

/// Single-line text input, fills the available width.
pub fn text_input(ui: &mut egui::Ui, value: &mut String, hint: &str) -> egui::Response {
    text_input_with_options(ui, value, hint, TextInputOptions::default())
}

/// Single-line text input rendered as a password field. Hides characters
/// behind dots and disables clipboard copy.
pub fn password_input(ui: &mut egui::Ui, value: &mut String, hint: &str) -> egui::Response {
    text_input_with_options(
        ui,
        value,
        hint,
        TextInputOptions {
            password: true,
            ..Default::default()
        },
    )
}

/// Monospaced single-line input (paths, identifiers, API keys).
pub fn mono_input(ui: &mut egui::Ui, value: &mut String, hint: &str) -> egui::Response {
    text_input_with_options(
        ui,
        value,
        hint,
        TextInputOptions {
            mono: true,
            ..Default::default()
        },
    )
}

#[derive(Default)]
pub struct TextInputOptions {
    pub password: bool,
    pub mono: bool,
    pub width: Option<f32>,
}

pub fn text_input_with_options(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    opts: TextInputOptions,
) -> egui::Response {
    let p = palette::current();
    let width = opts.width.unwrap_or_else(|| ui.available_width());
    let radius = Radius::md() as u8;

    let (rect, _interact) = ui.allocate_exact_size(Vec2::new(width, INPUT_HEIGHT), Sense::hover());

    // Background + border drawn first so the TextEdit sits inside cleanly.
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(radius), p.surface);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(radius),
        egui::Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );

    // Inner area where the editable text lives.
    let pad_x = Space::sm() + Space::x2s();
    let inner = rect.shrink2(Vec2::new(pad_x, 0.0));

    let font = if opts.mono {
        TextStyle::Mono.font_id()
    } else {
        TextStyle::Body.font_id()
    };

    let mut edit = egui::TextEdit::singleline(value)
        .hint_text(hint)
        .font(font)
        .frame(false)
        .margin(Margin::ZERO)
        .vertical_align(egui::Align::Center)
        .desired_width(inner.width());
    if opts.password {
        edit = edit.password(true);
    }

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.add(edit)
}
