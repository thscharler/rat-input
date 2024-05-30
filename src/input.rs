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
//! It is accessible as [TextInputState::screen_cursor()] after rendering.
//!
//! Event handling by calling the freestanding fn [handle_events].
//! There's [handle_mouse_events] if you want to override the default key bindings but keep
//! the mouse behaviour.
//!

use crate::_private::NonExhaustive;
use crate::event::{ReadOnly, TextOutcome};
use crate::util;
#[allow(unused_imports)]
use log::debug;
use rat_event::util::MouseFlags;
use rat_event::{ct_event, FocusKeys, HandleEvent, MouseOnly};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::BlockExt;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, StatefulWidget, Widget};
use std::cmp::{max, min};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Text input widget.
#[derive(Debug, Default, Clone)]
pub struct TextInput<'a> {
    block: Option<Block<'a>>,
    style: Style,
    focus_style: Option<Style>,
    select_style: Option<Style>,
    invalid_style: Option<Style>,
    focused: bool,
    invalid: bool,
}

/// Combined style for the widget.
#[derive(Debug, Clone)]
pub struct TextInputStyle {
    pub style: Style,
    pub focus: Option<Style>,
    pub select: Option<Style>,
    pub invalid: Option<Style>,
    pub non_exhaustive: NonExhaustive,
}

/// Textinput data & event-handling.
#[derive(Debug, Clone)]
pub struct TextInputState {
    /// The whole area with block.
    pub area: Rect,
    /// Area inside a possible block.
    pub inner: Rect,
    /// Mouse selection in progress.
    pub mouse: MouseFlags,
    /// Editing core
    pub value: core::InputCore,
    /// Construct with `..Default::default()`
    pub non_exhaustive: NonExhaustive,
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
    /// New widget.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the combined style.
    #[inline]
    pub fn styles(mut self, style: TextInputStyle) -> Self {
        self.style = style.style;
        self.focus_style = style.focus;
        self.select_style = style.select;
        self.invalid_style = style.invalid;
        self
    }

    /// Base text style.
    #[inline]
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.style = style.into();
        self
    }

    /// Style when focused.
    #[inline]
    pub fn focus_style(mut self, style: impl Into<Style>) -> Self {
        self.focus_style = Some(style.into());
        self
    }

    /// Style for selection
    #[inline]
    pub fn select_style(mut self, style: impl Into<Style>) -> Self {
        self.select_style = Some(style.into());
        self
    }

    /// Style for the invalid indicator.
    /// This is patched onto either base_style or focus_style
    #[inline]
    pub fn invalid_style(mut self, style: impl Into<Style>) -> Self {
        self.invalid_style = Some(style.into());
        self
    }

    #[inline]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Renders the content differently if focused.
    #[inline]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Renders the content differently if invalid.
    /// Uses the invalid style instead of the base style for rendering.
    #[inline]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
}

impl<'a> StatefulWidget for TextInput<'a> {
    type State = TextInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;
        state.inner = self.block.inner_if_some(area);
        state.value.set_width(state.inner.width as usize);

        self.block.render(area, buf);

        let area = state.inner.intersection(buf.area);

        let focus_style = if let Some(focus_style) = self.focus_style {
            focus_style
        } else {
            self.style
        };
        let select_style = if let Some(select_style) = self.select_style {
            select_style
        } else {
            Style::default().on_yellow()
        };
        let invalid_style = if let Some(invalid_style) = self.invalid_style {
            invalid_style
        } else {
            Style::default().red()
        };

        let (style, select_style) = if self.focused {
            if self.invalid {
                (
                    focus_style.patch(invalid_style),
                    select_style.patch(invalid_style),
                )
            } else {
                (focus_style, select_style)
            }
        } else {
            if self.invalid {
                (
                    self.style.patch(invalid_style),
                    self.style.patch(invalid_style),
                )
            } else {
                (self.style, select_style)
            }
        };

        buf.set_style(area, style);

