//! Text input with an input mask.
//!
//! * Can do the usual insert/delete/move operations.
//! * Text selection with keyboard + mouse
//! * Scrolls with the cursor.
//! * Modes for focus and valid.
//! * Info-overlay for sub-fields without value.
//! * Partially invalid input is possible.
//! * Localization with [format_num_pattern::NumberSymbols]
//!
//! * Accepts an input mask:
//!   * `0`: can enter digit, display as 0
//!   * `9`: can enter digit, display as space
//!   * `#`: digit, plus or minus sign, display as space
//!   * `-`: sign
//!   * `+`: sign, positive is '+', negative is '-', not localized.
//!   * `.` and `,`: decimal and grouping separators
//!
//!   * `H`: must enter a hex digit, display as 0
//!   * `h`: can enter a hex digit, display as space
//!   * `O`: must enter an octal digit, display as 0
//!   * `o`: can enter an octal digit, display as space
//!   * `D`: must enter a decimal digit, display as 0
//!   * `d`: can enter a decimal digit, display as space
//!
//!   * `l`: can enter letter, display as space
//!   * `a`: can enter letter or digit, display as space
//!   * `c`: can enter character or space, display as space
//!   * `_`: anything, display as space
//!
//!   * `<space>`, `:`, `;`, `/`: separator characters move the cursor when entered.
//!   * `\`: escapes the following character and uses it as a separator.
//!   * all other ascii characters a reserved.
//!   * other unicode characters can be used as separators without escaping.
//!
//! * Accepts a display overlay used instead of the default chars of the input mask.
//!
//! ```no_run rust
//! use ratatui::widgets::StatefulWidget;
//! use rat_input::masked_input::{MaskedInput, MaskedInputState};
//!
//! let mut date_state = MaskedInputState::new();
//! date_state.set_mask("99/99/9999");
//! date_state.set_display_mask("mm/dd/yyyy");
//!
//! let w_date = MaskedInput::default();
//! w_date.render(area, buf, &mut date_state);
//! if date_focused {
//!     frame.set_cursor(date_state.cursor.x, date_state.cursor.y);
//! }
//!
//! let mut creditcard_state = MaskedInputState::new();
//! creditcard_state.set_mask("dddd dddd dddd dddd");
//!
//! let w_creditcard = MaskedInput::default();
//! w_creditcard.render(area, buf, &mut creditcard_state);
//! if creditcard_focused {
//!     frame.set_cursor(creditcard_state.cursor.x, creditcard_state.cursor.y);
//! }
//!
//! ```
//!
//! Event handling by calling the freestanding fn [handle_events].
//! There's [handle_mouse_events] if you want to override the default key bindings but keep
//! the mouse behaviour.
//!

use crate::_private::NonExhaustive;
use crate::masked_input::core::InputMaskCore;
use crate::util::clamp_shift;
use crate::util::MouseFlags;
use crate::{ct_event, util, Outcome};
use format_num_pattern::NumberSymbols;
#[allow(unused_imports)]
use log::debug;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::{BlockExt, Stylize};
use ratatui::style::Style;
use ratatui::widgets::{Block, StatefulWidget, StatefulWidgetRef, WidgetRef};
use std::fmt;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Text input widget with input mask.
#[derive(Debug)]
pub struct MaskedInput<'a> {
    show_compact: bool,
    block: Option<Block<'a>>,
    style: Style,
    focus_style: Style,
    select_style: Style,
    invalid_style: Style,
    focused: bool,
    valid: bool,
}

/// Combined style.
#[derive(Debug)]
pub struct MaskedInputStyle {
    pub style: Style,
    pub focus: Style,
    pub select: Style,
    pub invalid: Style,
    pub non_exhaustive: NonExhaustive,
}

