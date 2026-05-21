//! Semantic color palette with light + dark variants and a runtime switch.
//!
//! Components pull colors via [`current()`], which returns the active
//! [`Palette`] for the current theme. Calling [`set_theme`] flips every
//! component on the next paint without touching their code.
//!
//! The light theme is the default and follows the Wispr Flow visual
//! direction: warm cream background, sage accent. The dark theme keeps the
//! monochrome direction from earlier versions for users who want it.

use std::sync::atomic::{AtomicU8, Ordering};

use egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
    }
    pub fn all() -> &'static [Theme] {
        &[Theme::Light, Theme::Dark]
    }
}

static ACTIVE_THEME: AtomicU8 = AtomicU8::new(0);

pub fn set_theme(theme: Theme) {
    let v: u8 = match theme {
        Theme::Light => 0,
        Theme::Dark => 1,
    };
    ACTIVE_THEME.store(v, Ordering::Relaxed);
}

pub fn active_theme() -> Theme {
    match ACTIVE_THEME.load(Ordering::Relaxed) {
        1 => Theme::Dark,
        _ => Theme::Light,
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    /// True if this palette represents the dark theme. Components can branch
    /// on this when they need theme-specific tweaks (rare).
    pub is_dark: bool,

    // Background tiers
    pub bg: Color32,
    pub sidebar_bg: Color32,
    pub surface: Color32,
    pub surface_subtle: Color32,
    pub surface_raised: Color32,
    pub surface_overlay: Color32,
    pub surface_overlay_strong: Color32,
    /// Dark background used for hero cards regardless of theme.
    pub hero_bg: Color32,

    // Borders
    pub border: Color32,
    pub border_strong: Color32,
    pub border_focus: Color32,

    // Text
    pub text: Color32,
    pub text_muted: Color32,
    pub text_subtle: Color32,
    pub text_inverse: Color32,

    // Accents (sage in light, monochrome in dark)
    pub accent: Color32,
    pub accent_subtle: Color32,
    pub accent_strong: Color32,
    pub accent_on: Color32,

    // Status
    pub danger: Color32,
    pub danger_subtle: Color32,
    pub warning: Color32,
    pub success: Color32,

    // Pro / brand badge
    pub lilac_bg: Color32,
    pub lilac_text: Color32,

    // Misc
    pub selection_bg: Color32,
    pub shadow: Color32,
    pub overlay_scrim: Color32,
}

impl Palette {
    /// Warm cream + sage. The default Attune look.
    pub const fn light() -> Self {
        Self {
            is_dark: false,

            bg: Color32::from_rgb(245, 242, 236),
            sidebar_bg: Color32::from_rgb(239, 235, 226),
            surface: Color32::from_rgb(255, 255, 255),
            surface_subtle: Color32::from_rgb(250, 247, 240),
            surface_raised: Color32::from_rgb(255, 255, 255),
            surface_overlay: Color32::from_rgb(232, 226, 213),
            surface_overlay_strong: Color32::from_rgb(214, 207, 192),
            hero_bg: Color32::from_rgb(31, 35, 41),

            border: Color32::from_rgb(232, 227, 216),
            border_strong: Color32::from_rgb(201, 194, 176),
            border_focus: Color32::from_rgb(45, 110, 94),

            text: Color32::from_rgb(26, 26, 26),
            text_muted: Color32::from_rgb(107, 107, 107),
            text_subtle: Color32::from_rgb(154, 150, 140),
            text_inverse: Color32::from_rgb(255, 255, 255),

            accent: Color32::from_rgb(45, 110, 94),
            accent_subtle: Color32::from_rgb(224, 237, 231),
            accent_strong: Color32::from_rgb(31, 80, 68),
            accent_on: Color32::from_rgb(255, 255, 255),

            danger: Color32::from_rgb(200, 70, 58),
            danger_subtle: Color32::from_rgb(245, 220, 217),
            warning: Color32::from_rgb(212, 162, 58),
            success: Color32::from_rgb(62, 132, 112),

            lilac_bg: Color32::from_rgb(232, 220, 255),
            lilac_text: Color32::from_rgb(124, 91, 202),

            selection_bg: Color32::from_rgb(224, 237, 231),
            shadow: Color32::from_black_alpha(28),
            overlay_scrim: Color32::from_rgba_premultiplied(13, 13, 16, 168),
        }
    }

    /// Monochrome dark variant. Same component semantics, darker surfaces,
    /// neutral accents.
    pub const fn dark() -> Self {
        Self {
            is_dark: true,

            bg: Color32::from_rgb(13, 13, 14),
            sidebar_bg: Color32::from_rgb(16, 16, 18),
            surface: Color32::from_rgb(18, 18, 20),
            surface_subtle: Color32::from_rgb(22, 22, 24),
            surface_raised: Color32::from_rgb(24, 24, 26),
            surface_overlay: Color32::from_rgb(34, 34, 38),
            surface_overlay_strong: Color32::from_rgb(50, 50, 55),
            hero_bg: Color32::from_rgb(10, 10, 12),

            border: Color32::from_rgb(36, 36, 40),
            border_strong: Color32::from_rgb(64, 64, 70),
            border_focus: Color32::from_rgb(130, 130, 138),

            text: Color32::from_rgb(232, 232, 234),
            text_muted: Color32::from_rgb(165, 165, 170),
            text_subtle: Color32::from_rgb(115, 115, 120),
            text_inverse: Color32::from_rgb(15, 15, 17),

            accent: Color32::from_rgb(232, 232, 234),
            accent_subtle: Color32::from_rgb(48, 48, 52),
            accent_strong: Color32::from_rgb(255, 255, 255),
            accent_on: Color32::from_rgb(15, 15, 17),

            danger: Color32::from_rgb(235, 80, 80),
            danger_subtle: Color32::from_rgb(60, 22, 24),
            warning: Color32::from_rgb(232, 178, 78),
            success: Color32::from_rgb(120, 200, 130),

            lilac_bg: Color32::from_rgb(58, 44, 96),
            lilac_text: Color32::from_rgb(196, 175, 255),

            selection_bg: Color32::from_rgb(70, 70, 76),
            shadow: Color32::from_black_alpha(96),
            overlay_scrim: Color32::from_black_alpha(184),
        }
    }
}

/// The active [`Palette`]. Cheap (Copy), call freely from component code.
pub fn current() -> Palette {
    match active_theme() {
        Theme::Light => Palette::light(),
        Theme::Dark => Palette::dark(),
    }
}
