#![doc = include_str!("../readme.md")]

mod crossterm;
pub mod input;
pub mod masked_input;
pub mod util;

pub use pure_rust_locales::Locale;

/// Result value for event-handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The given event was not handled at all.
    Unused,
    /// The event was handled, no repaint necessary.
    Unchanged,
    /// The event was handled, repaint necessary.
    Changed,
}

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
