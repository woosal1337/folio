//! Typography helpers. One function per semantic text style. Always returns
//! a `RichText` so callers can compose further (.color, .strong, etc.) but
//! the defaults are already production-correct.

use egui::RichText;

use crate::design::{palette, TextStyle};

pub fn title<S: Into<String>>(text: S) -> RichText {
    RichText::new(text)
        .font(TextStyle::Title.font_id())
        .color(palette::current().text)
}

pub fn heading<S: Into<String>>(text: S) -> RichText {
    RichText::new(text)
        .font(TextStyle::Heading.font_id())
        .color(palette::current().text)
}

pub fn body<S: Into<String>>(text: S) -> RichText {
    RichText::new(text)
        .font(TextStyle::Body.font_id())
        .color(palette::current().text)
}

pub fn body_strong<S: Into<String>>(text: S) -> RichText {
    RichText::new(text)
        .font(TextStyle::BodyStrong.font_id())
        .strong()
        .color(palette::current().text)
}

pub fn mono<S: Into<String>>(text: S) -> RichText {
    RichText::new(text)
        .font(TextStyle::Mono.font_id())
        .color(palette::current().text)
}

pub fn mono_small<S: Into<String>>(text: S) -> RichText {
    RichText::new(text)
        .font(TextStyle::MonoSmall.font_id())
        .color(palette::current().text_muted)
}

pub fn caption<S: Into<String>>(text: S) -> RichText {
    RichText::new(text)
        .font(TextStyle::Caption.font_id())
        .color(palette::current().text_muted)
}

pub fn micro<S: Into<String>>(text: S) -> RichText {
    RichText::new(text)
        .font(TextStyle::Micro.font_id())
        .color(palette::current().text_subtle)
}