impl Default for MaskedInputStyle {
    fn default() -> Self {
        Self {
            style: Default::default(),
            focus: Default::default(),
            select: Default::default(),
            invalid: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl<'a> Default for MaskedInput<'a> {
    fn default() -> Self {
        Self {
            show_compact: false,
            block: None,
            style: Default::default(),
            focus_style: Default::default(),
            select_style: Default::default(),
            invalid_style: Style::default().red().underlined(),
            focused: true,
            valid: true,
        }
    }
}

impl<'a> MaskedInput<'a> {
    /// Show the compact form, if the focus is not with this widget.
    pub fn show_compact(mut self, show_compact: bool) -> Self {
        self.show_compact = show_compact;
        self
    }

    /// Set the combined style.
    pub fn style(mut self, style: MaskedInputStyle) -> Self {
        self.style = style.style;
        self.focus_style = style.focus;
        self.select_style = style.select;
        self.invalid_style = style.invalid;
        self
    }

    /// Base text style.
    pub fn base_style(mut self, style: impl Into<Style>) -> Self {
        self.style = style.into();
        self
    }

    /// Style when focused.
    pub fn focus_style(mut self, style: impl Into<Style>) -> Self {
        self.focus_style = style.into();
        self
    }

    /// Style for selection
    pub fn select_style(mut self, style: impl Into<Style>) -> Self {
        self.select_style = style.into();
        self
    }

    /// Style for the invalid indicator.
    pub fn invalid_style(mut self, style: impl Into<Style>) -> Self {
        self.invalid_style = style.into();
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Renders the content differently if focused.
    ///
    /// * Selection is only shown if focused.
    ///
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Renders the content differently if invalid.
    /// Uses the invalid style instead of the base style for rendering.
    pub fn valid(mut self, valid: bool) -> Self {
        self.valid = valid;
        self
    }
}

impl<'a> StatefulWidget for MaskedInput<'a> {
    type State = MaskedInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidgetRef::render_ref(&self, area, buf, state);
    }
}

impl<'a> StatefulWidgetRef for MaskedInput<'a> {
    type State = MaskedInputState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = self.block.inner_if_some(area);
        state.value.set_width(state.area.width as usize);

        self.block.render_ref(area, buf);

        if self.focused {
            state.value.render_value();
        } else {
            if self.show_compact {
                state.value.render_condensed_value();
            } else {
                state.value.render_value();
            }
        }

        let (style, select_style, invalid_style, invalid_select_style) = if self.focused {
            (
                self.focus_style,
                self.select_style,
                self.focus_style.patch(self.invalid_style),
                self.select_style.patch(self.invalid_style),
            )
        } else {
            (
                self.style,
                self.style,
                self.style.patch(self.invalid_style),
                self.style.patch(self.invalid_style),
            )
        };

        let area = state.area.intersection(buf.area);

        let selection = clamp_shift(state.selection(), state.offset(), state.width());

        let mut cit = state.value.rendered().graphemes(true).skip(state.offset());
        for col in 0..area.width as usize {
            let cell = buf.get_mut(area.x + col as u16, area.y);
            if let Some(c) = cit.next() {
                cell.set_symbol(c);
            } else {
                cell.set_char(' ');
            }

            let valid = if let Some(valid_mask) = &state.valid_mask {
                valid_mask
                    .get(state.offset() + col)
                    .copied()
                    .unwrap_or(true)
            } else {
                self.valid
            };

            if selection.contains(&col) {
                if self.valid {
                    cell.set_style(select_style);
                } else {
                    cell.set_style(invalid_select_style);
                }
            } else {
                if valid {
                    cell.set_style(style);
                } else {
                    cell.set_style(invalid_style);
                }
            }
        }

        state.cursor = Position::new(state.area.x + state.visible_cursor(), state.area.y)
    }
}

/// State of the input-mask.
#[derive(Debug, Clone)]
pub struct MaskedInputState {
    /// The position of the cursor in screen coordinates.
    /// Can be directly used for [Frame::set_cursor()]
    pub cursor: Position,
    /// Area
    pub area: Rect,
    /// Mouse selection in progress.
    pub mouse: MouseFlags,
    /// Valid mask. Which characters of the input are valid/not valid.
    pub valid_mask: Option<Vec<bool>>,
    /// Editing core.
    pub value: InputMaskCore,
    /// Construct with `..Default::default()`
    pub non_exhaustive: NonExhaustive,
}

impl Default for MaskedInputState {
    fn default() -> Self {
        Self {
            cursor: Default::default(),
            area: Default::default(),
            mouse: Default::default(),
            valid_mask: None,
            value: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

/// Handle all events.
/// Text events are only processed if focus is true.
/// Mouse events are processed if they are in range.
pub fn handle_events(
    state: &mut MaskedInputState,
    focus: bool,
    event: &crossterm::event::Event,
) -> Result<Outcome, fmt::Error> {
    let r = 'f: {
        if focus {
            match event {
                ct_event!(keycode press Left) => state.move_to_prev(false),
                ct_event!(keycode press Right) => state.move_to_next(false),
                ct_event!(keycode press CONTROL-Left) => {
                    let pos = state.prev_word_boundary();
                    state.set_cursor(pos, false);
                }
                ct_event!(keycode press CONTROL-Right) => {
                    let pos = state.next_word_boundary();
                    state.set_cursor(pos, false);
                }
                ct_event!(keycode press Home) => state.set_cursor(0, false),
                ct_event!(keycode press End) => state.set_cursor(state.len(), false),
                ct_event!(keycode press SHIFT-Left) => state.move_to_prev(true),
                ct_event!(keycode press SHIFT-Right) => state.move_to_next(true),
                ct_event!(keycode press CONTROL_SHIFT-Left) => {
                    let pos = state.prev_word_boundary();
                    state.set_cursor(pos, true);
                }
                ct_event!(keycode press CONTROL_SHIFT-Right) => {
                    let pos = state.next_word_boundary();
                    state.set_cursor(pos, true);
                }
                ct_event!(keycode press SHIFT-Home) => state.set_cursor(0, true),
                ct_event!(keycode press SHIFT-End) => state.set_cursor(state.len(), true),
                ct_event!(key press CONTROL-'a') => state.set_selection(0, state.len()),
                ct_event!(keycode press Backspace) => state.delete_prev_char()?,
                ct_event!(keycode press Delete) => state.delete_next_char()?,
                ct_event!(keycode press CONTROL-Backspace) => {
                    let prev = state.prev_word_boundary();
                    state.remove_selection(prev..state.cursor())?;
                }
                ct_event!(keycode press CONTROL-Delete) => {
                    let next = state.next_word_boundary();
                    state.remove_selection(state.cursor()..next)?;
                }
                ct_event!(key press CONTROL-'d') => state.set_value(""),
                ct_event!(keycode press CONTROL_SHIFT-Backspace) => {
                    state.remove_selection(0..state.cursor())?
                }
                ct_event!(keycode press CONTROL_SHIFT-Delete) => {
                    state.remove_selection(state.cursor()..state.len())?
                }
                ct_event!(key press c) | ct_event!(key press SHIFT-c) => state.insert_char(*c)?,
                _ => break 'f Outcome::Unused,
            }
            Outcome::Changed
        } else {
            Outcome::Unchanged
        }
    };

    match r {
        Outcome::Unused => handle_mouse_events(state, event),
        v => Ok(v),
    }
}

/// Handle only mouse-events.
pub fn handle_mouse_events(
    state: &mut MaskedInputState,
    event: &crossterm::event::Event,
) -> Result<Outcome, fmt::Error> {
    let r = match event {
        ct_event!(mouse down Left for column,row) => {
            if state.area.contains(Position::new(*column, *row)) {
                state.mouse.set_drag();
                let c = column - state.area.x;
                state.set_offset_relative_cursor(c as isize, false);
                Outcome::Changed
            } else {
                Outcome::Unused
            }
        }
        ct_event!(mouse drag Left for column, _row) => {
            if state.mouse.do_drag() {
                let c = (*column as isize) - (state.area.x as isize);
                state.set_offset_relative_cursor(c, true);
                Outcome::Changed
            } else {
                Outcome::Unused
            }
        }
        ct_event!(mouse moved) => {
            state.mouse.clear_drag();
            Outcome::Unused
        }
        _ => Outcome::Unused,
    };

    Ok(r)
}

impl MaskedInputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_symbols(sym: NumberSymbols) -> Self {
        Self {
            value: InputMaskCore::new_with_symbols(sym),
            ..Self::default()
        }
    }

    /// Reset to empty.
    pub fn reset(&mut self) {
        self.value.reset();
    }

    /// Offset shown.
    pub fn offset(&self) -> usize {
        self.value.offset()
    }

    /// Offset shown. This is corrected if the cursor wouldn't be visible.
    pub fn set_offset(&mut self, offset: usize) {
        self.value.set_offset(offset);
    }

    /// Display width.
    pub fn width(&self) -> usize {
        self.value.width()
    }

    /// Display width
    pub fn set_width(&mut self, width: usize) {
        self.value.set_width(width);
    }

    /// Set the cursor position, reset selection.
    pub fn set_cursor(&mut self, cursor: usize, extend_selection: bool) {
        self.value.set_cursor(cursor, extend_selection);
    }

    /// Place cursor at decimal separator, if any. 0 otherwise.
    pub fn set_default_cursor(&mut self) {
        self.value.set_default_cursor();
    }

    /// Cursor position
    pub fn cursor(&self) -> usize {
        self.value.cursor()
    }

    /// Set the display mask. This text is used for parts that have
    /// no valid input yet. Part means consecutive characters of the
    /// input mask with the same mask type.
    ///
    /// There is a default representation for each mask type if this
    /// is not set.
    ///
    /// If the length differs from the mask, the difference will be
    /// ignored / filled with defaults.
    pub fn set_display_mask<S: Into<String>>(&mut self, s: S) {
        self.value.set_display_mask(s);
    }

    /// Display mask.
    pub fn display_mask(&self) -> String {
        self.value.display_mask()
    }

    /// Sets a mask of valid/invalid characters.
    pub fn set_valid_mask(&mut self, v: Option<Vec<bool>>) {
        self.valid_mask = v;
    }

    /// Mask of valid/invalid characters.
    pub fn valid_mask(&self) -> &Option<Vec<bool>> {
        &self.valid_mask
    }

    /// Set the input mask. This overwrites the display mask and the value
    /// with a default representation of the mask.
    ///
    /// The result value contains all punctuation and
    /// the value given as 'display' below. See [compact_value()](MaskedInputState::compact_value).
    ///
    /// * `0`: can enter digit, display as 0
    /// * `9`: can enter digit, display as space
    /// * `#`: digit, plus or minus sign, display as space
    /// * `+`: sign. display '+' for positive
    /// * `-`: sign. display ' ' for positive
    /// * `.` and `,`: decimal and grouping separators
    ///
    /// * `H`: must enter a hex digit, display as 0
    /// * `h`: can enter a hex digit, display as space
    /// * `O`: must enter an octal digit, display as 0
    /// * `o`: can enter an octal digit, display as space
    /// * `D`: must enter a decimal digit, display as 0
    /// * `d`: can enter a decimal digit, display as space
    ///
    /// * `l`: can enter letter, display as space
    /// * `a`: can enter letter or digit, display as space
    /// * `c`: can enter character or space, display as space
    /// * `_`: anything, display as space
    ///
    /// * `:` `;` `-` `/`: separator characters move the cursor when entered.
    /// * `\`: escapes the following character and uses it as a separator.
    /// * all other ascii characters a reserved.
    ///
    /// Inspired by <https://support.microsoft.com/en-gb/office/control-data-entry-formats-with-input-masks-e125997a-7791-49e5-8672-4a47832de8da>
    pub fn set_mask<S: AsRef<str>>(&mut self, s: S) -> Result<(), fmt::Error> {
        self.value.set_mask(s)
    }

    /// Display mask.
    pub fn mask(&self) -> String {
        self.value.mask()
    }

    /// Mask with some debug information.
    pub fn debug_mask(&self) -> String {
        self.value.debug_mask()
    }

    /// Set symbols for number display.
    ///
    /// These are only used for rendering and to map user input.
    /// The value itself uses ".", "," and "-".
    pub fn set_num_symbols(&mut self, sym: NumberSymbols) {
        self.value.set_num_symbols(sym);
    }

    /// Set the value.
    ///
    /// No checks if the value conforms to the mask.
    /// If the value is too short it will be filled with space.
    /// if the value is too long it will be truncated.
    pub fn set_value<S: Into<String>>(&mut self, s: S) {
        self.value.set_value(s);
    }

    /// Value with all punctuation and default values according to the mask type.
    pub fn value(&self) -> &str {
        self.value.value()
    }

    /// Value without optional whitespace and grouping separators. Might be easier to parse.
    pub fn compact_value(&self) -> String {
        self.value.compact_value()
    }

    /// Value.
    pub fn as_str(&self) -> &str {
        self.value.value()
    }

    ///
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Length in grapheme count.
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// Selection
    pub fn has_selection(&self) -> bool {
        self.value.has_selection()
    }

    /// Selection
    pub fn set_selection(&mut self, anchor: usize, cursor: usize) {
        self.value.set_cursor(anchor, false);
        self.value.set_cursor(cursor, true);
    }

    /// Selection
    pub fn select_all(&mut self) {
        // the other way round it fails if width is 0.
        self.value.set_cursor(self.value.len(), false);
        self.value.set_cursor(0, true);
    }

    /// Selection
    pub fn selection(&self) -> Range<usize> {
        self.value.selection()
    }

    /// Selection
    pub fn selection_str(&self) -> &str {
        util::split3(self.value.value(), self.value.selection()).1
    }

    /// Set the cursor position from a visual position relative to the origin.
    pub fn set_offset_relative_cursor(&mut self, rpos: isize, extend_selection: bool) {
        let pos = if rpos < 0 {
            self.value.offset().saturating_sub(-rpos as usize)
        } else {
            self.value.offset() + rpos as usize
        };
        self.value.set_cursor(pos, extend_selection);
    }

    /// The current text cursor as a absolute screen position.
    pub fn visual_cursor(&self) -> Position {
        self.cursor
    }

    /// Previous word boundary.
    pub fn prev_word_boundary(&self) -> usize {
        self.value.prev_word_boundary()
    }

    /// Next word boundary.
    pub fn next_word_boundary(&self) -> usize {
        self.value.next_word_boundary()
    }

    /// Move to the next char.
    pub fn move_to_next(&mut self, extend_selection: bool) {
        if !extend_selection && self.value.has_selection() {
            let c = self.value.selection().end;
            self.value.set_cursor(c, false);
        } else if self.value.cursor() < self.value.len() {
            self.value
                .set_cursor(self.value.cursor() + 1, extend_selection);
        }
    }

    /// Move to the previous char.
    pub fn move_to_prev(&mut self, extend_selection: bool) {
        if !extend_selection && self.value.has_selection() {
            let c = self.value.selection().start;
            self.value.set_cursor(c, false);
        } else if self.value.cursor() > 0 {
            self.value
                .set_cursor(self.value.cursor() - 1, extend_selection);
        }
    }

    /// Insert a char at the current position.
    pub fn insert_char(&mut self, c: char) -> Result<(), fmt::Error> {
        if self.value.has_selection() {
            self.value.remove_selection(self.value.selection())?;
        }
        self.value.advance_cursor(c);
        self.value.insert_char(c)?;
        Ok(())
    }

    /// Remove the selected range. The text will be replaced with the default value
    /// as defined by the mask.
    pub fn remove_selection(&mut self, selection: Range<usize>) -> Result<(), fmt::Error> {
        self.value.remove_selection(selection)
    }

    /// Delete the char before the cursor.
    pub fn delete_prev_char(&mut self) -> Result<(), fmt::Error> {
        if self.value.is_select_all() {
            self.value.reset();
        } else if self.value.has_selection() {
            self.value.remove_selection(self.value.selection())?;
        } else if self.value.cursor() > 0 {
            self.value.remove_prev()?;
        }
        Ok(())
    }

    /// Delete the char after the cursor.
    pub fn delete_next_char(&mut self) -> Result<(), fmt::Error> {
        if self.value.is_select_all() {
            self.value.reset();
        } else if self.value.has_selection() {
            self.value.remove_selection(self.value.selection())?;
        } else if self.value.cursor() < self.value.len() {
            self.value.remove_next()?;
        }
        Ok(())
    }

    /// Visible cursor position.
    fn visible_cursor(&mut self) -> u16 {
        (self.value.cursor() - self.value.offset()) as u16
    }
}

pub mod core {
    use crate::util;
    use format_num_pattern as number;
    use format_num_pattern::{CurrencySym, NumberFormat, NumberSymbols};
    #[allow(unused_imports)]
    use log::debug;
    use std::fmt::{Debug, Display, Formatter};
    use std::iter::{once, repeat_with};
    use std::ops::Range;
    use std::{fmt, mem};
    use unicode_segmentation::UnicodeSegmentation;

    /// Edit direction for part of a mask.
    /// Numeric values can switch between right-to-left (integer part) and left-to-right (fraction).
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum EditDirection {
        Ltor,
        Rtol,
    }

    /// One char of the input mask.
    #[allow(variant_size_differences)]
    #[derive(Clone, PartialEq, Eq, Default)]
    #[non_exhaustive]
    pub enum Mask {
        Digit0(EditDirection),
        Digit(EditDirection),
        Numeric(EditDirection),
        DecimalSep,
        GroupingSep,
        Sign,
        Plus,
        Hex0,
        Hex,
        Oct0,
        Oct,
        Dec0,
        Dec,
        Letter,
        LetterOrDigit,
        LetterDigitSpace,
        AnyChar,
        Separator(Box<str>),
        #[default]
        None,
    }

    /// One token of the input mask.
    ///
    /// Which field of the mask does this belong to:
    /// * Number with integer part, decimal separator, fraction and sign.
    /// * Consecutive mask parts of similar type.
    ///
    /// At this cursor position, what is the main mask (right) and what is possible left of
    /// the cursor position (peek_left).
    ///
    /// Default-values for editing and display.
    #[derive(Clone, PartialEq, Eq)]
    #[non_exhaustive]
    pub struct MaskToken {
        pub nr_id: usize,
        pub nr_start: usize,
        pub nr_end: usize,

        pub sec_id: usize,
        pub sec_start: usize,
        pub sec_end: usize,

        pub peek_left: Mask,
        pub right: Mask,
        pub edit: Box<str>,
        pub display: Box<str>,
    }

    impl Debug for EditDirection {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    EditDirection::Ltor => ">",
                    EditDirection::Rtol => "<",
                }
            )
        }
    }

