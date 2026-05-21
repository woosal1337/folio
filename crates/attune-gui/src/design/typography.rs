//! Typography setup. Loads bundled Inter (UI), Spectral (serif display),
//! and JetBrains Mono (mono) fonts, registers an icon font from
//! `egui_phosphor`, and defines a small semantic text-style scale.
//!
//! Component code never references a `FontId` directly. It picks a
//! [`TextStyle`] (Display, Title, Heading, …) and lets the style supply
//! the size, family, and line spacing.

use egui::{FontData, FontDefinitions, FontFamily, FontId, Style};

/// Semantic text styles. Sizes follow a roughly 1.25 type scale rooted at
/// 14 px body for the light theme (vs 13 in the prior dark theme — the
/// warm cream background reads better at slightly heavier weight).
#[derive(Clone, Copy, Debug)]
pub enum TextStyle {
    /// Serif display, used in hero card headings.
    Display,
    /// Serif italic for emphasis inside Display.
    DisplayItalic,
    /// Screen titles.
    Title,
    /// Section headings inside a screen.
    Heading,
    /// Default body text.
    Body,
    /// Body text emphasised by weight (medium / semibold).
    BodyStrong,
    /// Default monospace, used for paths, code, and the recording timer.
    Mono,
    /// Smaller monospace for inline tags and metadata.
    MonoSmall,
    /// Smaller, secondary text. Labels above fields, metadata.
    Caption,
    /// Tracked uppercase labels for section markers (TODAY, SETTINGS, etc.).
    CapsLabel,
    /// Smallest text. Footer disclaimers, fine print.
    Micro,
}

/// Named font families. Components compose families with sizes via
/// `FontId::new(size, family)`.
pub fn family_proportional() -> FontFamily {
    FontFamily::Proportional
}
pub fn family_monospace() -> FontFamily {
    FontFamily::Monospace
}
pub fn family_serif() -> FontFamily {
    FontFamily::Name("serif".into())
}

impl TextStyle {
    pub fn font_id(self) -> FontId {
        match self {
            TextStyle::Display => FontId::new(36.0, family_serif()),
            TextStyle::DisplayItalic => FontId::new(36.0, FontFamily::Name("serif_italic".into())),
            TextStyle::Title => FontId::new(24.0, family_proportional()),
            TextStyle::Heading => FontId::new(16.0, family_proportional()),
            TextStyle::Body => FontId::new(14.0, family_proportional()),
            TextStyle::BodyStrong => FontId::new(14.0, family_proportional()),
            TextStyle::Mono => FontId::new(13.0, family_monospace()),
            TextStyle::MonoSmall => FontId::new(11.5, family_monospace()),
            TextStyle::Caption => FontId::new(12.0, family_proportional()),
            TextStyle::CapsLabel => FontId::new(11.0, family_proportional()),
            TextStyle::Micro => FontId::new(10.5, family_proportional()),
        }
    }
}

// ---------------------------------------------------------------------------
// Bundled font files (Open Font License)
// ---------------------------------------------------------------------------

const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.otf");
const INTER_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Inter-Medium.otf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.otf");
const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
const JETBRAINS_MONO_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Medium.ttf");
const SPECTRAL_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Spectral-Regular.ttf");
const SPECTRAL_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Spectral-Medium.ttf");
const SPECTRAL_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Spectral-Italic.ttf");

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Bundled font data.
    fonts
        .font_data
        .insert("inter".into(), FontData::from_static(INTER_REGULAR).into());
    fonts.font_data.insert(
        "inter_medium".into(),
        FontData::from_static(INTER_MEDIUM).into(),
    );
    fonts.font_data.insert(
        "inter_semibold".into(),
        FontData::from_static(INTER_SEMIBOLD).into(),
    );
    fonts.font_data.insert(
        "jbmono".into(),
        FontData::from_static(JETBRAINS_MONO_REGULAR).into(),
    );
    fonts.font_data.insert(
        "jbmono_medium".into(),
        FontData::from_static(JETBRAINS_MONO_MEDIUM).into(),
    );
    fonts.font_data.insert(
        "spectral".into(),
        FontData::from_static(SPECTRAL_REGULAR).into(),
    );
    fonts.font_data.insert(
        "spectral_medium".into(),
        FontData::from_static(SPECTRAL_MEDIUM).into(),
    );
    fonts.font_data.insert(
        "spectral_italic".into(),
        FontData::from_static(SPECTRAL_ITALIC).into(),
    );

    // Phosphor icon font lives on its own family so we can switch between
    // text and icons inline without affecting glyph fallback for normal text.
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    // Proportional family stack — Inter weights with phosphor fallback added
    // by add_to_fonts above.
    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.insert(0, "inter".into());
    prop.insert(1, "inter_medium".into());
    prop.insert(2, "inter_semibold".into());

    // Monospace family stack — JetBrains Mono first.
    let mono = fonts.families.entry(FontFamily::Monospace).or_default();
    mono.insert(0, "jbmono".into());
    mono.insert(1, "jbmono_medium".into());

    // Serif family (named) for Display text. Regular + Medium.
    let serif = fonts
        .families
        .entry(FontFamily::Name("serif".into()))
        .or_default();
    serif.insert(0, "spectral".into());
    serif.insert(1, "spectral_medium".into());
    // Fall back to phosphor for any odd glyph users paste in.
    serif.push("egui-phosphor-Regular".into());

    let serif_italic = fonts
        .families
        .entry(FontFamily::Name("serif_italic".into()))
        .or_default();
    serif_italic.insert(0, "spectral_italic".into());
    serif_italic.push("spectral".into());

    ctx.set_fonts(fonts);
}

/// Register Attune's text scale on top of egui's defaults so `Heading`,
/// `Body`, etc. show in the right font + size.
pub fn apply_text_styles(style: &mut Style) {
    style
        .text_styles
        .insert(egui::TextStyle::Heading, TextStyle::Heading.font_id());
    style
        .text_styles
        .insert(egui::TextStyle::Body, TextStyle::Body.font_id());
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, TextStyle::Mono.font_id());
    style
        .text_styles
        .insert(egui::TextStyle::Button, TextStyle::Body.font_id());
    style
        .text_styles
        .insert(egui::TextStyle::Small, TextStyle::Caption.font_id());
}
