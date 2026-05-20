//! Visual theme for the Attune GUI.
//!
//! Monochrome dark, generous spacing, mono accent. Matches the Intel /
//! @woosal1337 brand direction: black/white only, opacity-based depth,
//! minimal chrome. No accent colors except status (red for recording,
//! subdued green for success).

use egui::{Color32, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};

pub const RECORD_RED: Color32 = Color32::from_rgb(232, 76, 76);
pub const SUCCESS_GREEN: Color32 = Color32::from_rgb(120, 200, 130);
pub const SUBTLE: Color32 = Color32::from_rgb(140, 140, 140);
pub const SUBTLER: Color32 = Color32::from_rgb(100, 100, 100);
pub const STRONG: Color32 = Color32::from_rgb(240, 240, 240);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(14, 14, 14);
    visuals.window_fill = Color32::from_rgb(18, 18, 18);
    visuals.extreme_bg_color = Color32::from_rgb(8, 8, 8);
    visuals.faint_bg_color = Color32::from_rgb(22, 22, 22);
    visuals.code_bg_color = Color32::from_rgb(22, 22, 22);

    // Hyperlink + selection
    visuals.hyperlink_color = Color32::from_rgb(220, 220, 220);
    visuals.selection.bg_fill = Color32::from_rgb(70, 70, 70);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(200, 200, 200));

    // Widget colors
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(18, 18, 18);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(18, 18, 18);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(34, 34, 34));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(190, 190, 190));

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 28, 28);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(24, 24, 24);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(50, 50, 50));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(210, 210, 210));

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 40, 40);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(34, 34, 34);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(80, 80, 80));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, STRONG);

    visuals.widgets.active.bg_fill = Color32::from_rgb(55, 55, 55);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(45, 45, 45);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(110, 110, 110));
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, STRONG);

    visuals.widgets.open.bg_fill = Color32::from_rgb(34, 34, 34);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, Color32::from_rgb(60, 60, 60));

    visuals.override_text_color = Some(Color32::from_rgb(220, 220, 220));

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(22.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.5, FontFamily::Monospace),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.5, FontFamily::Proportional),
    );

    style.spacing.item_spacing = egui::vec2(8.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = Margin::same(24);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.combo_height = 220.0;
    style.spacing.indent = 14.0;

    style.visuals.window_corner_radius = egui::CornerRadius::same(8);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(6);

    ctx.set_style(style);
}
