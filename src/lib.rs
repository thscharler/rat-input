#![doc = include_str!("../readme.md")]

pub mod button;
mod crossterm;
pub mod events;
pub mod input;
pub mod masked_input;
pub mod util;

pub use pure_rust_locales::Locale;

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
