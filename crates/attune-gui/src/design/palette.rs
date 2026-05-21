//! Semantic color palette. Components should never use a `Color32` literal
//! directly — they pull a named slot from [`Palette`] so theme switches
//! (e.g., a future light mode) ripple through the app from one place.

use egui::Color32;

#[derive(Clone, Copy)]
pub struct Palette {
    // Surfaces, ordered by elevation from background up to overlays.
    pub surface_recessed: Color32,
    pub surface: Color32,
    pub surface_subtle: Color32,
    pub surface_raised: Color32,
    pub surface_overlay: Color32,
    pub surface_overlay_strong: Color32,

    // Borders.
    pub border: Color32,
    pub border_strong: Color32,
    pub border_focus: Color32,

    // Text.
    pub text: Color32,
    pub text_muted: Color32,
    pub text_subtle: Color32,
    pub text_inverse: Color32,

    // Accents and status.
    pub accent: Color32,
    pub accent_subtle: Color32,
    pub danger: Color32,
    pub danger_subtle: Color32,
    pub success: Color32,
    pub warning: Color32,

    // Misc.
    pub selection_bg: Color32,
    pub shadow: Color32,
    pub overlay_scrim: Color32,
}

impl Palette {
    /// The dark mode palette. Monochrome black/white with a single red used
    /// only for the recording status.
    pub const fn dark() -> Self {
        Self {
            surface_recessed: Color32::from_rgb(7, 7, 8),
            surface: Color32::from_rgb(13, 13, 14),
            surface_subtle: Color32::from_rgb(17, 17, 19),
            surface_raised: Color32::from_rgb(22, 22, 24),
            surface_overlay: Color32::from_rgb(30, 30, 33),
            surface_overlay_strong: Color32::from_rgb(40, 40, 44),

            border: Color32::from_rgb(32, 32, 35),
            border_strong: Color32::from_rgb(58, 58, 64),
            border_focus: Color32::from_rgb(120, 120, 128),

            text: Color32::from_rgb(232, 232, 234),
            text_muted: Color32::from_rgb(165, 165, 170),
            text_subtle: Color32::from_rgb(115, 115, 120),
            text_inverse: Color32::from_rgb(15, 15, 17),

            accent: Color32::from_rgb(232, 232, 234),
            accent_subtle: Color32::from_rgb(50, 50, 54),

            danger: Color32::from_rgb(235, 80, 80),
            danger_subtle: Color32::from_rgb(60, 22, 24),
            success: Color32::from_rgb(120, 200, 130),
            warning: Color32::from_rgb(232, 178, 78),

            selection_bg: Color32::from_rgb(70, 70, 76),
            shadow: Color32::from_black_alpha(96),
            overlay_scrim: Color32::from_black_alpha(160),
        }
    }
}

/// Convenience constructor for the current theme. Today this is the dark
/// palette unconditionally; a future settings toggle will swap this.
pub fn current() -> Palette {
    Palette::dark()
}
