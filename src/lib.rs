#![doc = include_str!("../readme.md")]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::if_same_then_else)]

pub mod button;
pub mod calendar;
pub mod date_input;
pub mod fill;
pub mod input;
pub mod layout_dialog;
pub mod layout_edit;
pub mod layout_grid;
pub mod masked_input;
pub mod menubar;
pub mod menuline;
pub mod msgdialog;
pub mod number_input;
pub mod popup_menu;
pub mod statusline;
pub mod textarea;
pub mod util;

pub use pure_rust_locales::Locale;

pub mod event {
    //!
    //! Event-handler traits and Keybindings.
    //!

    pub use rat_event::{
        crossterm, ct_event, flow, flow_ok, util, ConsumedEvent, FocusKeys, HandleEvent, MouseOnly,
        Outcome,
    };

    /// Runs only the event-handling for the popup-parts of a widget.
    /// These should be run before the standard `FocusKey` or `MouseOnly` event-handlers,
    /// to mitigate the front/back problem of overlaying widgets.
    ///
    /// There is no separate `MouseOnlyPopup`, as popups should always have the
    /// input focus.
    #[derive(Debug)]
    pub struct Popup;

    /// Runs only the navigation events, not any editing.
    #[derive(Debug)]
    pub struct ReadOnly;

    /// Result of event handling.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum TextOutcome {
        /// The given event has not been used at all.
        NotUsed,
        /// The event has been recognized, but the result was nil.
        /// Further processing for this event may stop.
        Unchanged,
        /// The event has been recognized and there is some change
        /// due to it.
        /// Further processing for this event may stop.
        /// Rendering the ui is advised.
        Changed,
        /// Text content has changed.
        TextChanged,
    }

    impl ConsumedEvent for TextOutcome {
        fn is_consumed(&self) -> bool {
            *self != TextOutcome::NotUsed
        }
    }

    // Useful for converting most navigation/edit results.
    impl From<bool> for TextOutcome {
        fn from(value: bool) -> Self {
            if value {
                TextOutcome::Changed
            } else {
                TextOutcome::Unchanged
            }
        }
    }

    impl From<TextOutcome> for Outcome {
        fn from(value: TextOutcome) -> Self {
            match value {
                TextOutcome::NotUsed => Outcome::NotUsed,
                TextOutcome::Unchanged => Outcome::Unchanged,
                TextOutcome::Changed => Outcome::Changed,
                TextOutcome::TextChanged => Outcome::Changed,
            }
        }
    }
}

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
