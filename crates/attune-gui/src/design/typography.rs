//! Typography setup. Loads bundled Inter (UI) and JetBrains Mono (mono)
//! fonts, registers an icon font from `egui_phosphor`, and defines a small
//! semantic text-style scale.
//!
//! Component code never references a `FontId` directly. It picks a
//! [`TextStyle`] (Display, Title, Heading, …) and lets the style supply
//! the size, family, and line spacing.

use egui::{FontData, FontDefinitions, FontFamily, FontId, Style};

/// Semantic text styles. The numeric values come from a 1.25 type scale
/// rooted at 13 px body.
#[derive(Clone, Copy, Debug)]
pub enum TextStyle {
    /// Hero-sized text. Rare. Used for the recording timer and similar.
    Display,
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
    /// Smallest text. Footer disclaimers, fine print.
    Micro,
}

impl TextStyle {
    pub fn font_id(self) -> FontId {
        match self {
            TextStyle::Display => FontId::new(34.0, FontFamily::Proportional),
            TextStyle::Title => FontId::new(22.0, FontFamily::Proportional),
            TextStyle::Heading => FontId::new(16.0, FontFamily::Proportional),
            TextStyle::Body => FontId::new(13.0, FontFamily::Proportional),
            TextStyle::BodyStrong => FontId::new(13.0, FontFamily::Proportional),
            TextStyle::Mono => FontId::new(13.0, FontFamily::Monospace),
            TextStyle::MonoSmall => FontId::new(11.5, FontFamily::Monospace),
            TextStyle::Caption => FontId::new(11.5, FontFamily::Proportional),
            TextStyle::Micro => FontId::new(10.5, FontFamily::Proportional),
        }
    }
}

const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.otf");
const INTER_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Inter-Medium.otf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.otf");
const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
const JETBRAINS_MONO_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Medium.ttf");

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Bundled UI / mono fonts.
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

    // Phosphor icon font lives on its own family so we can switch between
    // text and icons inline without affecting glyph fallback for normal text.
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    // Proportional family stack — bias toward Inter, fall back to whatever
    // egui's defaults provide (then the icon font for glyph fallback).
    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.insert(0, "inter".into());
    prop.insert(1, "inter_medium".into());
    prop.insert(2, "inter_semibold".into());

    // Mono family stack — JetBrains Mono first.
    let mono = fonts.families.entry(FontFamily::Monospace).or_default();
    mono.insert(0, "jbmono".into());
    mono.insert(1, "jbmono_medium".into());

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
