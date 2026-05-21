//! Attune design system.
//!
//! Owns every visual decision the app makes: color palette, type scale,
//! spacing scale, corner radii, motion timings, icon set. Everything else
//! consumes these tokens via [`tokens`], [`palette`], [`typography`], and
//! [`icon`].
//!
//! Apply once at app startup with [`apply`].

// The design system defines a complete scale up-front. Not every token is
// consumed by today's screens; the rest are kept available so future
// components reach for an existing rung instead of inventing new ones.
#![allow(dead_code, unused_imports)]

pub mod icon;
pub mod palette;
pub mod tokens;
pub mod typography;

pub use icon::Icon;
pub use palette::Palette;
pub use tokens::{Motion, Radius, Space};
pub use typography::TextStyle;

/// Install the Attune theme on an egui context: load fonts, register text
/// styles, apply visuals and spacing. Call once during app creation.
pub fn apply(ctx: &egui::Context) {
    typography::install_fonts(ctx);
    apply_visuals(ctx);
    apply_style(ctx);
}

fn apply_visuals(ctx: &egui::Context) {
    let p = Palette::dark();
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;

    v.panel_fill = p.surface;
    v.window_fill = p.surface_raised;
    v.extreme_bg_color = p.surface_recessed;
    v.faint_bg_color = p.surface_subtle;
    v.code_bg_color = p.surface_subtle;
    v.window_stroke = egui::Stroke::new(1.0, p.border);
    v.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: egui::Color32::from_black_alpha(64),
    };

    v.hyperlink_color = p.text;
    v.selection.bg_fill = p.selection_bg;
    v.selection.stroke = egui::Stroke::new(1.0, p.text);

    // Non-interactive (labels)
    v.widgets.noninteractive.bg_fill = p.surface;
    v.widgets.noninteractive.weak_bg_fill = p.surface;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, p.text);
    v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(tokens::Radius::sm() as u8);

    // Inactive (buttons at rest)
    v.widgets.inactive.bg_fill = p.surface_raised;
    v.widgets.inactive.weak_bg_fill = p.surface_subtle;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, p.text);
    v.widgets.inactive.corner_radius = egui::CornerRadius::same(tokens::Radius::md() as u8);

    // Hovered
    v.widgets.hovered.bg_fill = p.surface_overlay;
    v.widgets.hovered.weak_bg_fill = p.surface_raised;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.border_strong);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, p.text);
    v.widgets.hovered.corner_radius = egui::CornerRadius::same(tokens::Radius::md() as u8);

    // Active (pressed)
    v.widgets.active.bg_fill = p.surface_overlay_strong;
    v.widgets.active.weak_bg_fill = p.surface_overlay;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, p.border_strong);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, p.text);
    v.widgets.active.corner_radius = egui::CornerRadius::same(tokens::Radius::md() as u8);

    // Open menus
    v.widgets.open.bg_fill = p.surface_raised;
    v.widgets.open.weak_bg_fill = p.surface_raised;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, p.border_strong);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, p.text);
    v.widgets.open.corner_radius = egui::CornerRadius::same(tokens::Radius::md() as u8);

    v.override_text_color = Some(p.text);
    v.popup_shadow = egui::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: egui::Color32::from_black_alpha(96),
    };
    v.menu_corner_radius = egui::CornerRadius::same(tokens::Radius::md() as u8);
    v.window_corner_radius = egui::CornerRadius::same(tokens::Radius::lg() as u8);

    ctx.set_visuals(v);
}

fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    typography::apply_text_styles(&mut style);

    style.spacing.item_spacing = egui::vec2(tokens::Space::xs(), tokens::Space::sm());
    style.spacing.button_padding = egui::vec2(tokens::Space::md(), tokens::Space::sm() * 0.875);
    style.spacing.menu_margin = egui::Margin::same(tokens::Space::sm() as i8);
    style.spacing.window_margin = egui::Margin::same(tokens::Space::lg() as i8);
    style.spacing.combo_height = 240.0;
    style.spacing.indent = tokens::Space::md();
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.bar_inner_margin = 4.0;
    style.spacing.scroll.bar_outer_margin = 2.0;
    style.spacing.interact_size = egui::vec2(40.0, 28.0);

    style.animation_time = tokens::Motion::fast();

    ctx.set_style(style);
}
