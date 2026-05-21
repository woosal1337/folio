//! Spacing, radius, and motion tokens. Numeric values used by every visual
//! decision. Centralized so the entire app moves coherently when one number
//! changes.

/// 4 px scale. All spacing is a multiple of these tokens — no magic numbers
/// in component code.
pub struct Space;

impl Space {
    pub const fn x2s() -> f32 {
        2.0
    }
    pub const fn xs() -> f32 {
        4.0
    }
    pub const fn sm() -> f32 {
        8.0
    }
    pub const fn md() -> f32 {
        12.0
    }
    pub const fn lg() -> f32 {
        16.0
    }
    pub const fn xl() -> f32 {
        24.0
    }
    pub const fn x2l() -> f32 {
        32.0
    }
    pub const fn x3l() -> f32 {
        48.0
    }
    pub const fn x4l() -> f32 {
        64.0
    }
}

/// Corner radii. Components must not invent their own values.
pub struct Radius;

impl Radius {
    pub const fn xs() -> f32 {
        3.0
    }
    pub const fn sm() -> f32 {
        6.0
    }
    pub const fn md() -> f32 {
        8.0
    }
    pub const fn lg() -> f32 {
        12.0
    }
    pub const fn xl() -> f32 {
        16.0
    }
    /// Use only for pill-shaped elements where the radius equals half the
    /// shorter side of the element.
    pub const fn pill() -> f32 {
        999.0
    }
}

/// Animation durations in seconds.
pub struct Motion;

impl Motion {
    pub const fn instant() -> f32 {
        0.0
    }
    pub const fn fast() -> f32 {
        0.10
    }
    pub const fn medium() -> f32 {
        0.18
    }
    pub const fn slow() -> f32 {
        0.28
    }
}

/// Common layout constants.
pub struct Layout;

impl Layout {
    /// Width of the left sidebar in the main app shell.
    pub const fn sidebar_width() -> f32 {
        220.0
    }
    /// Maximum inner content width on wide windows. Keeps line lengths
    /// readable without forcing a fixed app size.
    pub const fn content_max_width() -> f32 {
        720.0
    }
    /// Header / status bar height.
    pub const fn header_height() -> f32 {
        56.0
    }
    /// Hairline weight for subtle separators.
    pub const fn hairline() -> f32 {
        1.0
    }
}
