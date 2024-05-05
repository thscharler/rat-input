//!
//! Text input widget.
//!
//! * Can do the usual insert/delete/movement operations.
//! * Text selection via keyboard and mouse.
//! * Scrolls with the cursor.
//! * Modes for focused and valid.
//!
//!
//! The visual cursor must be set separately after rendering.
//! It is accessible as [TextInputState::visual_cursor()] or [TextInputState::cursor] after rendering.
//!
//! Event handling by calling the freestanding fn [crate::masked_input::handle_events].
//! There's [handle_mouse_events] if you want to override the default key bindings but keep
//! the mouse behaviour.
//!

use crate::_private::NonExhaustive;
use crate::events::{DefaultKeys, HandleEvent, MouseOnly, Outcome};
use crate::util::MouseFlags;
use crate::{ct_event, util};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::{BlockExt, StatefulWidget};
use ratatui::style::Style;
use ratatui::widgets::{Block, StatefulWidgetRef, WidgetRef};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Text input widget.
#[derive(Debug)]
pub struct TextInput<'a> {
    block: Option<Block<'a>>,
    style: Style,
    focus_style: Style,
    select_style: Style,
    invalid_style: Style,
    focused: bool,
    valid: bool,
}

/// Combined style for the widget.
#[derive(Debug)]
pub struct TextInputStyle {
    pub style: Style,
    pub focus: Style,
    pub select: Style,
    pub invalid: Style,
    pub non_exhaustive: NonExhaustive,
}

impl<'a> Default for TextInput<'a> {
    fn default() -> Self {
        Self {
            block: None,
            style: Default::default(),
            focus_style: Default::default(),
            select_style: Default::default(),
            invalid_style: Default::default(),
            focused: true,
            valid: true,
        }
    }
}

impl Default for TextInputStyle {
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

impl<'a> TextInput<'a> {
    /// Set the combined style.
    pub fn style(mut self, style: TextInputStyle) -> Self {
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
    /// This is patched onto either base_style or focus_style
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

impl<'a> StatefulWidget for TextInput<'a> {
    type State = TextInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidgetRef::render_ref(&self, area, buf, state);
    }
}

impl<'a> StatefulWidgetRef for TextInput<'a> {
    type State = TextInputState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = self.block.inner_if_some(area);
        state.value.set_width(state.area.width as usize);

        self.block.render_ref(area, buf);

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

        let selection = util::clamp_shift(
            state.value.selection(),
            state.value.offset(),
            state.value.width(),
        );

        let mut cit = state.value.value().graphemes(true).skip(state.offset());
        for col in 0..area.width as usize {
            let cell = buf.get_mut(area.x + col as u16, area.y);
            if let Some(c) = cit.next() {
                cell.set_symbol(c);
            } else {
                cell.set_char(' ');
            }

            if selection.contains(&col) {
                if self.valid {
                    cell.set_style(select_style);
                } else {
                    cell.set_style(invalid_select_style);
                }
            } else {
                if self.valid {
                    cell.set_style(style);
                } else {
                    cell.set_style(invalid_style);
                }
            }
        }

