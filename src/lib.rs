#![doc = include_str!("../readme.md")]

pub mod button;
pub mod calendar;
pub mod date_input;
pub mod input;
pub mod layout_dialog;
pub mod layout_edit;
pub mod masked_input;
pub mod menuline;
pub mod msgdialog;
pub mod statusline;
pub mod textarea;
pub mod util;

pub use pure_rust_locales::Locale;

pub mod event {
    pub use rat_event::util::Outcome;
    pub use rat_event::{FocusKeys, HandleEvent, MouseOnly, UsedEvent};
}

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
