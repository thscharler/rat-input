//!
//! Widgets for text-input based on ratatui.
//!

mod crossterm;
pub mod input;
pub mod util;

pub use pure_rust_locales::Locale;

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