        let cursor = state.value.cursor().saturating_sub(state.value.offset()) as u16;
        state.cursor = Position::new(state.area.x + cursor, state.area.y);
    }
}

/// Input state data.
#[derive(Debug, Clone)]
pub struct TextInputState {
    /// The position of the cursor in screen coordinates.
    /// Can be directly used for [Frame::set_cursor()]
    pub cursor: Position,
    /// Area inside a possible block.
    pub area: Rect,
    /// Mouse selection in progress.
    pub mouse: MouseFlags,
    /// Editing core
    pub value: core::InputCore,
    /// Construct with `..Default::default()`
    pub non_exhaustive: NonExhaustive,
}

impl Default for TextInputState {
    fn default() -> Self {
        Self {
            cursor: Default::default(),
            area: Default::default(),
            mouse: Default::default(),
            value: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl HandleEvent<crossterm::event::Event, DefaultKeys, Outcome> for TextInputState {
    fn handle(
        &mut self,
        event: &crossterm::event::Event,
        focus: bool,
        _keymap: DefaultKeys,
    ) -> Outcome {
        let r = 'f: {
            if focus {
                match event {
                    ct_event!(keycode press Left) => self.move_to_prev(false),
                    ct_event!(keycode press Right) => self.move_to_next(false),
                    ct_event!(keycode press CONTROL-Left) => {
                        let pos = self.prev_word_boundary();
                        self.set_cursor(pos, false);
                    }
                    ct_event!(keycode press CONTROL-Right) => {
                        let pos = self.next_word_boundary();
                        self.set_cursor(pos, false);
                    }
                    ct_event!(keycode press Home) => self.set_cursor(0, false),
                    ct_event!(keycode press End) => self.set_cursor(self.len(), false),
                    ct_event!(keycode press SHIFT-Left) => self.move_to_prev(true),
                    ct_event!(keycode press SHIFT-Right) => self.move_to_next(true),
                    ct_event!(keycode press CONTROL_SHIFT-Left) => {
                        let pos = self.prev_word_boundary();
                        self.set_cursor(pos, true);
                    }
                    ct_event!(keycode press CONTROL_SHIFT-Right) => {
                        let pos = self.next_word_boundary();
                        self.set_cursor(pos, true);
                    }
                    ct_event!(keycode press SHIFT-Home) => self.set_cursor(0, true),
                    ct_event!(keycode press SHIFT-End) => self.set_cursor(self.len(), true),
                    ct_event!(key press CONTROL-'a') => self.set_selection(0, self.len()),
                    ct_event!(keycode press Backspace) => self.delete_prev_char(),
                    ct_event!(keycode press Delete) => self.delete_next_char(),
                    ct_event!(keycode press CONTROL-Backspace) => {
                        let prev = self.prev_word_boundary();
                        self.replace(prev..self.cursor(), "");
                    }
                    ct_event!(keycode press CONTROL-Delete) => {
                        let next = self.next_word_boundary();
                        self.replace(self.cursor()..next, "");
                    }
                    ct_event!(key press CONTROL-'d') => self.set_value(""),
                    ct_event!(keycode press CONTROL_SHIFT-Backspace) => {
                        self.replace(0..self.cursor(), "")
                    }
                    ct_event!(keycode press CONTROL_SHIFT-Delete) => {
                        self.replace(self.cursor()..self.len(), "")
                    }
                    ct_event!(key press c) | ct_event!(key press SHIFT-c) => self.insert_char(*c),
                    _ => break 'f Outcome::NotUsed,
                }
                Outcome::Changed
            } else {
                Outcome::NotUsed
            }
        };

        match r {
            Outcome::NotUsed => HandleEvent::handle(self, event, focus, MouseOnly),
            v => v,
        }
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for TextInputState {
    fn handle(
        &mut self,
        event: &crossterm::event::Event,
        _focus: bool,
        _keymap: MouseOnly,
    ) -> Outcome {
        match event {
            ct_event!(mouse down Left for column,row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    self.mouse.set_drag();
                    let c = column - self.area.x;
                    self.set_offset_relative_cursor(c as isize, false);
                    Outcome::Changed
                } else {
                    Outcome::NotUsed
                }
            }
            ct_event!(mouse drag Left for column, _row) => {
                if self.mouse.do_drag() {
                    let c = (*column as isize) - (self.area.x as isize);
                    self.set_offset_relative_cursor(c, true);
                    Outcome::Changed
                } else {
                    Outcome::NotUsed
                }
            }
            ct_event!(mouse moved) => {
                self.mouse.clear_drag();
                Outcome::NotUsed
            }
            _ => Outcome::NotUsed,
        }
    }
}

/// Handle all events.
/// Text events are only processed if focus is true.
/// Mouse events are processed if they are in range.
pub fn handle_events(
    state: &mut TextInputState,
    focus: bool,
    event: &crossterm::event::Event,
) -> Outcome {
    HandleEvent::handle(state, event, focus, DefaultKeys)
}

/// Handle only mouse-events.
pub fn handle_mouse_events(state: &mut TextInputState, event: &crossterm::event::Event) -> Outcome {
    HandleEvent::handle(state, event, false, MouseOnly)
}

impl TextInputState {
    /// Reset to empty.
    pub fn reset(&mut self) {
        self.value.clear();
    }

    /// Offset shown.
    pub fn offset(&self) -> usize {
        self.value.offset()
    }

    /// Offset shown. This is corrected if the cursor wouldn't be visible.
    pub fn set_offset(&mut self, offset: usize) {
        self.value.set_offset(offset);
    }

    /// Set the cursor position, reset selection.
    pub fn set_cursor(&mut self, cursor: usize, extend_selection: bool) {
        self.value.set_cursor(cursor, extend_selection);
    }

    /// Cursor position.
    pub fn cursor(&self) -> usize {
        self.value.cursor()
    }

    /// Set text.
    pub fn set_value<S: Into<String>>(&mut self, s: S) {
        self.value.set_value(s);
    }

    /// Text.
    pub fn value(&self) -> &str {
        self.value.value()
    }

    /// Text
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Text length as grapheme count.
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// Selection.
    pub fn has_selection(&self) -> bool {
        self.value.has_selection()
    }

    /// Selection.
    pub fn set_selection(&mut self, anchor: usize, cursor: usize) {
        self.value.set_cursor(anchor, false);
        self.value.set_cursor(cursor, true);
    }

    /// Selection.
    pub fn select_all(&mut self) {
        self.value.set_cursor(0, false);
        self.value.set_cursor(self.value.len(), true);
    }

    /// Selection.
    pub fn selection(&self) -> Range<usize> {
        self.value.selection()
    }

    /// Selection.
    pub fn selection_str(&self) -> &str {
        util::split3(self.value.as_str(), self.value.selection()).1
    }

    /// Previous word boundary
    pub fn prev_word_boundary(&self) -> usize {
        self.value.prev_word_boundary()
    }

    /// Next word boundary
    pub fn next_word_boundary(&self) -> usize {
        self.value.next_word_boundary()
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

    /// Insert a char a the current position.
    pub fn insert_char(&mut self, c: char) {
        self.value.insert_char(c);
    }

    /// Replace the given range with a new string.
    pub fn replace(&mut self, range: Range<usize>, new: &str) {
        self.value.replace(range, new);
    }

    /// Delete the char before the cursor.
    pub fn delete_prev_char(&mut self) {
        if self.value.has_selection() {
            self.value.replace(self.value.selection(), "");
        } else if self.value.cursor() == 0 {
        } else {
            self.value
                .replace(self.value.cursor() - 1..self.value.cursor(), "");
        }
    }

    /// Delete the char after the cursor.
    pub fn delete_next_char(&mut self) {
        if self.value.has_selection() {
            self.value.replace(self.value.selection(), "");
        } else if self.value.cursor() == self.value.len() {
        } else {
            self.value
                .replace(self.value.cursor()..self.value.cursor() + 1, "");
        }
    }
}

pub mod core {
    use crate::util;
    use std::mem;
    use std::ops::Range;
    use unicode_segmentation::UnicodeSegmentation;

    /// Text editing core.
    #[derive(Debug, Default, Clone)]
    pub struct InputCore {
        // Text
        value: String,
        // Len in grapheme count.
        len: usize,

        offset: usize,
        width: usize,

        cursor: usize,
        anchor: usize,

        char_buf: String,
        buf: String,
    }

    impl InputCore {
        /// Offset
        pub fn offset(&self) -> usize {
            self.offset
        }

        /// Change the offset
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

        /// Cursor position as grapheme-idx. Moves the cursor to the new position,
        /// but can leave the current cursor position as anchor of the selection.
        pub fn set_cursor(&mut self, cursor: usize, extend_selection: bool) {
            let cursor = if cursor > self.len { self.len } else { cursor };

            self.cursor = cursor;

            if !extend_selection {
                self.anchor = cursor;
            }

            if self.offset > cursor {
                self.offset = cursor;
            } else if self.offset + self.width < cursor {
                self.offset = cursor - self.width;
            }
        }

        /// Cursor position as grapheme-idx.
        pub fn cursor(&self) -> usize {
            self.cursor
        }

        /// Set the value. Resets cursor and anchor to 0.
        pub fn set_value<S: Into<String>>(&mut self, s: S) {
            self.value = s.into();
            self.len = self.value.graphemes(true).count();
            self.cursor = 0;
            self.offset = 0;
            self.anchor = 0;
        }

        /// Value
        pub fn value(&self) -> &str {
            self.value.as_str()
        }

        /// Value
        pub fn as_str(&self) -> &str {
            self.value.as_str()
        }

        /// Clear
        pub fn clear(&mut self) {
            self.set_value("");
        }

        /// Empty
        pub fn is_empty(&self) -> bool {
            self.value.is_empty()
        }

        /// Value lenght as grapheme-count
        pub fn len(&self) -> usize {
            self.len
        }

        /// Selection anchor
        pub fn anchor(&self) -> usize {
            self.anchor
        }

        /// Anchor is active
        pub fn has_selection(&self) -> bool {
            self.anchor != self.cursor
        }

        /// Selection.
        pub fn selection(&self) -> Range<usize> {
            if self.cursor < self.anchor {
                self.cursor..self.anchor
            } else {
                self.anchor..self.cursor
            }
        }

        /// Find next word.
        pub fn next_word_boundary(&self) -> usize {
            if self.cursor == self.len {
                self.len
            } else {
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

        /// Find previous word.
        pub fn prev_word_boundary(&self) -> usize {
            if self.cursor == 0 {
                0
            } else {
                self.value
                    .graphemes(true)
                    .rev()
                    .skip(self.len - self.cursor)
                    .skip_while(|c| !util::is_alphanumeric(c))
                    .skip_while(|c| util::is_alphanumeric(c))
                    .count()
            }
        }

        /// Insert a char, replacing the selection.
        pub fn insert_char(&mut self, new: char) {
            let selection = self.selection();

            let mut char_buf = mem::take(&mut self.char_buf);
            char_buf.clear();
            char_buf.push(new);
            self.replace(selection, char_buf.as_str());
            self.char_buf = char_buf;
        }

        /// Insert a string, replacing the selection.
        pub fn replace(&mut self, range: Range<usize>, new: &str) {
            let new_len = new.graphemes(true).count();

            let (before_str, sel_str, after_str) = util::split3(self.value.as_str(), range);
            let sel_len = sel_str.graphemes(true).count();
            let before_len = before_str.graphemes(true).count();

            self.len -= sel_len;
            self.len += new_len;

            if self.cursor >= before_len + sel_len {
                self.cursor -= sel_len;
                self.cursor += new_len;
            } else if self.cursor >= before_len {
                self.cursor = before_len + new_len;
            }

            if self.anchor >= before_len + sel_len {
                self.anchor -= sel_len;
                self.anchor += new_len;
            } else if self.anchor >= before_len {
                self.anchor = before_len + new_len;
            }

            // fix offset
            if self.offset > self.cursor {
                self.offset = self.cursor;
            } else if self.offset + self.width < self.cursor {
                self.offset = self.cursor - self.width;
            }

            self.buf.clear();
            self.buf.push_str(before_str);
            self.buf.push_str(new);
            self.buf.push_str(after_str);

            mem::swap(&mut self.value, &mut self.buf);
        }
    }
}
