use crate::_private::NonExhaustive;
use crate::event::{ReadOnly, TextOutcome};
use crate::masked_input::{MaskedInput, MaskedInputState, MaskedInputStyle};
use format_num_pattern::{NumberFmtError, NumberFormat, NumberSymbols};
use log::debug;
use rat_event::{FocusKeys, HandleEvent, MouseOnly};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Style;
use ratatui::widgets::{Block, StatefulWidget};
use std::fmt::{Debug, Display, LowerExp};
use std::str::FromStr;

/// Numeric input.
///
/// Uses [format_num_pattern](https://docs.rs/format_num_pattern/latest/format_num_pattern/index.html)
/// for the actual formatting/parsing and [MaskedInput] for the rendering.
///
#[derive(Debug, Default, Clone)]
pub struct NumberInput<'a> {
    widget: MaskedInput<'a>,
}

#[derive(Debug, Clone)]
pub struct NumberInputState {
    pub widget: MaskedInputState,
    /// NumberFormat pattern.
    pattern: String,
    /// Locale
    locale: format_num_pattern::Locale,
    // MaskedInput internally always works with the POSIX locale.
    // So don't be surprised, if you see that one instead of the
    // paramter locale used here.
    format: NumberFormat,

    pub non_exhaustive: NonExhaustive,
}

impl<'a> NumberInput<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the compact form, if the focus is not with this widget.
    #[inline]
    pub fn show_compact(mut self, show_compact: bool) -> Self {
        self.widget = self.widget.show_compact(show_compact);
        self
    }

    /// Set the combined style.
    #[inline]
    pub fn styles(mut self, style: MaskedInputStyle) -> Self {
        self.widget = self.widget.styles(style);
        self
    }

    /// Base text style.
    #[inline]
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.widget = self.widget.style(style);
        self
    }

    /// Style when focused.
    #[inline]
    pub fn focus_style(mut self, style: impl Into<Style>) -> Self {
        self.widget = self.widget.focus_style(style);
        self
    }

    /// Style for selection
    #[inline]
    pub fn select_style(mut self, style: impl Into<Style>) -> Self {
        self.widget = self.widget.select_style(style);
        self
    }

    /// Style for the invalid indicator.
    #[inline]
    pub fn invalid_style(mut self, style: impl Into<Style>) -> Self {
        self.widget = self.widget.invalid_style(style);
        self
    }

    #[inline]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.widget = self.widget.block(block);
        self
    }

    /// Renders the content differently if focused.
    ///
    /// * Selection is only shown if focused.
    ///
    #[inline]
    pub fn focused(mut self, focused: bool) -> Self {
        self.widget = self.widget.focused(focused);
        self
    }

    /// Renders the content differently if invalid.
    /// Uses the invalid style instead of the base style for rendering.
    #[inline]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.widget = self.widget.invalid(invalid);
        self
    }
}

impl<'a> StatefulWidget for NumberInput<'a> {
    type State = NumberInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.widget.render(area, buf, &mut state.widget);
    }
}

impl Default for NumberInputState {
    fn default() -> Self {
        Self {
            widget: Default::default(),
            pattern: Default::default(),
            locale: Default::default(),
            format: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl NumberInputState {
    pub fn new<S: AsRef<str>>(pattern: S) -> Result<Self, NumberFmtError> {
        let mut s = Self::default();
        s.set_format(pattern)?;
        Ok(s)
    }

    pub fn new_loc<S: AsRef<str>>(
        pattern: S,
        locale: format_num_pattern::Locale,
    ) -> Result<Self, NumberFmtError> {
        let mut s = Self::default();
        s.set_format_loc(pattern.as_ref(), locale)?;
        Ok(s)
    }

    /// Reset to empty
    pub fn clear(&mut self) {
        self.widget.clear();
    }

    /// [format_num_pattern] format string.
    #[inline]
    pub fn format(&self) -> &str {
        self.pattern.as_str()
    }

    /// chrono locale.
    #[inline]
    pub fn locale(&self) -> chrono::Locale {
        self.locale
    }

    /// Set format.
    pub fn set_format<S: AsRef<str>>(&mut self, pattern: S) -> Result<(), NumberFmtError> {
        self.set_format_loc(pattern, format_num_pattern::Locale::default())
    }

    /// Set format and locale.
    pub fn set_format_loc<S: AsRef<str>>(
        &mut self,
        pattern: S,
        locale: format_num_pattern::Locale,
    ) -> Result<(), NumberFmtError> {
        let sym = NumberSymbols::monetary(locale);

        self.format = NumberFormat::new(pattern.as_ref())?;
        self.widget.set_mask(pattern.as_ref())?;
        self.widget.set_num_symbols(sym);

        Ok(())
    }

    pub fn value<T: FromStr>(&self) -> Result<T, NumberFmtError> {
        let s = self.widget.value();
        self.format.parse(s)
    }

    pub fn set_value<T: LowerExp + Display + Debug>(
        &mut self,
        number: T,
    ) -> Result<(), NumberFmtError> {
        debug!("set_value {:?}", number);
        debug!("format {:?}", self.format);
        let s = self.format.fmt(number)?;
        debug!("results {:?}", s);
        self.widget.set_value(s);
        Ok(())
    }

    /// Select all text.
    #[inline]
    pub fn select_all(&mut self) {
        self.widget.select_all();
    }

    /// Screen position of the cursor for rendering.
    #[inline]
    pub fn screen_cursor(&self) -> Option<(u16, u16)> {
        self.widget.screen_cursor()
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, TextOutcome> for NumberInputState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: FocusKeys) -> TextOutcome {
        self.widget.handle(event, FocusKeys)
    }
}

impl HandleEvent<crossterm::event::Event, ReadOnly, TextOutcome> for NumberInputState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: ReadOnly) -> TextOutcome {
        self.widget.handle(event, ReadOnly)
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, TextOutcome> for NumberInputState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: MouseOnly) -> TextOutcome {
        self.widget.handle(event, MouseOnly)
    }
}

/// Handle all events.
/// Text events are only processed if focus is true.
/// Mouse events are processed if they are in range.
pub fn handle_events(
    state: &mut NumberInputState,
    focus: bool,
    event: &crossterm::event::Event,
) -> TextOutcome {
    if focus {
        HandleEvent::handle(state, event, FocusKeys)
    } else {
        HandleEvent::handle(state, event, MouseOnly)
    }
}

/// Handle only navigation events.
/// Text events are only processed if focus is true.
/// Mouse events are processed if they are in range.
pub fn handle_readonly_events(
    state: &mut NumberInputState,
    focus: bool,
    event: &crossterm::event::Event,
) -> TextOutcome {
    if focus {
        state.handle(event, ReadOnly)
    } else {
        state.handle(event, MouseOnly)
    }
}

/// Handle only mouse-events.
pub fn handle_mouse_events(
    state: &mut NumberInputState,
    event: &crossterm::event::Event,
) -> TextOutcome {
    HandleEvent::handle(state, event, MouseOnly)
}
