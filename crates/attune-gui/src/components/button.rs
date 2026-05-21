//! Buttons. Three semantic variants: primary, secondary, ghost. Plus an
//! icon-only ghost variant. All use the same sizing so they line up in
//! horizontal rows without manual tweaking.

use egui::{RichText, Vec2};

use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};

const BUTTON_HEIGHT: f32 = 30.0;
const BUTTON_HEIGHT_LG: f32 = 36.0;

/// High-emphasis action. One per row at most.
pub fn primary_button(ui: &mut egui::Ui, label: impl Into<String>) -> egui::Response {
    let p = palette::current();
    let text = RichText::new(label)
        .font(TextStyle::BodyStrong.font_id())
        .color(p.text_inverse);
    let button = egui::Button::new(text)
        .corner_radius(egui::CornerRadius::same(Radius::md() as u8))
        .fill(p.text)
        .stroke(egui::Stroke::NONE)
        .min_size(Vec2::new(0.0, BUTTON_HEIGHT_LG));
    ui.add(button)
}

/// Medium-emphasis action. Multiple per row is fine.
pub fn secondary_button(ui: &mut egui::Ui, label: impl Into<String>) -> egui::Response {
    let p = palette::current();
    let text = RichText::new(label)
        .font(TextStyle::Body.font_id())
        .color(p.text);
    let button = egui::Button::new(text)
        .corner_radius(egui::CornerRadius::same(Radius::md() as u8))
        .fill(p.surface_raised)
        .stroke(egui::Stroke::new(1.0, p.border))
        .min_size(Vec2::new(0.0, BUTTON_HEIGHT));
    ui.add(button)
}

/// Low-emphasis text-only action. Subtle background appears on hover.
pub fn ghost_button(ui: &mut egui::Ui, label: impl Into<String>) -> egui::Response {
    let p = palette::current();
    let text = RichText::new(label)
        .font(TextStyle::Body.font_id())
        .color(p.text_muted);
    let button = egui::Button::new(text)
        .corner_radius(egui::CornerRadius::same(Radius::sm() as u8))
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE)
        .min_size(Vec2::new(0.0, BUTTON_HEIGHT));
    ui.add(button)
}

/// Ghost button with a leading icon glyph.
pub fn ghost_button_icon(
    ui: &mut egui::Ui,
    icon: Icon,
    label: impl Into<String>,
) -> egui::Response {
    let p = palette::current();
    let mut text = String::new();
    text.push_str(icon.glyph());
    text.push_str("  ");
    text.push_str(&label.into());
    let rich = RichText::new(text)
        .font(TextStyle::Body.font_id())
        .color(p.text_muted);
    let button = egui::Button::new(rich)
        .corner_radius(egui::CornerRadius::same(Radius::sm() as u8))
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE)
        .min_size(Vec2::new(0.0, BUTTON_HEIGHT));
    let resp = ui.add(button);
    let _ = Space::sm();
    resp
}
