//! Reusable UI components. Every screen composes from these.
//!
//! Components read tokens from `crate::design` and never invent their own
//! colors, spacing, or text sizes. Anything visual that's not here is a
//! signal we either need a new component or to extend an existing one.

// Some components (primary_button, ghost_button, vertical_divider, etc.) are
// part of the public component vocabulary but not yet used by today's
// screens. Keeping them available for future work.
#![allow(dead_code, unused_imports)]

pub mod audio_player;
pub mod button;
pub mod divider;
pub mod empty_state;
pub mod nav_item;
pub mod record_button;
pub mod section;
pub mod status_pill;
pub mod surface;
pub mod text;
pub mod text_input;

pub use audio_player::{audio_player, AudioPlayerAction};
pub use button::{ghost_button, ghost_button_icon, primary_button, secondary_button};
pub use divider::{divider, vertical_divider};
pub use empty_state::empty_state;
pub use nav_item::nav_item;
pub use record_button::record_button;
pub use section::{labeled_section, section_header};
pub use status_pill::status_pill;
pub use surface::card;
pub use text::{body, body_strong, caption, heading, micro, mono, mono_small, title};
pub use text_input::{mono_input, password_input, text_input};