        let selection = state.value.selection();
        let ox = state.offset();
        let mut cit = state.value.value().graphemes(true).skip(state.offset());
        let mut col = 0;
        let mut cx = 0;
        loop {
            if col >= area.width {
                break;
            }

            let ch = if let Some(c) = cit.next() { c } else { " " };

            let tx = cx + ox;
            let style = if selection.contains(&tx) {
                select_style
            } else {
                style
            };

            let cell = buf.get_mut(area.x + col, area.y);
            cell.set_symbol(ch);
            cell.set_style(style);

            // extra cells for wide chars.
            let ww = unicode_display_width::width(ch) as u16;
            for x in 1..ww {
                let cell = buf.get_mut(area.x + col + x, area.y);
                cell.set_symbol("");
                cell.set_style(style);
            }

            col += max(ww, 1);
            cx += 1;
        }
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self {
            area: Default::default(),
            inner: Default::default(),
            mouse: Default::default(),
            value: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl TextInputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset to empty.
    #[inline]
    pub fn clear(&mut self) -> bool {
        if self.is_empty() {
            false
        } else {
            self.value.clear();
            true
        }
    }

    /// Offset shown.
    #[inline]
    pub fn offset(&self) -> usize {
        self.value.offset()
    }

    /// Offset shown. This is corrected if the cursor wouldn't be visible.
    #[inline]
    pub fn set_offset(&mut self, offset: usize) {
        self.value.set_offset(offset);
    }

    /// Cursor position.
    #[inline]
    pub fn cursor(&self) -> usize {
        self.value.cursor()
    }

    /// Set the cursor position, reset selection.
    #[inline]
    pub fn set_cursor(&mut self, cursor: usize, extend_selection: bool) -> bool {
        self.value.set_cursor(cursor, extend_selection)
    }

    /// Selection anchor.
    #[inline]
    pub fn anchor(&self) -> usize {
        self.value.anchor()
    }

    /// Text.
    #[inline]
    pub fn value(&self) -> &str {
        self.value.value()
    }

    /// Set text.
    #[inline]
    pub fn set_value<S: Into<String>>(&mut self, s: S) {
        self.value.set_value(s);
    }

    /// Empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Text length as grapheme count.
    #[inline]
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// Selection.
    #[inline]
    pub fn has_selection(&self) -> bool {
        self.value.has_selection()
    }

    /// Selection.
    #[inline]
    pub fn selection(&self) -> Range<usize> {
        self.value.selection()
    }

    /// Selection.
    #[inline]
    pub fn set_selection(&mut self, anchor: usize, cursor: usize) -> bool {
        let old_selection = self.value.selection();

        self.value.set_cursor(anchor, false);
        self.value.set_cursor(cursor, true);

        old_selection != self.value.selection()
    }

    /// Selection.
    #[inline]
    pub fn select_all(&mut self) -> bool {
        let old_selection = self.value.selection();

        self.value.set_cursor(0, false);
        self.value.set_cursor(self.value.len(), true);

        old_selection != self.value.selection()
    }

    /// Selection.
    #[inline]
    pub fn selected_value(&self) -> &str {
        util::split3(self.value.value(), self.value.selection()).1
    }

    /// Insert a char at the current position.
    #[inline]
    pub fn insert_char(&mut self, c: char) -> bool {
        self.value.insert_char(c)
    }

    /// Deletes the given range.
    #[inline]
    pub fn delete_range(&mut self, range: Range<usize>) -> bool {
        if range.is_empty() {
            false
        } else {
            self.value.remove(range);
            true
        }
    }

    /// Deletes the next word.
    #[inline]
    pub fn delete_next_word(&mut self) -> bool {
        if self.value.has_selection() {
            self.delete_range(self.value.selection())
        } else {
            let cp = self.value.cursor();
            if let Some(ep) = self.value.next_word_boundary(cp) {
                self.delete_range(cp..ep)
            } else {
                false
            }
        }
    }

    /// Deletes the given range.
    #[inline]
    pub fn delete_prev_word(&mut self) -> bool {
        if self.value.has_selection() {
            self.delete_range(self.value.selection())
        } else {
            let cp = self.value.cursor();
            if let Some(sp) = self.value.prev_word_boundary(cp) {
                self.delete_range(sp..cp)
            } else {
                false
            }
        }
    }

    /// Delete the char before the cursor.
    #[inline]
    pub fn delete_prev_char(&mut self) -> bool {
        if self.value.has_selection() {
            self.value.remove(self.value.selection())
        } else if self.value.cursor() == 0 {
            false
        } else {
            self.value
                .remove(self.value.cursor() - 1..self.value.cursor());
            true
        }
    }

    /// Delete the char after the cursor.
    #[inline]
    pub fn delete_next_char(&mut self) -> bool {
        if self.value.has_selection() {
            self.value.remove(self.value.selection())
        } else if self.value.cursor() == self.value.len() {
            false
        } else {
            self.value
                .remove(self.value.cursor()..self.value.cursor() + 1);
            true
        }
    }

    #[inline]
    pub fn move_to_next_word(&mut self, extend_selection: bool) -> bool {
        let cp = self.value.cursor();
        if let Some(cp) = self.value.next_word_boundary(cp) {
            self.value.set_cursor(cp, extend_selection)
        } else {
            false
        }
    }

    #[inline]
    pub fn move_to_prev_word(&mut self, extend_selection: bool) -> bool {
        let cp = self.value.cursor();
        if let Some(cp) = self.value.prev_word_boundary(cp) {
            self.value.set_cursor(cp, extend_selection)
        } else {
            false
        }
    }

    /// Move to the next char.
    #[inline]
    pub fn move_to_next(&mut self, extend_selection: bool) -> bool {
        let c = min(self.value.cursor() + 1, self.value.len());
        self.value.set_cursor(c, extend_selection)
    }

    /// Move to the previous char.
    #[inline]
    pub fn move_to_prev(&mut self, extend_selection: bool) -> bool {
        let c = self.value.cursor().saturating_sub(1);
        self.value.set_cursor(c, extend_selection)
    }

    /// Start of line
    #[inline]
    pub fn move_to_line_start(&mut self, extend_selection: bool) -> bool {
        let c = 0;
        self.value.set_cursor(c, extend_selection)
    }

    // End of line
    #[inline]
    pub fn move_to_line_end(&mut self, extend_selection: bool) -> bool {
        let c = self.value.len();
        self.value.set_cursor(c, extend_selection)
    }

    /// Converts a grapheme based position to a screen position
    /// relative to the widget area.
    pub fn to_screen_col(&self, pos: usize) -> Option<u16> {
        let px = pos;
        let ox = self.value.offset();

        let mut sx = 0;
        let line = self.value.value_graphemes();
        for c in line.skip(ox).take(px - ox) {
            sx += unicode_display_width::width(c) as usize;
        }

        Some(sx as u16)
    }

    /// Converts from a widget relative screen coordinate to a grapheme index.
    /// x is the relative screen position.
    pub fn from_screen_col(&self, x: usize) -> Option<usize> {
        let mut cx = 0;
        let ox = self.value.offset();

        let line = self.value.value_graphemes();
        let mut test = 0;
        for c in line.skip(ox) {
            if test >= x {
                break;
            }

            test += unicode_display_width::width(c) as usize;

            cx += 1;
        }

        Some(cx + ox)
    }

    /// Set the cursor position from a screen position relative to the origin
    /// of the widget. This value can be negative, which selects a currently
    /// not visible position and scrolls to it.
    #[inline]
    pub fn set_screen_cursor(&mut self, cursor: isize, extend_selection: bool) -> bool {
        let sc = cursor;

        let c = if sc < 0 {
            self.value.offset().saturating_sub(-sc as usize)
        } else {
            if let Some(c) = self.from_screen_col(sc as usize) {
                c
            } else {
                self.value.len()
            }
        };

        let old_cursor = self.value.cursor();
        let old_anchor = self.value.anchor();

        self.value.set_cursor(c, extend_selection);

        old_cursor != self.value.cursor() || old_anchor != self.value.anchor()
    }

    /// The current text cursor as an absolute screen position.
    #[inline]
    pub fn screen_cursor(&self) -> Option<(u16, u16)> {
        let cx = self.value.cursor();
        let ox = self.value.offset();

        if cx < ox {
            None
        } else if cx > ox + self.inner.width as usize {
            None
        } else {
            let sc = self.to_screen_col(cx).expect("valid_cursor");
            Some((self.inner.x + sc, self.inner.y))
        }
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, TextOutcome> for TextInputState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: FocusKeys) -> TextOutcome {
        let mut r = match event {
            ct_event!(key press c)
            | ct_event!(key press SHIFT-c)
            | ct_event!(key press CONTROL_ALT-c) => self.insert_char(*c).into(),
            ct_event!(keycode press Backspace) => self.delete_prev_char().into(),
            ct_event!(keycode press Delete) => self.delete_next_char().into(),
            ct_event!(keycode press CONTROL-Backspace) => self.delete_prev_word().into(),
            ct_event!(keycode press CONTROL-Delete) => self.delete_next_word().into(),
            ct_event!(key press CONTROL-'d') => self.clear().into(),

            ct_event!(key release _)
            | ct_event!(key release SHIFT-_)
            | ct_event!(key release CONTROL_ALT-_)
            | ct_event!(keycode release Backspace)
            | ct_event!(keycode release Delete)
            | ct_event!(keycode release CONTROL-Backspace)
            | ct_event!(keycode release CONTROL-Delete)
            | ct_event!(key release CONTROL-'d') => TextOutcome::Unchanged,

            _ => TextOutcome::NotUsed,
        };
        // remap to TextChanged
        if r == TextOutcome::Changed {
            r = TextOutcome::TextChanged;
        }

        if r == TextOutcome::NotUsed {
            r = self.handle(event, ReadOnly);
        }
        if r == TextOutcome::NotUsed {
            r = self.handle(event, MouseOnly);
        }
        r
    }
}

impl HandleEvent<crossterm::event::Event, ReadOnly, TextOutcome> for TextInputState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: ReadOnly) -> TextOutcome {
        let mut r = match event {
            ct_event!(keycode press Left) => self.move_to_prev(false).into(),
            ct_event!(keycode press Right) => self.move_to_next(false).into(),
            ct_event!(keycode press CONTROL-Left) => self.move_to_prev_word(false).into(),
            ct_event!(keycode press CONTROL-Right) => self.move_to_next_word(false).into(),
            ct_event!(keycode press Home) => self.move_to_line_start(false).into(),
            ct_event!(keycode press End) => self.move_to_line_end(false).into(),
            ct_event!(keycode press SHIFT-Left) => self.move_to_prev(true).into(),
            ct_event!(keycode press SHIFT-Right) => self.move_to_next(true).into(),
            ct_event!(keycode press CONTROL_SHIFT-Left) => self.move_to_prev_word(true).into(),
            ct_event!(keycode press CONTROL_SHIFT-Right) => self.move_to_next_word(true).into(),
            ct_event!(keycode press SHIFT-Home) => self.move_to_line_start(true).into(),
            ct_event!(keycode press SHIFT-End) => self.move_to_line_end(true).into(),
            ct_event!(key press CONTROL-'a') => self.select_all().into(),

            ct_event!(keycode release Left)
            | ct_event!(keycode release Right)
            | ct_event!(keycode release CONTROL-Left)
            | ct_event!(keycode release CONTROL-Right)
            | ct_event!(keycode release Home)
            | ct_event!(keycode release End)
            | ct_event!(keycode release SHIFT-Left)
            | ct_event!(keycode release SHIFT-Right)
            | ct_event!(keycode release CONTROL_SHIFT-Left)
            | ct_event!(keycode release CONTROL_SHIFT-Right)
            | ct_event!(keycode release SHIFT-Home)
            | ct_event!(keycode release SHIFT-End)
            | ct_event!(key release CONTROL-'a') => TextOutcome::Unchanged,

            _ => TextOutcome::NotUsed,
        };