    impl Display for Mask {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let s = match self {
                Mask::Digit0(_) => "0",
                Mask::Digit(_) => "9",
                Mask::Numeric(_) => "#",
                Mask::DecimalSep => ".",
                Mask::GroupingSep => ",",
                Mask::Sign => "-",
                Mask::Plus => "+",
                Mask::Hex0 => "H",
                Mask::Hex => "h",
                Mask::Oct0 => "O",
                Mask::Oct => "o",
                Mask::Dec0 => "D",
                Mask::Dec => "d",
                Mask::Letter => "l",
                Mask::LetterOrDigit => "a",
                Mask::LetterDigitSpace => "c",
                Mask::AnyChar => "_",
                Mask::Separator(s) => {
                    if matches!(
                        s.as_ref(),
                        "0" | "9"
                            | "#"
                            | "."
                            | ","
                            | "-"
                            | "+"
                            | "H"
                            | "h"
                            | "O"
                            | "o"
                            | "D"
                            | "d"
                            | "l"
                            | "a"
                            | "c"
                            | "_"
                    ) {
                        write!(f, "\\")?;
                    }
                    s
                }
                Mask::None => "",
            };
            write!(f, "{}", s)
        }
    }

    impl Debug for Mask {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Mask::Digit0(d) => {
                    write!(f, "{:?}0", d)
                }
                Mask::Digit(d) => {
                    write!(f, "{:?}9", d)
                }
                Mask::Numeric(d) => {
                    write!(f, "{:?}#", d)
                }
                Mask::DecimalSep => write!(f, "."),
                Mask::GroupingSep => write!(f, ","),
                Mask::Sign => write!(f, "-"),
                Mask::Plus => write!(f, "+"),
                Mask::Hex0 => write!(f, "H"),
                Mask::Hex => write!(f, "h"),
                Mask::Oct0 => write!(f, "O"),
                Mask::Oct => write!(f, "o"),
                Mask::Dec0 => write!(f, "D"),
                Mask::Dec => write!(f, "d"),
                Mask::Letter => write!(f, "l"),
                Mask::LetterOrDigit => write!(f, "a"),
                Mask::LetterDigitSpace => write!(f, "c"),
                Mask::AnyChar => write!(f, "_"),
                Mask::Separator(s) => {
                    if matches!(
                        s.as_ref(),
                        "0" | "9"
                            | "#"
                            | "."
                            | ","
                            | "-"
                            | "+"
                            | "H"
                            | "h"
                            | "O"
                            | "o"
                            | "D"
                            | "d"
                            | "l"
                            | "a"
                            | "c"
                            | "_"
                    ) {
                        write!(f, "\\")?;
                    }
                    write!(f, "{}", s)
                }
                Mask::None => write!(f, ""),
            }
        }
    }

    impl EditDirection {
        fn is_ltor(&self) -> bool {
            *self == EditDirection::Ltor
        }

        fn is_rtol(&self) -> bool {
            *self == EditDirection::Rtol
        }
    }

    impl Mask {
        /// is not editable. the last field of the mask at position txt.len() can not be edited,
        /// but it's a valid cursor position.
        fn is_none(&self) -> bool {
            *self == Mask::None
        }

        /// is a number mask
        fn is_number(&self) -> bool {
            match self {
                Mask::Digit0(_) => true,
                Mask::Digit(_) => true,
                Mask::Numeric(_) => true,
                Mask::DecimalSep => true,
                Mask::GroupingSep => true,
                Mask::Sign => true,
                Mask::Plus => true,

                Mask::Hex0 => false,
                Mask::Hex => false,
                Mask::Oct0 => false,
                Mask::Oct => false,
                Mask::Dec0 => false,
                Mask::Dec => false,
                Mask::Letter => false,
                Mask::LetterOrDigit => false,
                Mask::LetterDigitSpace => false,
                Mask::AnyChar => false,
                Mask::Separator(_) => false,
                Mask::None => false,
            }
        }

        /// left to right editing
        fn is_ltor(&self) -> bool {
            match self {
                Mask::Digit0(d) => d.is_ltor(),
                Mask::Digit(d) => d.is_ltor(),
                Mask::Numeric(d) => d.is_ltor(),
                Mask::GroupingSep => false,
                Mask::Sign => false,
                Mask::Plus => false,
                Mask::DecimalSep => true,
                Mask::Hex0 => true,
                Mask::Hex => true,
                Mask::Oct0 => true,
                Mask::Oct => true,
                Mask::Dec0 => true,
                Mask::Dec => true,
                Mask::Letter => true,
                Mask::LetterOrDigit => true,
                Mask::LetterDigitSpace => true,
                Mask::AnyChar => true,
                Mask::Separator(_) => true,
                Mask::None => false,
            }
        }

        /// right to left editing
        fn is_rtol(&self) -> bool {
            match self {
                Mask::Digit0(d) => d.is_rtol(),
                Mask::Digit(d) => d.is_rtol(),
                Mask::Numeric(d) => d.is_rtol(),
                Mask::GroupingSep => true,
                Mask::Sign => true,
                Mask::Plus => true,
                Mask::DecimalSep => false,
                Mask::Hex0 => false,
                Mask::Hex => false,
                Mask::Oct0 => false,
                Mask::Oct => false,
                Mask::Dec0 => false,
                Mask::Dec => false,
                Mask::Letter => false,
                Mask::LetterOrDigit => false,
                Mask::LetterDigitSpace => false,
                Mask::AnyChar => false,
                Mask::Separator(_) => false,
                Mask::None => false,
            }
        }

        fn is_fraction(&self) -> bool {
            match self {
                Mask::Digit0(d) => d.is_ltor(),
                Mask::Digit(d) => d.is_ltor(),
                Mask::Numeric(d) => d.is_ltor(),
                Mask::GroupingSep => false,
                Mask::Sign => false,
                Mask::Plus => false,
                Mask::DecimalSep => false,
                Mask::Hex0 => false,
                Mask::Hex => false,
                Mask::Oct0 => false,
                Mask::Oct => false,
                Mask::Dec0 => false,
                Mask::Dec => false,
                Mask::Letter => false,
                Mask::LetterOrDigit => false,
                Mask::LetterDigitSpace => false,
                Mask::AnyChar => false,
                Mask::Separator(_) => false,
                Mask::None => false,
            }
        }

        /// which mask-types are put together into one section.
        fn section(&self) -> u8 {
            match self {
                Mask::Digit0(_) => 0,
                Mask::Digit(_) => 0,
                Mask::Numeric(_) => 0,
                Mask::GroupingSep => 0,

                Mask::Sign => 1,

                Mask::Plus => 2,

                Mask::DecimalSep => 3,

                Mask::Hex0 => 4,
                Mask::Hex => 4,

                Mask::Oct0 => 5,
                Mask::Oct => 5,

                Mask::Dec0 => 6,
                Mask::Dec => 6,

                Mask::Letter => 7,
                Mask::LetterOrDigit => 7,
                Mask::LetterDigitSpace => 7,
                Mask::AnyChar => 7,

                Mask::Separator(_) => 8,

                Mask::None => 9,
            }
        }

        /// which mask-types constitute a number
        fn number(&self) -> u8 {
            match self {
                Mask::Digit0(_) => 0,
                Mask::Digit(_) => 0,
                Mask::Numeric(_) => 0,
                Mask::GroupingSep => 0,
                Mask::Sign => 0,
                Mask::Plus => 0,
                Mask::DecimalSep => 0,

                Mask::Hex0 => 1,
                Mask::Hex => 1,
                Mask::Oct0 => 1,
                Mask::Oct => 1,
                Mask::Dec0 => 1,
                Mask::Dec => 1,
                Mask::Letter => 1,
                Mask::LetterOrDigit => 1,
                Mask::LetterDigitSpace => 1,
                Mask::AnyChar => 1,

                Mask::Separator(_) => 2,
                Mask::None => 3,
            }
        }

        // mask should overwrite instead of insert
        fn can_overwrite(&self, c: &str) -> bool {
            match self {
                Mask::Digit0(_) | Mask::Digit(_) | Mask::Numeric(_) => false,
                Mask::DecimalSep => "." == c,
                Mask::GroupingSep => false,
                Mask::Sign => "-" == c || " " == c,
                Mask::Plus => "-" == c || "+" == c || " " == c,
                Mask::Hex0 => c == "0",
                Mask::Hex => false,
                Mask::Oct0 => c == "0",
                Mask::Oct => false,
                Mask::Dec0 => c == "0",
                Mask::Dec => false,
                Mask::Letter => false,
                Mask::LetterOrDigit => false,
                Mask::LetterDigitSpace => false,
                Mask::AnyChar => false,
                Mask::Separator(sep) => sep.as_ref() == c,
                Mask::None => false,
            }
        }

        // char can be dropped
        fn can_drop(&self, c: &str) -> bool {
            match self {
                Mask::Digit0(_) => c == "0",
                Mask::Digit(_) => c == " ",
                Mask::Numeric(_) => c == " ",
                Mask::DecimalSep => false,
                Mask::Sign => false,
                Mask::Plus => false,
                Mask::GroupingSep => true,
                Mask::Hex0 => c == "0",
                Mask::Hex => c == " ",
                Mask::Oct0 => c == "0",
                Mask::Oct => c == " ",
                Mask::Dec0 => c == "0",
                Mask::Dec => c == " ",
                Mask::Letter => c == " ",
                Mask::LetterOrDigit => c == " ",
                Mask::LetterDigitSpace => c == " ",
                Mask::AnyChar => c == " ",
                Mask::Separator(_sep) => false,
                Mask::None => false,
            }
        }

        // can be skipped when generating the condensed form
        fn can_skip(&self, c: &str) -> bool {
            match self {
                Mask::Digit0(_) => false,
                Mask::Digit(_) => c == " ",
                Mask::Numeric(_) => c == " ",
                Mask::DecimalSep => false,
                Mask::Sign => false,
                Mask::Plus => false,
                Mask::GroupingSep => true,
                Mask::Hex0 => false,
                Mask::Hex => c == " ",
                Mask::Oct0 => false,
                Mask::Oct => c == " ",
                Mask::Dec0 => false,
                Mask::Dec => c == " ",
                Mask::Letter => c == " ",
                Mask::LetterOrDigit => c == " ",
                Mask::LetterDigitSpace => c == " ",
                Mask::AnyChar => false,
                Mask::Separator(_) => false,
                Mask::None => true,
            }
        }

        /// Get the default char for this mask.
        fn edit_value(&self) -> &str {
            match self {
                Mask::Digit0(_) => "0",
                Mask::Digit(_) => " ",
                Mask::Numeric(_) => " ",
                Mask::DecimalSep => ".",
                Mask::GroupingSep => " ", // don't show. remap_number fills it in if necessary.
                Mask::Sign => " ",
                Mask::Plus => "+",
                Mask::Hex0 => "0",
                Mask::Hex => " ",
                Mask::Oct0 => "0",
                Mask::Oct => " ",
                Mask::Dec0 => "0",
                Mask::Dec => " ",
                Mask::Letter => " ",
                Mask::LetterOrDigit => " ",
                Mask::LetterDigitSpace => " ",
                Mask::AnyChar => " ",
                Mask::Separator(g) => g.as_ref(),
                Mask::None => "",
            }
        }

        /// Get the default display char for this mask.
        fn disp_value(&self) -> &str {
            match self {
                Mask::Digit0(_) => "0",
                Mask::Digit(_) => " ",
                Mask::Numeric(_) => " ",
                Mask::DecimalSep => " ",  // only used by get_display_mask()
                Mask::GroupingSep => " ", // only used by get_display_mask()
                Mask::Sign => " ",
                Mask::Plus => "+",
                Mask::Hex0 => "0",
                Mask::Hex => " ",
                Mask::Oct0 => "0",
                Mask::Oct => " ",
                Mask::Dec0 => "0",
                Mask::Dec => " ",
                Mask::Letter => " ",
                Mask::LetterOrDigit => " ",
                Mask::LetterDigitSpace => " ",
                Mask::AnyChar => " ",
                Mask::Separator(g) => g.as_ref(),
                Mask::None => "",
            }
        }

        fn first<'a>(&self, s: &'a str) -> &'a str {
            if s.is_empty() {
                ""
            } else {
                util::split_at(s, 1).0
            }
        }

        fn can_drop_first(&self, s: &str) -> bool {
            if s.is_empty() {
                false
            } else {
                let (c, _a) = util::split_at(s, 1);
                self.can_drop(c)
            }
        }

        fn can_drop_last(&self, s: &str) -> bool {
            if s.is_empty() {
                false
            } else {
                let end = s.graphemes(true).count();
                let (_, c) = util::split_at(s, end - 1);
                self.can_drop(c)
            }
        }

        fn can_overwrite_first(&self, s: &str) -> bool {
            if s.is_empty() {
                false
            } else {
                let (c, _) = util::split_at(s, 1);
                self.can_overwrite(c)
            }
        }
    }

    impl Debug for MaskToken {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Mask #{}:{}:{}-{} {:?} | {:?}",
                self.nr_id, self.sec_id, self.sec_start, self.sec_end, self.peek_left, self.right
            )
        }
    }

    impl MaskToken {
        /// Number range as Range.
        fn nr_range(&self) -> Range<usize> {
            self.nr_start..self.nr_end
        }

        /// Range as Range.
        fn range(&self) -> Range<usize> {
            self.sec_start..self.sec_end
        }

        /// Create a string with the default edit mask.
        fn empty_section(mask: &[MaskToken]) -> String {
            let mut buf = String::new();
            for m in mask {
                buf.push_str(&m.edit);
            }
            buf
        }

        fn remap_number(submask: &[MaskToken], v: &str) -> Result<String, fmt::Error> {
            // to be safe, always use our internal symbol set.
            let sym = NumberSymbols {
                decimal_sep: '.',
                decimal_grp: Some(','),
                negative_sym: '-',
                positive_sym: ' ',
                exponent_upper_sym: 'E',
                exponent_lower_sym: 'e',
                currency_sym: CurrencySym::new("$"),
            };

            // remove all non numbers and leading 0.
            let mut clean = String::new();
            _ = number::core::clean_num(v, &sym, &mut clean);

            // create number format
            let mut tok = String::new();
            // default fmt.sym is nice
            for t in submask {
                match &t.right {
                    Mask::Digit0(_) => tok.push('0'),
                    Mask::Digit(_) => tok.push('9'),
                    Mask::Numeric(_) => tok.push('#'),
                    Mask::DecimalSep => tok.push('.'),
                    Mask::GroupingSep => tok.push(','),
                    Mask::Sign => tok.push('-'),
                    Mask::Plus => tok.push('+'),
                    Mask::Separator(s) => {
                        for c in s.chars() {
                            tok.push('\\');
                            tok.push(c);
                        }
                    }
                    Mask::None => {}
                    _ => unreachable!("invalid mask"),
                }
            }

            let fmt = match NumberFormat::news(tok, sym) {
                Ok(v) => v,
                Err(_) => return Err(fmt::Error),
            };
            let mut out = String::new();
            match number::core::map_num::<_, false>(clean.as_str(), &fmt, fmt.sym(), &mut out) {
                Ok(_) => {}
                Err(_) => return Err(fmt::Error),
            }

            Ok(out)
        }
    }

    /// Text editing core.
    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    pub struct InputMaskCore {
        mask: Vec<MaskToken>,
        value: String,
        rendered: String,
        len: usize,

        offset: usize,
        width: usize,

        cursor: usize,
        anchor: usize,

        sym: Option<NumberSymbols>,
    }

    impl InputMaskCore {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn new_with_symbols(sym: NumberSymbols) -> Self {
            Self {
                mask: Default::default(),
                value: Default::default(),
                rendered: Default::default(),
                len: 0,
                offset: 0,
                width: 0,
                cursor: 0,
                anchor: 0,
                sym: Some(sym),
            }
        }

        /// Tokens used for the mask.
        pub fn tokens(&self) -> &[MaskToken] {
            &self.mask
        }

        /// Reset value but not the mask and width.
        /// Resets offset and cursor position too.
        pub fn reset(&mut self) {
            self.offset = 0;
            self.set_value(MaskToken::empty_section(&self.mask));
            self.set_default_cursor();
        }

        /// Offset
        pub fn offset(&self) -> usize {
            self.offset
        }

        /// Change the offset.
        ///
        /// Ensures the cursor is visible and modifies any given offset.
        /// Ensures the offset is not beyond the length.
        pub fn set_offset(&mut self, offset: usize) {
            if offset > self.len {
                self.offset = self.len;
            } else if offset > self.cursor {
                self.offset = self.cursor;
            } else if offset + self.width < self.cursor {
                self.offset = self.cursor - self.width;
            } else {
                self.offset = offset;
            }
        }

        /// Display width
        pub fn width(&self) -> usize {
            self.width
        }

        /// Display width
        pub fn set_width(&mut self, width: usize) {
            self.width = width;

            if self.offset + width < self.cursor {
                self.offset = self.cursor - self.width;
            }
        }

        /// Cursor position as grapheme-idx.
        pub fn cursor(&self) -> usize {
            self.cursor
        }

        pub fn anchor(&self) -> usize {
            self.anchor
        }

        pub fn has_selection(&self) -> bool {
            self.cursor != self.anchor
        }

        pub fn is_select_all(&self) -> bool {
            let selection = self.selection();
            selection.start == 0 && selection.end == self.mask.len() - 1
        }

        pub fn selection(&self) -> Range<usize> {
            if self.cursor < self.anchor {
                self.cursor..self.anchor
            } else {
                self.anchor..self.cursor
            }
        }

        pub fn set_cursor(&mut self, cursor: usize, extend_selection: bool) {
            if cursor > self.len {
                self.cursor = self.len;
            } else {
                self.cursor = cursor;
            }

            if !extend_selection {
                self.anchor = self.cursor;
            }

            self.fix_offset();
        }

        fn fix_offset(&mut self) {
            if self.offset > self.cursor {
                self.offset = self.cursor;
            } else if self.offset + self.width < self.cursor {
                self.offset = self.cursor - self.width;
            }
        }

        /// Place cursor at decimal separator, if any. 0 otherwise.
        pub fn set_default_cursor(&mut self) {
            'f: {
                for (i, t) in self.mask.iter().enumerate() {
                    if t.right == Mask::DecimalSep {
                        self.cursor = i;
                        self.anchor = i;
                        break 'f;
                    }
                }
                self.cursor = 0;
                self.anchor = 0;
                self.fix_offset();
            }
        }

        /// Set the decimal separator and other symbols.
        /// Only used for rendering and to map user input.
        /// The value itself uses "."
        pub fn set_num_symbols(&mut self, sym: NumberSymbols) {
            self.sym = Some(sym);
        }

        fn dec_sep(&self) -> char {
            if let Some(sym) = &self.sym {
                sym.decimal_sep
            } else {
                '.'
            }
        }

        fn grp_sep(&self) -> char {
            if let Some(sym) = &self.sym {
                if let Some(grp) = sym.decimal_grp {
                    grp
                } else {
                    // fallback for empty grp-char.
                    // it would be really ugly, if we couldn't keep
                    //   mask-idx == grapheme-idx
                    ' '
                }
            } else {
                ','
            }
        }

        fn neg_sym(&self) -> char {
            if let Some(sym) = &self.sym {
                sym.negative_sym
            } else {
                '-'
            }
        }

        fn pos_sym(&self) -> char {
            if let Some(sym) = &self.sym {
                sym.positive_sym
            } else {
                ' '
            }
        }

        /// Changes the mask.
        /// Resets the value to a default.
        pub fn set_mask<S: AsRef<str>>(&mut self, s: S) -> Result<(), fmt::Error> {
            self.mask = parse_mask(s.as_ref())?;
            self.set_value(MaskToken::empty_section(&self.mask));
            Ok(())
        }

        /// Return the mask.
        pub fn mask(&self) -> String {
            use std::fmt::Write;

            let mut buf = String::new();
            for t in self.mask.iter() {
                _ = write!(buf, "{}", t.right);
            }
            buf
        }

        /// Return the mask.
        pub fn debug_mask(&self) -> String {
            use std::fmt::Write;

            let mut buf = String::new();
            for t in self.mask.iter() {
                _ = write!(buf, "{:?}", t.right);
            }
            buf
        }

        /// Set the mask that is shown.
        pub fn set_display_mask<S: Into<String>>(&mut self, s: S) {
            let display_mask = s.into();

            for (t, m) in self
                .mask
                .iter_mut()
                .zip(display_mask.graphemes(true).chain(repeat_with(|| "")))
            {
                if m.is_empty() {
                    t.display = t.right.disp_value().into();
                } else {
                    t.display = m.into();
                }
            }
        }

        /// Display mask
        pub fn display_mask(&self) -> String {
            let mut buf = String::new();
            for t in &self.mask {
                buf.push_str(&t.display);
            }
            buf
        }

        /// Sets the value.
        /// No checks if the value conforms to the mask.
        /// If the value is too short it will be filled with space.
        /// if the value is too long it will be truncated.
        #[allow(clippy::comparison_chain)]
        pub fn set_value<S: Into<String>>(&mut self, s: S) {
            let mut value = s.into();

            let len = value.graphemes(true).count();
            if len > self.mask.len() - 1 {
                for _ in len..self.mask.len() - 1 {
                    value.pop();
                }
            } else if len < self.mask.len() - 1 {
                for _ in len..self.mask.len() - 1 {
                    value.push(' ');
                }
            }
            let len = value.graphemes(true).count();

            assert_eq!(len, self.mask.len() - 1);

            self.value = value;
            self.len = len;

            if self.offset > self.len {
                self.offset = self.len;
            }
            if self.cursor > self.len {
                self.cursor = self.len;
            }
        }

        /// Value
        pub fn value(&self) -> &str {
            self.value.as_str()
        }

        /// Value without whitespace and grouping separators. Might be easier to parse.
        pub fn compact_value(&self) -> String {
            let mut buf = String::new();
            for (c, m) in self.value.graphemes(true).zip(self.mask.iter()) {
                if !m.right.can_skip(c) {
                    buf.push_str(c);
                }
            }
            buf
        }

        /// No value different from the default.
        pub fn is_empty(&self) -> bool {
            for (m, c) in self.mask.iter().zip(self.value.graphemes(true)) {
                if c != m.edit.as_ref() {
                    return false;
                }
            }
            true
        }

        /// Length
        pub fn len(&self) -> usize {
            self.len
        }

        /// Rendered string for display.
        pub fn rendered(&self) -> &str {
            self.rendered.as_str()
        }

        /// Create the rendered value.
        #[allow(unused_variables)]
        pub fn render_value(&mut self) {
            let mut rendered = mem::take(&mut self.rendered);
            rendered.clear();

            let mut idx = 0;
            loop {
                let mask = &self.mask[idx];

                if mask.right == Mask::None {
                    break;
                }

                let (b, sec, a) = util::split3(&self.value, mask.sec_start..mask.sec_end);
                let sec_mask = &self.mask[mask.sec_start..mask.sec_end];
                let empty = MaskToken::empty_section(sec_mask);

                if sec == empty {
                    for t in sec_mask {
                        match t.right {
                            Mask::Digit0(_) | Mask::Digit(_) | Mask::Numeric(_) => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::DecimalSep => {
                                rendered.push(self.dec_sep());
                            }
                            Mask::GroupingSep => {
                                rendered.push(' ');
                            }
                            Mask::Sign => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::Plus => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::Hex0
                            | Mask::Hex
                            | Mask::Oct0
                            | Mask::Oct
                            | Mask::Dec0
                            | Mask::Dec => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::Letter
                            | Mask::LetterOrDigit
                            | Mask::LetterDigitSpace
                            | Mask::AnyChar => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::Separator(_) => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::None => {}
                        }
                    }
                } else {
                    for (t, s) in sec_mask.iter().zip(sec.graphemes(true)) {
                        match t.right {
                            Mask::Digit0(_) | Mask::Digit(_) => {
                                rendered.push_str(s);
                            }
                            Mask::Numeric(_) => {
                                if s == "." {
                                    rendered.push(self.neg_sym());
                                } else {
                                    rendered.push_str(s);
                                }
                            }
                            Mask::DecimalSep => {
                                if s == "." {
                                    rendered.push(self.dec_sep());
                                } else {
                                    rendered.push(' ');
                                }
                            }
                            Mask::GroupingSep => {
                                if s == "," {
                                    rendered.push(self.grp_sep());
                                } else if s == "-" {
                                    rendered.push(self.neg_sym());
                                } else {
                                    rendered.push(' ');
                                }
                            }
                            Mask::Sign => {
                                if s == "-" {
                                    rendered.push(self.neg_sym());
                                } else {
                                    rendered.push(self.pos_sym());
                                }
                            }
                            Mask::Plus => {
                                if s == "-" {
                                    rendered.push('-');
                                } else {
                                    rendered.push('+');
                                }
                            }
                            Mask::Hex0
                            | Mask::Hex
                            | Mask::Oct0
                            | Mask::Oct
                            | Mask::Dec0
                            | Mask::Dec => {
                                rendered.push_str(s);
                            }
                            Mask::Letter
                            | Mask::LetterOrDigit
                            | Mask::LetterDigitSpace
                            | Mask::AnyChar => {
                                rendered.push_str(s);
                            }
                            Mask::Separator(_) => {
                                rendered.push_str(s);
                            }
                            Mask::None => {}
                        }
                    }
                }

                idx = mask.sec_end;
            }

            self.rendered = rendered;
        }

        /// Create the rendered value.
        #[allow(unused_variables)]
        pub fn render_condensed_value(&mut self) {
            let mut rendered = mem::take(&mut self.rendered);
            rendered.clear();

            let mut idx = 0;
            loop {
                let mask = &self.mask[idx];

                if mask.right == Mask::None {
                    break;
                }

                let (b, sec, a) = util::split3(&self.value, mask.sec_start..mask.sec_end);
                let sec_mask = &self.mask[mask.sec_start..mask.sec_end];
                let empty = MaskToken::empty_section(sec_mask);

                if sec == empty {
                    for t in sec_mask {
                        match t.right {
                            Mask::Digit0(_) | Mask::Digit(_) | Mask::Numeric(_) => {
                                if t.display.as_ref() != " " {
                                    rendered.push_str(t.display.as_ref());
                                }
                            }
                            Mask::DecimalSep => {
                                rendered.push(self.dec_sep());
                            }
                            Mask::GroupingSep => {}
                            Mask::Sign => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::Plus => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::Hex0
                            | Mask::Hex
                            | Mask::Oct0
                            | Mask::Oct
                            | Mask::Dec0
                            | Mask::Dec => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::Letter
                            | Mask::LetterOrDigit
                            | Mask::LetterDigitSpace
                            | Mask::AnyChar => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::Separator(_) => {
                                rendered.push_str(t.display.as_ref());
                            }
                            Mask::None => {}
                        }
                    }
                } else {
                    for (t, s) in sec_mask.iter().zip(sec.graphemes(true)) {
                        match t.right {
                            Mask::Digit0(_) | Mask::Digit(_) => {
                                if s != " " {
                                    rendered.push_str(s);
                                }
                            }
                            Mask::Numeric(_) => {
                                if s == "-" {
                                    rendered.push(self.neg_sym());
                                } else if s != " " {
                                    rendered.push_str(s);
                                }
                            }
                            Mask::DecimalSep => {
                                if s == "." {
                                    rendered.push(self.dec_sep());
                                }
                            }
                            Mask::GroupingSep => {
                                if s == "," {
                                    rendered.push(self.grp_sep());
                                } else if s == "-" {
                                    rendered.push(self.neg_sym());
                                }
                            }
                            Mask::Sign => {
                                if s == "-" {
                                    rendered.push(self.neg_sym());
                                }
                            }
                            Mask::Plus => {
                                if s == "-" {
                                    rendered.push('-');
                                } else {
                                    rendered.push('+');
                                }
                            }
                            Mask::Hex0
                            | Mask::Hex
                            | Mask::Oct0
                            | Mask::Oct
                            | Mask::Dec0
                            | Mask::Dec => {
                                rendered.push_str(s);
                            }
                            Mask::Letter
                            | Mask::LetterOrDigit
                            | Mask::LetterDigitSpace
                            | Mask::AnyChar => {
                                rendered.push_str(s);
                            }
                            Mask::Separator(_) => {
                                rendered.push_str(s);
                            }
                            Mask::None => {}
                        }
                    }
                }

                idx = mask.sec_end;
            }

            self.rendered = rendered;
        }

        /// Next boundary.
        pub fn next_word_boundary(&self) -> usize {
            if self.cursor == self.len {
                self.len
            } else {
                // todo: skip to sep
                self.value
                    .graphemes(true)
                    .enumerate()
                    .skip(self.cursor)
                    .skip_while(|(_, c)| util::is_alphanumeric(c))
                    .find(|(_, c)| util::is_alphanumeric(c))
                    .map(|(i, _)| i)
                    .unwrap_or_else(|| self.len)
            }
        }

        /// Previous boundary.
        pub fn prev_word_boundary(&self) -> usize {
            if self.cursor == 0 {
                0
            } else {
                self.value
                    .graphemes(true)
                    .rev()
                    .skip(self.len - self.cursor)
                    .skip_while(|c| util::is_alphanumeric(c))
                    .skip_while(|c| util::is_alphanumeric(c))
                    .count()
            }
        }

        /// Start at the cursor position and find a valid insert position for the input c.
        /// Put the cursor at that position.
        #[allow(clippy::if_same_then_else)]
        pub fn advance_cursor(&mut self, c: char) {
            let mut new_cursor = self.cursor;

            // debug!("// ADVANCE CURSOR {:?}  ", c);
            // debug!("#[rustfmt::skip]");
            // debug!("let mut b = {};", test_state(self));
            // debug!("b.advance_cursor({:?});", c);

            loop {
                let mask = &self.mask[new_cursor];

                if mask.peek_left.is_rtol()
                    && (mask.right.is_ltor() || mask.right.is_none())
                    && self.can_edit_left_integer(new_cursor, c)
                {
                    // At the gap between an integer field and something else.
                    // Integer fields are served first.
                    break;
                } else if mask.right.is_rtol() && self.is_integer_insert_pos(mask, new_cursor, c) {
                    // Insert position inside an integer field. After any spaces
                    // and the sign.
                    break;
                } else if mask.right.is_number() && self.can_edit_sign(mask, c) {
                    // Can insert a sign here.
                    break;
                } else if mask.right == Mask::DecimalSep && self.is_valid_c(&mask.right, c) {
                    // Decimal separator matches.
                    break;
                } else if mask.right == Mask::GroupingSep {
                    // Never stop here.
                    new_cursor += 1;
                } else if matches!(mask.right, Mask::Separator(_))
                    && self.is_valid_c(&mask.right, c)
                {
                    break;
                } else if mask.peek_left.is_fraction()
                    && self.can_skip_left_in_fraction(mask, new_cursor, c)
                {
                    // skip left
                    new_cursor -= 1;
                } else if mask.right.is_fraction() && self.is_valid_c(&mask.right, c) {
                    break;
                } else if matches!(
                    mask.right,
                    Mask::Hex0 | Mask::Hex | Mask::Dec0 | Mask::Dec | Mask::Oct0 | Mask::Oct
                ) && self.is_valid_c(&mask.right, c)
                {
                    break;
                } else if matches!(
                    mask.right,
                    Mask::Letter | Mask::LetterOrDigit | Mask::LetterDigitSpace | Mask::AnyChar
                ) && self.is_valid_c(&mask.right, c)
                {
                    break;
                } else if mask.right == Mask::None {
                    // No better position found. Reset and break;
                    new_cursor = self.cursor;
                    break;
                } else {
                    new_cursor += 1;
                }
            }

            // debug!("CURSOR {} => {}", self.cursor, new_cursor);
            self.cursor = new_cursor;
            self.anchor = self.cursor;
            self.fix_offset();

            // debug!("#[rustfmt::skip]");
            // debug!("let a = {};", test_state(self));
            // debug!("assert_eq_core(&b,&a);");
        }

        /// Use mapped-char instead of input.
        fn map_input_c(&self, mask: &Mask, c: char) -> char {
            match mask {
                Mask::Numeric(_) => {
                    if c == self.neg_sym() {
                        return '-';
                    } else if c == self.pos_sym() {
                        return ' ';
                    }
                }
                Mask::DecimalSep => {
                    if c == self.dec_sep() {
                        return '.';
                    }
                }
                Mask::Sign => {
                    if c == self.neg_sym() {
                        return '-';
                    } else if c == self.pos_sym() || c == '+' {
                        return ' ';
                    }
                }
                Mask::Plus => {
                    if c == self.neg_sym() {
                        return '-';
                    } else if c == self.pos_sym() {
                        return '+';
                    }
                }
                _ => {}
            }
            c
        }

        /// Valid input for this mask.
        fn is_valid_c(&self, mask: &Mask, c: char) -> bool {
            match mask {
                Mask::Digit0(_) => c.is_ascii_digit(),
                Mask::Digit(_) => c.is_ascii_digit() || c == ' ',
                Mask::Numeric(_) => {
                    c.is_ascii_digit()
                        || c == ' '
                        || c == self.neg_sym()
                        || c == self.pos_sym()
                        || c == '-'
                        || c == '+'
                }
                Mask::DecimalSep => c == self.dec_sep(),
                Mask::GroupingSep => false,
                Mask::Sign => c == self.neg_sym() || c == self.pos_sym() || c == '-' || c == '+',
                Mask::Plus => c == self.neg_sym() || c == self.pos_sym() || c == '-' || c == '+',
                Mask::Hex0 => c.is_ascii_hexdigit(),
                Mask::Hex => c.is_ascii_hexdigit() || c == ' ',
                Mask::Oct0 => c.is_digit(8),
                Mask::Oct => c.is_digit(8) || c == ' ',
                Mask::Dec0 => c.is_ascii_digit(),
                Mask::Dec => c.is_ascii_digit() || c == ' ',
                Mask::Letter => c.is_alphabetic(),
                Mask::LetterOrDigit => c.is_alphanumeric(),
                Mask::LetterDigitSpace => c.is_alphanumeric() || c == ' ',
                Mask::AnyChar => true,
                Mask::Separator(sep) => {
                    // todo: don't know better
                    if let Some(sepc) = sep.chars().next() {
                        sepc == c
                    } else {
                        false
                    }
                }
                Mask::None => false,
            }
        }

        // Can insert one more digit into the field to the left.
        #[inline]
        fn can_skip_left_in_fraction(&self, mask: &MaskToken, new_cursor: usize, c: char) -> bool {
            let (_b, a) = util::split_at(&self.value, new_cursor - 1);
            // is there space to the left?
            mask.peek_left.can_drop_first(a) && self.is_valid_c(&mask.peek_left, c)
        }

        // Can input a sign here?
        #[inline]
        fn can_edit_sign(&self, mask: &MaskToken, c: char) -> bool {
            if !self.is_valid_c(&Mask::Sign, c) {
                return false;
            }

            for i in mask.nr_range() {
                let t = &self.mask[i];
                match t.right {
                    Mask::Plus => return true,
                    Mask::Sign => return true,
                    Mask::Numeric(EditDirection::Rtol) => {
                        // Numeric fields can hold a sign.
                        // If they are not otherwise occupied.
                        let (_b, a) = util::split_at(&self.value, i);
                        return t.right.can_drop_first(a) || t.right.first(a) == "-";
                    }
                    _ => {}
                }
            }

            false
        }

        // Is this the correct input position for a rtol field
        #[inline]
        fn is_integer_insert_pos(&self, mask: &MaskToken, new_cursor: usize, c: char) -> bool {
            let (_b, a) = util::split_at(&self.value, new_cursor);
            // stop at real digit, that is the first non-droppable grapheme. except '-'
            !mask.right.can_drop_first(a)
                && mask.right.first(a) != "-"
                && self.is_valid_c(&mask.right, c)
        }

        // Can edit the field left of the cursor.
        #[inline]
        fn can_edit_left_integer(&self, new_cursor: usize, c: char) -> bool {
            let left = &self.mask[new_cursor - 1];
            let mask0 = &self.mask[left.sec_start];
            let (_b, c0, _c1, _a) = util::split_mask(&self.value, new_cursor, left.range());
            // can insert at mask gap?
            mask0.right.can_drop_first(c0) && self.is_valid_c(&left.right, c)
        }

        /// Insert the char if it matches the cursor mask and the current section is not full.
        ///
        ///
        ///
        /// `advance_cursor()` must be called before for correct functionality.
        ///
        /// Otherwise: your mileage might vary.
        pub fn insert_char(&mut self, c: char) -> Result<(), fmt::Error> {
            // let mask = &self.mask[self.cursor];
            // debug!("// INSERT CHAR {:?} {:?}", mask, c);
            // debug!("#[rustfmt::skip]");
            // debug!("let mut b = {};", test_state(self));
            // debug!("b.insert_char({:?});", c);

            // note: because of borrow checker. calls &mut methods.
            {
                let mask = &self.mask[self.cursor];
                if mask.right.is_number() && self.can_edit_sign(mask, c) {
                    if self.insert_sign(c)? {
                        return Ok(());
                    }
                }
            }
            {
                let mask = &self.mask[self.cursor];
                if mask.peek_left.is_number() && (mask.right.is_ltor() || mask.right.is_none()) {
                    let left = &self.mask[self.cursor - 1];
                    if self.can_edit_sign(left, c) {
                        if self.insert_sign(c)? {
                            return Ok(());
                        }
                    }
                }
            }
            {
                let mask = &self.mask[self.cursor];
                if mask.right.is_rtol() {
                    if self.insert_rtol(c)? {
                        return Ok(());
                    }
                }
            }
            {
                let mask = &self.mask[self.cursor];
                if mask.peek_left.is_rtol() && (mask.right.is_ltor() || mask.right.is_none()) {
                    if self.insert_rtol(c)? {
                        return Ok(());
                    }
                }
            }
            {
                let mask = &self.mask[self.cursor];
                if mask.right.is_ltor() {
                    if self.insert_ltor(c)? {
                        return Ok(());
                    }
                }
            }

            // debug!("#[rustfmt::skip]");
            // debug!("let a = {};", test_state(self));
            // debug!("assert_eq_core(&b,&a);");

            Ok(())
        }

        /// Insert c into a ltor section.
        fn insert_ltor(&mut self, c: char) -> Result<bool, fmt::Error> {
            let mask = &self.mask[self.cursor];
            let mask9 = &self.mask[mask.sec_end - 1];
            let (b, c0, c1, a) = util::split_mask(&self.value, self.cursor, mask.range());

            if mask.right.can_overwrite_first(c1) && self.is_valid_c(&mask.right, c) {
                let mut buf = String::new();
                buf.push_str(b);
                buf.push_str(c0);
                buf.push(self.map_input_c(&mask.right, c));
                buf.push_str(util::drop_first(c1));
                buf.push_str(a);
                debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                self.value = buf;

                self.cursor += 1;
                self.anchor = self.cursor;

                return Ok(true);
            }
            if mask9.right.can_drop_last(c1) && self.is_valid_c(&mask.right, c) {
                let mut buf = String::new();
                buf.push_str(b);
                buf.push_str(c0);
                buf.push(self.map_input_c(&mask.right, c));
                buf.push_str(util::drop_last(c1));
                buf.push_str(a);
                debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                self.value = buf;

                self.cursor += 1;
                self.anchor = self.cursor;

                return Ok(true);
            }

            Ok(false)
        }

        /// Insert c into a rtol section
        fn insert_rtol(&mut self, c: char) -> Result<bool, fmt::Error> {
            let mut mask = &self.mask[self.cursor];
            // boundary right/left. prefer right, change mask.
            if mask.peek_left.is_rtol() && (mask.right.is_ltor() || mask.right.is_none()) {
                mask = &self.mask[self.cursor - 1];
            }
            let mask0 = &self.mask[mask.sec_start];
            let (b, c0, c1, a) = util::split_mask(&self.value, self.cursor, mask.range());

            if mask0.right.can_drop_first(c0) && self.is_valid_c(&mask.right, c) {
                let mut mstr = String::new();
                mstr.push_str(util::drop_first(c0));
                mstr.push(self.map_input_c(&mask.right, c));
                mstr.push_str(c1);

                let submask = &self.mask[mask.sec_start..mask.sec_end];
                let mmstr = MaskToken::remap_number(submask, &mstr)?;

                let mut buf = String::new();
                buf.push_str(b);
                buf.push_str(mmstr.as_str());
                buf.push_str(a);
                debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                self.value = buf;
                // cursor stays

                return Ok(true);
            }

            Ok(false)
        }

        /// Insert a sign c into the current number section
        #[allow(clippy::single_match)]
        fn insert_sign(&mut self, c: char) -> Result<bool, fmt::Error> {
            let mut mask = &self.mask[self.cursor];
            // boundary right/left. prefer right, change mask.
            if mask.peek_left.is_number() && (mask.right.is_ltor() || mask.right.is_none()) {
                mask = &self.mask[self.cursor - 1];
            }
            for i in mask.nr_range() {
                if matches!(
                    &self.mask[i],
                    MaskToken {
                        right: Mask::Sign,
                        ..
                    }
                ) {
                    let cc = self.map_input_c(&Mask::Sign, c);
                    let (b, c0, a) = util::split3(self.value(), i..i + 1);
                    let repl = if cc == ' ' {
                        " "
                    } else if cc == '-' {
                        if c0 == "-" {
                            " "
                        } else {
                            "-"
                        }
                    } else {
                        unreachable!();
                    };

                    let mut buf = String::new();
                    buf.push_str(b);
                    buf.push_str(repl);
                    buf.push_str(a);
                    debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                    self.value = buf;
                    // note: probably no remap necessary?
                    return Ok(true);
                } else if matches!(
                    &self.mask[i],
                    MaskToken {
                        right: Mask::Plus,
                        ..
                    }
                ) {
                    let cc = self.map_input_c(&Mask::Plus, c);
                    let (b, c0, a) = util::split3(self.value(), i..i + 1);
                    let repl = if cc == '+' {
                        "+"
                    } else if cc == '-' {
                        if c0 == "-" {
                            "+"
                        } else {
                            "-"
                        }
                    } else {
                        unreachable!();
                    };

                    let mut buf = String::new();
                    buf.push_str(b);
                    buf.push_str(repl);
                    buf.push_str(a);
                    debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                    self.value = buf;
                    // note: probably no remap necessary?
                    return Ok(true);
                }
            } // else
              // find "-" sign at a moving position.
            let (b, c0, p, c1, a) = util::split_mask_match(&self.value, "-", mask.nr_range());
            if p == "-" {
                let mut buf = String::new();
                buf.push_str(b);
                buf.push_str(c0);
                buf.push(' ');
                buf.push_str(c1);
                buf.push_str(a);
                debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                self.value = buf;
                // note: probably no remap necessary?
                return Ok(true);
            }
            // else
            // insert a fresh "-" somewhere
            if c == self.neg_sym() {
                for i in mask.nr_range() {
                    let mask = &self.mask[i];
                    if matches!(
                        mask,
                        MaskToken {
                            right: Mask::Numeric(EditDirection::Rtol),
                            ..
                        }
                    ) {
                        let submask = &self.mask[mask.nr_range()];
                        let (b, c0, c1, a) = util::split_mask(self.value(), i, mask.nr_range());

                        if self.mask[i].right.can_drop_first(c1) {
                            let mut mstr = String::new();
                            mstr.push_str(c0);
                            mstr.push('-');
                            mstr.push_str(util::drop_first(c1));
                            let mmstr = MaskToken::remap_number(submask, &mstr)?;

                            let mut buf = String::new();
                            buf.push_str(b);
                            buf.push_str(mmstr.as_str());
                            buf.push_str(a);
                            debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                            self.value = buf;
                        };

                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }

        /// Remove the selection.
        pub fn remove_selection(&mut self, selection: Range<usize>) -> Result<(), fmt::Error> {
            let mut buf = String::new();

            let mut mask = &self.mask[selection.start];

            // debug!("// REMOVE SELECTION {:?} {:?}", mask, selection);
            // debug!("#[rustfmt::skip]");
            // debug!("let mut b = {};", test_state(self));
            // debug!("b.remove_selection({:?});", selection);

            let (a, _, _, _, _) =
                util::split_remove_mask(self.value.as_str(), selection.clone(), mask.range());
            buf.push_str(a); // stuff before any part of the selection

            loop {
                // remove section by section.
                let (_, c0, s, c1, _) =
                    util::split_remove_mask(self.value.as_str(), selection.clone(), mask.range());

                if mask.right.is_rtol() {
                    let remove_count = s.graphemes(true).count();
                    let fill_before = &self.mask[mask.sec_start..mask.sec_start + remove_count];

                    let mut mstr = String::new();
                    mstr.push_str(MaskToken::empty_section(fill_before).as_str());
                    mstr.push_str(c0);
                    mstr.push_str(c1);

                    let mmstr =
                        MaskToken::remap_number(&self.mask[mask.sec_start..mask.sec_end], &mstr)?;

                    buf.push_str(&mmstr);
                } else if mask.right.is_ltor() {
                    let c0_count = c0.graphemes(true).count();
                    let c1_count = c1.graphemes(true).count();
                    let fill_after = &self.mask[mask.sec_start + c0_count + c1_count..mask.sec_end];

                    let mut mstr = String::new();
                    mstr.push_str(c0);
                    mstr.push_str(c1);
                    mstr.push_str(MaskToken::empty_section(fill_after).as_str());

                    buf.push_str(&mstr);
                }

                if mask.sec_end >= selection.end {
                    // todo: should this be selection.end..mask.sec_end instead?
                    let (_, _, a) = util::split3(&self.value, mask.sec_end..mask.sec_end);
                    buf.push_str(a);
                    break;
                }

                mask = &self.mask[mask.sec_end];
            }
            debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
            self.value = buf;

            self.cursor = selection.start;
            self.anchor = self.cursor;
            self.fix_offset();

            // debug!("#[rustfmt::skip]");
            // debug!("let a = {};", test_state(self));
            // debug!("assert_eq_core(&b,&a);");

            Ok(())
        }

        /// Remove the previous char.
        pub fn remove_prev(&mut self) -> Result<(), fmt::Error> {
            if self.cursor == 0 {
                return Ok(());
            }

            let left = &self.mask[self.cursor - 1];

            // debug!("// REMOVE PREV {:?} ", left);
            // debug!("#[rustfmt::skip]");
            // debug!("let mut b = {};", test_state(self));
            // debug!("b.remove_prev();");

            let (b, c0, _s, c1, a) = util::split_remove_mask(
                self.value.as_str(),
                self.cursor - 1..self.cursor,
                left.range(),
            );

            // remove and fill with empty
            if left.right.is_rtol() {
                let fill_mask = &self.mask[left.sec_start..left.sec_start + 1];
                let mut mstr = String::new();
                mstr.push_str(MaskToken::empty_section(fill_mask).as_str());
                mstr.push_str(c0);
                mstr.push_str(c1);
                let mmstr =
                    MaskToken::remap_number(&self.mask[left.sec_start..left.sec_end], &mstr)?;

                let mut buf = String::new();
                buf.push_str(b);
                buf.push_str(&mmstr);
                buf.push_str(a);
                debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                self.value = buf;
            } else if left.right.is_ltor() {
                let mut buf = String::new();
                buf.push_str(b);
                buf.push_str(c0);
                buf.push_str(c1);

                let c0_count = c0.graphemes(true).count();
                let c1_count = c1.graphemes(true).count();
                let fill_mask = &self.mask[left.sec_start + c0_count + c1_count..left.sec_end];
                buf.push_str(MaskToken::empty_section(fill_mask).as_str());

                buf.push_str(a);
                debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                self.value = buf;
            }

            // place cursor after deletion
            if left.right.is_rtol() {
                // in a rtol field keep the cursor at the same position until the
                // whole section is empty. Only then put it at the beginning of the section
                // to continue left of the section.
                let (_b, s, _a) = util::split3(self.value(), left.sec_start..left.sec_end);
                let sec_mask = &self.mask[left.sec_start..left.sec_end];
                if s == MaskToken::empty_section(sec_mask) {
                    self.cursor = left.sec_start;
                    self.anchor = self.cursor;
                } else {
                    // cursor stays
                }
            } else if left.right.is_ltor() {
                self.cursor -= 1;
                self.anchor = self.cursor;
            }
            self.fix_offset();

            // debug!("#[rustfmt::skip]");
            // debug!("let a = {};", test_state(self));
            // debug!("assert_eq_core(&b,&a);");

            Ok(())
        }

        /// Remove the next char.
        pub fn remove_next(&mut self) -> Result<(), fmt::Error> {
            if self.cursor == self.mask.len() - 1 {
                return Ok(());
            }

            let right = &self.mask[self.cursor];

            // debug!("// REMOVE NEXT {:?} ", right);
            // debug!("#[rustfmt::skip]");
            // debug!("let mut b = {};", test_state(self));
            // debug!("b.remove_next();");

            let (b, c0, _, c1, a) = util::split_remove_mask(
                self.value.as_str(),
                self.cursor..self.cursor + 1,
                right.range(),
            );

            // remove and fill with empty
            if right.right.is_rtol() {
                let mut mstr = String::new();
                let fill_mask = &self.mask[right.sec_start..right.sec_start + 1];
                mstr.push_str(MaskToken::empty_section(fill_mask).as_str());
                mstr.push_str(c0);
                mstr.push_str(c1);
                let mmstr =
                    MaskToken::remap_number(&self.mask[right.sec_start..right.sec_end], &mstr)?;

                let mut buf = String::new();
                buf.push_str(b);
                buf.push_str(&mmstr);
                buf.push_str(a);
                debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                self.value = buf;
            } else if right.right.is_ltor() {
                let mut buf = String::new();
                buf.push_str(b);
                buf.push_str(c0);
                buf.push_str(c1);

                let c0_count = c0.graphemes(true).count();
                let c1_count = c1.graphemes(true).count();
                let fill_mask = &self.mask[right.sec_start + c0_count + c1_count..right.sec_end];
                buf.push_str(MaskToken::empty_section(fill_mask).as_str());

                buf.push_str(a);
                debug_assert_eq!(util::gr_len(&buf), util::gr_len(&self.value));
                self.value = buf;
            }

            // place cursor after deletion
            if right.right.is_rtol() {
                self.cursor += 1;
                self.anchor = self.cursor;
            } else if right.right.is_ltor() {
                // in a ltor field keep the cursor at the same position until the
                // whole section is empty. Only then put it at the end of the section
                // to continue right of the section.
                let (_b, s, _a) = util::split3(self.value(), right.sec_start..right.sec_end);
                let sec_mask = &self.mask[right.sec_start..right.sec_end];
                if s == MaskToken::empty_section(sec_mask) {
                    self.cursor = right.sec_end;
                    self.anchor = self.cursor;
                } else {
                    // cursor stays
                }
            }
            self.fix_offset();

            // debug!("#[rustfmt::skip]");
            // debug!("let a = {};", test_state(self));
            // debug!("assert_eq_core(&b,&a);");

            Ok(())
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub fn parse_mask(mask_str: &str) -> Result<Vec<MaskToken>, fmt::Error> {
        let mut out = Vec::<MaskToken>::new();

        let mut start_id = 0;
        let mut id = 0;
        let mut start_nr = 0;
        let mut nr_id = 0;
        let mut last_mask = Mask::None;
        let mut dec_dir = EditDirection::Rtol;
        let mut esc = false;
        let mut idx = 0;
        for m in mask_str.graphemes(true).chain(once("")) {
            let mask = if esc {
                esc = false;
                Mask::Separator(Box::from(m))
            } else {
                match m {
                    "0" => Mask::Digit0(dec_dir),
                    "9" => Mask::Digit(dec_dir),
                    "#" => Mask::Numeric(dec_dir),
                    "." => Mask::DecimalSep,
                    "," => Mask::GroupingSep,
                    "-" => Mask::Sign,
                    "+" => Mask::Plus,
                    "h" => Mask::Hex,
                    "H" => Mask::Hex0,
                    "o" => Mask::Oct,
                    "O" => Mask::Oct0,
                    "d" => Mask::Dec,
                    "D" => Mask::Dec0,
                    "l" => Mask::Letter,
                    "a" => Mask::LetterOrDigit,
                    "c" => Mask::LetterDigitSpace,
                    "_" => Mask::AnyChar,
                    "" => Mask::None,
                    " " | ";" | ":" | "/" => Mask::Separator(Box::from(m)),
                    "\\" => {
                        esc = true;
                        continue;
                    }
                    s if s.is_ascii() => return Err(fmt::Error),
                    s => Mask::Separator(Box::from(s)),
                }
            };

            match mask {
                Mask::Digit0(_)
                | Mask::Digit(_)
                | Mask::Numeric(_)
                | Mask::GroupingSep
                | Mask::Sign
                | Mask::Plus => {
                    // no change
                }
                Mask::DecimalSep => {
                    dec_dir = EditDirection::Ltor;
                }
                Mask::Hex0
                | Mask::Hex
                | Mask::Oct0
                | Mask::Oct
                | Mask::Dec0
                | Mask::Dec
                | Mask::Letter
                | Mask::LetterOrDigit
                | Mask::LetterDigitSpace
                | Mask::AnyChar
                | Mask::Separator(_) => {
                    // reset to default number input direction
                    dec_dir = EditDirection::Rtol
                }
                Mask::None => {
                    // no change, doesn't matter
                }
            }

            if matches!(mask, Mask::Separator(_)) || mask.number() != last_mask.number() {
                for j in start_nr..idx {
                    out[j].nr_id = nr_id;
                    out[j].nr_start = start_nr;
                    out[j].nr_end = idx;
                }
                nr_id += 1;
                start_nr = idx;
            }
            if matches!(mask, Mask::Separator(_)) || mask.section() != last_mask.section() {
                for j in start_id..idx {
                    out[j].sec_id = id;
                    out[j].sec_start = start_id;
                    out[j].sec_end = idx;
                }
                id += 1;
                start_id = idx;
            }

            let tok = MaskToken {
                nr_id: 0,
                nr_start: 0,
                nr_end: 0,
                sec_id: 0,
                sec_start: 0,
                sec_end: 0,
                peek_left: last_mask,
                right: mask.clone(),
                edit: mask.edit_value().into(),
                display: mask.disp_value().into(),
            };
            out.push(tok);

            idx += 1;
            last_mask = mask;
        }
        for j in start_nr..out.len() {
            out[j].nr_id = nr_id;
            out[j].nr_start = start_nr;
            out[j].nr_end = mask_str.graphemes(true).count();
        }
        for j in start_id..out.len() {
            out[j].sec_id = id;
            out[j].sec_start = start_id;
            out[j].sec_end = mask_str.graphemes(true).count();
        }

        Ok(out)
    }

    /// dump the current state as code.
    pub fn test_state(m: &InputMaskCore) -> String {
        use std::fmt::Write;

        let mut buf = String::new();
        _ = write!(buf, "test_input_mask_core(");
        _ = write!(buf, "{:?}, ", m.mask());
        _ = write!(buf, "{:?}, ", m.value);
        _ = write!(buf, "{:?}, ", m.rendered);
        _ = write!(buf, "{:?}, ", m.len);
        _ = write!(buf, "{:?}, ", m.offset);
        _ = write!(buf, "{:?}, ", m.width);
        _ = write!(buf, "{:?}, ", m.cursor);
        _ = write!(buf, "{:?},", m.anchor);
        if let Some(sym) = &m.sym {
            _ = write!(
                buf,
                "Some(\"{}{}{}{}{}{}\")",
                sym.decimal_sep,
                if let Some(decimal_grp) = sym.decimal_grp {
                    decimal_grp
                } else {
                    ' '
                },
                sym.negative_sym,
                sym.positive_sym,
                sym.exponent_upper_sym,
                sym.exponent_lower_sym
            );
        } else {
            _ = write!(buf, "sym: None, ");
        }
        _ = write!(buf, ")");
        buf
    }

    #[track_caller]
    pub fn assert_eq_core(a: &InputMaskCore, b: &InputMaskCore) {
        assert_eq!(b.value, a.value);
        assert_eq!(b.rendered, a.rendered);
        assert_eq!(b.len, a.len);
        assert_eq!(b.offset, a.offset);
        assert_eq!(b.width, a.width);
        assert_eq!(b.cursor, a.cursor);
        assert_eq!(b.anchor, a.anchor);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn test_input_mask_core(
        mask: &str,
        value: &str,
        rendered: &str,
        len: usize,
        offset: usize,
        width: usize,
        cursor: usize,
        anchor: usize,
        sym: Option<&str>,
    ) -> Result<InputMaskCore, fmt::Error> {
        Ok(InputMaskCore {
            mask: parse_mask(mask)?,
            value: value.to_string(),
            rendered: rendered.to_string(),
            len,
            offset,
            width,
            cursor,
            anchor,
            sym: sym.map(parse_number_symbols),
        })
    }

    pub fn parse_number_symbols(s: &str) -> NumberSymbols {
        let mut s = s.chars();
        NumberSymbols {
            decimal_sep: s.next().expect("decimal_sep"),
            decimal_grp: Some(s.next().expect("decimal_grp")),
            negative_sym: s.next().expect("negative_sym"),
            positive_sym: s.next().expect("positive_sym"),
            exponent_upper_sym: s.next().expect("exponent_upper_sym"),
            exponent_lower_sym: s.next().expect("exponent_lower_sym"),
            currency_sym: s.collect::<String>().as_str().into(),
        }
    }
}
