#![doc = include_str!("../readme.md")]

pub mod button;
pub mod date_input;
pub mod input;
pub mod masked_input;
pub mod util;

pub use pure_rust_locales::Locale;

pub mod event {
    pub use rat_event::{FocusKeys, HandleEvent, MouseOnly};
}

/// Result type for event-handling. Used by widgets in this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The given event was not handled at all.
    NotUsed,
    /// The event was handled, no repaint necessary.
    Unchanged,
    /// The event was handled, repaint necessary.
    Changed,
}

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