        if r == TextOutcome::NotUsed {
            r = self.handle(event, MouseOnly);
        }
        r
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, TextOutcome> for TextInputState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: MouseOnly) -> TextOutcome {
        match event {
            ct_event!(mouse any for m) if self.mouse.drag(self.area, m) => {
                let c = (m.column as isize) - (self.inner.x as isize);
                self.set_screen_cursor(c, true).into()
            }
            ct_event!(mouse down Left for column,row) => {
                if self.inner.contains((*column, *row).into()) {
                    let c = column - self.inner.x;
                    self.set_screen_cursor(c as isize, false).into()
                } else {
                    TextOutcome::NotUsed
                }
            }
            _ => TextOutcome::NotUsed,
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
) -> TextOutcome {
    if focus {
        state.handle(event, FocusKeys)
    } else {
        state.handle(event, MouseOnly)
    }
}

/// Handle only navigation events.
/// Text events are only processed if focus is true.
/// Mouse events are processed if they are in range.
pub fn handle_readonly_events(
    state: &mut TextInputState,
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
    state: &mut TextInputState,
    event: &crossterm::event::Event,
) -> TextOutcome {
    state.handle(event, MouseOnly)
}

pub mod core {
    use crate::util;
    use crate::util::gr_len;
    #[allow(unused_imports)]
    use log::debug;
    use std::cmp::min;
    use std::iter::once;
    use std::mem;
    use std::ops::Range;
    use unicode_segmentation::{Graphemes, UnicodeSegmentation};

    /// Text editing core.
    #[derive(Debug, Default, Clone)]
    pub struct InputCore {
        // Text
        value: String,
        // Len in grapheme count.
        len: usize,

        // display information
        offset: usize,
        width: usize,

        // cursor and selection
        cursor: usize,
        anchor: usize,

        // tmp string for inserting a char.
        char_buf: String,
        // tmp string for editing.
        buf: String,
    }

    impl InputCore {
        pub fn new() -> Self {
            Self::default()
        }

        /// Offset
        #[inline]
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
        #[inline]
        pub fn width(&self) -> usize {
            self.width
        }

        /// Display width
        #[inline]
        pub fn set_width(&mut self, width: usize) {
            self.width = width;

            if self.offset + width < self.cursor {
                self.offset = self.cursor - self.width;
            }
        }

        /// Cursor position as grapheme-idx. Moves the cursor to the new position,
        /// but can leave the current cursor position as anchor of the selection.
        pub fn set_cursor(&mut self, cursor: usize, extend_selection: bool) -> bool {
            let old_cursor = self.cursor;

            let c = min(self.len, cursor);

            self.cursor = c;

            if !extend_selection {
                self.anchor = c;
            }

            if self.offset > c {
                self.offset = c;
            } else if self.offset + self.width < c {
                self.offset = c - self.width;
            }

            c != old_cursor
        }

        /// Cursor position as grapheme-idx.
        #[inline]
        pub fn cursor(&self) -> usize {
            self.cursor
        }

        /// Selection anchor
        #[inline]
        pub fn anchor(&self) -> usize {
            self.anchor
        }

        /// Set the value. Resets cursor and anchor to 0.
        #[inline]
        pub fn set_value<S: Into<String>>(&mut self, s: S) {
            self.value = s.into();
            self.len = self.value.graphemes(true).count();
            self.cursor = 0;
            self.offset = 0;
            self.anchor = 0;
        }

        /// Value
        #[inline]
        pub fn value(&self) -> &str {
            self.value.as_str()
        }

        /// Value as grapheme iterator.
        #[inline]
        pub fn value_graphemes(&self) -> Graphemes<'_> {
            self.value.graphemes(true)
        }

        /// Clear
        #[inline]
        pub fn clear(&mut self) {
            self.set_value("");
        }

        /// Empty
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.value.is_empty()
        }

        /// Value lenght as grapheme-count
        #[inline]
        pub fn len(&self) -> usize {
            self.len
        }

        /// Anchor is active
        #[inline]
        pub fn has_selection(&self) -> bool {
            self.anchor != self.cursor
        }

        /// Selection.
        #[inline]
        pub fn selection(&self) -> Range<usize> {
            if self.cursor < self.anchor {
                self.cursor..self.anchor
            } else {
                self.anchor..self.cursor
            }
        }

        /// Convert the byte-position to a grapheme position.
        pub fn byte_pos(&self, byte_pos: usize) -> Option<usize> {
            let mut pos = None;

            for (gp, (bp, _cc)) in self
                .value
                .grapheme_indices(true)
                .chain(once((self.len(), "")))
                .enumerate()
            {
                if bp >= byte_pos {
                    pos = Some(gp);
                    break;
                }
            }

            pos
        }

        /// Grapheme position to byte position.
        /// Returns the byte-range for the grapheme at pos.
        pub fn byte_at(&self, pos: usize) -> Option<(usize, usize)> {
            let mut byte_pos = None;

            for (gp, (bp, cc)) in self
                .value
                .grapheme_indices(true)
                .chain(once((self.value.len(), "")))
                .enumerate()
            {
                if gp == pos {
                    byte_pos = Some((bp, bp + cc.len()));
                    break;
                }
            }

            byte_pos
        }

        /// Grapheme position to char position.
        /// Returns the first char position for the grapheme at pos.
        pub fn char_at(&self, pos: usize) -> Option<usize> {
            let mut char_pos = 0;
            for (gp, (_bp, cc)) in self
                .value
                .grapheme_indices(true)
                .chain(once((self.len(), "")))
                .enumerate()
            {
                if gp == pos {
                    return Some(char_pos);
                }
                char_pos += cc.chars().count();
            }

            None
        }

        /// Char position to grapheme position.
        pub fn char_pos(&self, char_pos: usize) -> Option<usize> {
            let mut cp = 0;
            for (gp, (_bp, cc)) in self
                .value
                .grapheme_indices(true)
                .chain(once((self.len(), "")))
                .enumerate()
            {
                if cp >= char_pos {
                    return Some(gp);
                }
                cp += cc.chars().count();
            }

            None
        }

        /// Find next word.
        pub fn next_word_boundary(&self, pos: usize) -> Option<usize> {
            let byte_pos = self.byte_at(pos)?;

            let (_, str_after) = self.value.split_at(byte_pos.0);
            let mut it = str_after.graphemes(true);
            let mut init = true;
            let mut gp = 0;
            loop {
                let Some(c) = it.next() else {
                    break;
                };

                if init {
                    if let Some(c) = c.chars().next() {
                        if !c.is_whitespace() {
                            init = false;
                        }
                    }
                } else {
                    if let Some(c) = c.chars().next() {
                        if c.is_whitespace() {
                            break;
                        }
                    }
                }

                gp += 1;
            }

            Some(pos + gp)
        }

        /// Find previous word.
        pub fn prev_word_boundary(&self, pos: usize) -> Option<usize> {
            let byte_pos = self.byte_at(pos)?;

            let (str_before, _) = self.value.split_at(byte_pos.0);
            let mut it = str_before.graphemes(true).rev();
            let mut init = true;
            let mut gp = gr_len(str_before);
            loop {
                let Some(c) = it.next() else {
                    break;
                };

                if init {
                    if let Some(c) = c.chars().next() {
                        if !c.is_whitespace() {
                            init = false;
                        }
                    }
                } else {
                    if let Some(c) = c.chars().next() {
                        if c.is_whitespace() {
                            break;
                        }
                    }
                }

                gp -= 1;
            }

            Some(gp)
        }

        /// Insert a char, replacing the selection.
        pub fn insert_char(&mut self, new: char) -> bool {
            let selection = self.selection();

            let mut char_buf = mem::take(&mut self.char_buf);
            char_buf.clear();
            char_buf.push(new);
            self.replace(selection, char_buf.as_str());
            self.char_buf = char_buf;

            true
        }

        /// Remove the selection.
        pub fn remove(&mut self, range: Range<usize>) -> bool {
            if range.is_empty() {
                false
            } else {
                self.replace(range, "");
                true
            }
        }

        /// Insert a string, replacing the selection.
        fn replace(&mut self, range: Range<usize>, new: &str) {
            let (before_str, _, after_str) = util::split3(self.value.as_str(), range.clone());
            self.buf.clear();
            self.buf.push_str(before_str);
            self.buf.push_str(new);
            self.buf.push_str(after_str);
            mem::swap(&mut self.value, &mut self.buf);

            let old_len = self.len;
            let new_len = self.value.graphemes(true).count();
            let c_ins = new_len as isize - old_len as isize;

            self.len = new_len;

            if self.cursor < range.start {
                // noop
            } else if self.cursor < range.end {
                self.cursor = range.start;
            } else {
                self.cursor = (self.cursor as isize + c_ins) as usize;
            }

            if self.anchor < range.start {
                // noop
            } else if self.anchor < range.end {
                self.anchor = range.start;
            } else {
                self.anchor = (self.anchor as isize + c_ins) as usize;
            }

            // fix offset
            if self.offset > self.cursor {
                self.offset = self.cursor;
            } else if self.offset + self.width < self.cursor {
                self.offset = self.cursor - self.width;
            }
        }
    }
}
