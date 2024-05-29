//!
//! A text-area with text-styling abilities.
//!
use crate::_private::NonExhaustive;
use crate::textarea::core::{RopeGraphemes, TextRange};
use crate::util::MouseFlags;
use log::debug;
use rat_event::util::Outcome;
use rat_event::{ct_event, FocusKeys, HandleEvent, MouseOnly, UsedEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::{BlockExt, Stylize};
use ratatui::style::Style;
use ratatui::widgets::{Block, StatefulWidget, Widget};
use ropey::{Rope, RopeSlice};
use std::cmp::{max, min};

/// Text area widget.
///
/// Backend used is [ropey](https://docs.rs/ropey/latest/ropey/), so large
/// texts are no problem. Editing time increases with the number of
/// styles applied. Everything below a million styles should be fine.
///
/// For emoji support this uses
/// [unicode_display_width](https://docs.rs/unicode-display-width/latest/unicode_display_width/index.html)
/// which helps with those double-width emojis. Input of emojis
/// strongly depends on the terminal. It may or may not work.
/// And even with display there are sometimes strange glitches
/// that I haven't found yet.
///
/// Keyboard and mouse are implemented for crossterm, but it should be
/// trivial to extend to other event-types. Every interaction is available
/// as function on the state.
///
/// Scrolling doesn't depend on the cursor, but the editing and move
/// functions take care that the cursor stays visible.
///
/// Wordwrap is not available. For display only use
/// [Paragraph](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Paragraph.html), as
/// for editing: why?
///
/// You can directly access the underlying Rope for readonly purposes, and
/// conversion from/to byte/char positions are available. That should probably be
/// enough to write a parser that generates some styling.
///
/// The cursor must set externally on the ratatui Frame as usual.
/// [screen_cursor](TextAreaState::screen_cursor) gives you the correct value.
/// There is the inverse too [set_screen_cursor](TextAreaState::set_screen_cursor)
/// For more interactions you can use [from_screen_col](TextAreaState::from_screen_col),
/// and [to_screen_col](TextAreaState::to_screen_col). They calculate everything,
/// even in the presence of more complex graphemes and those double-width emojis.
///
#[derive(Debug, Default, Clone)]
pub struct TextArea<'a> {
    block: Option<Block<'a>>,
    style: Style,
    focus_style: Option<Style>,
    select_style: Option<Style>,
    text_style: Vec<Style>,
    focused: bool,
}

/// State for the text-area.
///
#[derive(Debug, Clone)]
pub struct TextAreaState {
    /// Complete area.
    pub area: Rect,
    /// Area inside the borders.
    pub inner: Rect,
    /// Text edit core
    pub value: core::InputCore,
    /// Helper for mouse.
    pub mouse: MouseFlags,

    pub non_exhaustive: NonExhaustive,
}

impl<'a> TextArea<'a> {
    /// New widget.
    pub fn new() -> Self {
        Self::default()
    }

    /// Base style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Style for selection.
    pub fn select_style(mut self, style: Style) -> Self {
        self.select_style = Some(style);
        self
    }

    /// Selection style when focused.
    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = Some(style);
        self
    }

    /// List of text-styles.
    ///
    /// Use [TextAreaState::add_style()] to refer a text range to
    /// one of these styles.
    pub fn text_style<T: IntoIterator<Item = Style>>(mut self, styles: T) -> Self {
        self.text_style = styles.into_iter().collect();
        self
    }
}

impl<'a> StatefulWidget for TextArea<'a> {
    type State = TextAreaState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;
        state.inner = self.block.inner_if_some(area);

        self.block.render(area, buf);

        let area = state.area.intersection(buf.area);

        buf.set_style(area, self.style);

        let focus_style = if let Some(focus_style) = self.focus_style {
            focus_style
        } else {
            Style::default()
        };

        let select_style = if self.focused {
            if let Some(select_style) = self.select_style {
                select_style.patch(focus_style)
            } else {
                self.style.reversed().patch(focus_style)
            }
        } else {
            if let Some(select_style) = self.select_style {
                select_style
            } else {
                self.style.reversed()
            }
        };

        let selection = state.selection();
        let mut styles = Vec::new();

        let mut line_iter = state.value.iter_scrolled();
        for row in 0..area.height {
            if let Some(mut line) = line_iter.next() {
                let mut col = 0;
                let mut cx = 0;
                loop {
                    if col >= area.width {
                        break;
                    }

                    let tmp_str;
                    let ch = if let Some(ch) = line.next() {
                        if let Some(ch) = ch.as_str() {
                            // would do a newline on the console.
                            if ch != "\n" {
                                ch
                            } else {
                                " "
                            }
                        } else {
                            tmp_str = ch.to_string();
                            tmp_str.as_str()
                        }
                    } else {
                        " "
                    };

                    // text based
                    let (ox, oy) = state.offset();
                    let tx = cx as usize + ox;
                    let ty = row as usize + oy;

                    let mut style = Style::default();

                    // text-styles
                    state.styles_at((tx, ty), &mut styles);
                    for idx in styles.iter().copied() {
                        let Some(s) = self.text_style.get(idx) else {
                            panic!("invalid style nr: {}", idx);
                        };
                        style = style.patch(*s);
                    }

                    // selection
                    if selection.contains((tx, ty)) {
                        style = style.patch(select_style);
                    };

                    let cell = buf.get_mut(area.x + col, area.y + row);
                    cell.set_symbol(ch);
                    cell.set_style(style);

                    // extra cells for wide chars.
                    let ww = unicode_display_width::width(ch) as u16;
                    for x in 1..ww {
                        let cell = buf.get_mut(area.x + col + x, area.y + row);
                        cell.set_symbol(" ");
                        cell.set_style(style);
                    }

                    col += ww;
                    cx += 1;
                }
            } else {
                for col in 0..area.width {
                    let cell = buf.get_mut(area.x + col, area.y + row);
                    cell.set_symbol(" ");
                }
            }
        }
    }
}

impl Default for TextAreaState {
    fn default() -> Self {
        Self {
            area: Default::default(),
            inner: Default::default(),
            mouse: Default::default(),
            value: core::InputCore::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl TextAreaState {
    /// New State.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the offset for scrolling.
    #[inline]
    pub fn set_offset(&mut self, offset: (usize, usize)) -> bool {
        self.value.set_offset(offset)
    }

    /// Current offset for scrolling.
    #[inline]
    pub fn offset(&self) -> (usize, usize) {
        self.value.offset()
    }

    /// Set the cursor position.
    /// This doesn't scroll the cursor to a visible position.
    /// Use [TextAreaState::scroll_cursor_to_visible()] for that.
    #[inline]
    pub fn set_cursor(&mut self, cursor: (usize, usize), extend_selection: bool) -> bool {
        self.value.set_cursor(cursor, extend_selection)
    }

    /// Cursor position.
    #[inline]
    pub fn cursor(&self) -> (usize, usize) {
        self.value.cursor()
    }

    /// Selection anchor.
    #[inline]
    pub fn anchor(&self) -> (usize, usize) {
        self.value.anchor()
    }

    /// Set the text value.
    /// Resets all internal state.
    #[inline]
    pub fn set_value<S: AsRef<str>>(&mut self, s: S) {
        self.value.set_value(s);
    }

    /// Set the text value as a Rope.
    /// Resets all internal state.
    #[inline]
    pub fn set_rope(&mut self, s: Rope) {
        self.value.set_rope(s);
    }

    /// Text value
    #[inline]
    pub fn value(&self) -> String {
        self.value.value()
    }

    /// Text value
    #[inline]
    pub fn value_range(&self, range: TextRange) -> Option<RopeSlice<'_>> {
        self.value.value_range(range)
    }

    /// Text as Bytes iterator.
    #[inline]
    pub fn value_as_bytes(&self) -> ropey::iter::Bytes<'_> {
        self.value.value_as_bytes()
    }

    /// Text as Bytes iterator.
    #[inline]
    pub fn value_as_chars(&self) -> ropey::iter::Bytes<'_> {
        self.value.value_as_bytes()
    }

    /// Convert a byte position to a text area position.
    /// Uses grapheme based column indexes.
    #[inline]
    pub fn byte_pos(&self, byte: usize) -> Option<(usize, usize)> {
        self.value.byte_pos(byte)
    }

    /// Convert a text area position to a byte range.
    /// Uses grapheme based column indexes.
    /// Returns (byte-start, byte-end) of the grapheme at the given position.
    #[inline]
    pub fn byte_at(&self, pos: (usize, usize)) -> Option<(usize, usize)> {
        self.value.byte_at(pos)
    }

    /// Convert a char position to a text area position.
    /// Uses grapheme based column indexes.
    #[inline]
    pub fn char_pos(&self, byte: usize) -> Option<(usize, usize)> {
        self.value.char_pos(byte)
    }

    /// Convert a text area position to a char position.
    /// Uses grapheme based column indexes.
    #[inline]
    pub fn char_at(&self, pos: (usize, usize)) -> Option<usize> {
        self.value.char_at(pos)
    }

    /// Grapheme iterator for a given line.
    /// This contains the \n at the end.
    #[inline]
    pub fn line(&self, n: usize) -> Option<RopeGraphemes<'_>> {
        self.value.line(n)
    }

    /// Line width as grapheme count.
    #[inline]
    pub fn line_width(&self, n: usize) -> Option<usize> {
        self.value.line_width(n)
    }

    /// Line count.
    #[inline]
    pub fn line_len(&self) -> usize {
        self.value.len_lines()
    }

    /// Clear everything.
    #[inline]
    pub fn clear(&mut self) {
        self.value.clear();
    }

    /// Empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Has a selection?
    #[inline]
    pub fn has_selection(&self) -> bool {
        self.value.has_selection()
    }

    /// Current selection.
    #[inline]
    pub fn selection(&self) -> TextRange {
        self.value.selection()
    }

    /// Set the selection.
    #[inline]
    pub fn set_selection(&mut self, range: TextRange) -> bool {
        self.value.set_selection(range)
    }

    /// Select all.
    #[inline]
    pub fn select_all(&mut self) -> bool {
        self.value.select_all()
    }

    /// Clear all set styles.
    #[inline]
    pub fn clear_styles(&mut self) {
        self.value.clear_styles();
    }

    /// Add a style for a [TextRange]. The style-nr refers to one
    /// of the styles set with the widget.
    #[inline]
    pub fn add_style(&mut self, range: TextRange, style: usize) {
        self.value.add_style(range, style);
    }

    /// All styles active at the given position.
    #[inline]
    pub fn styles_at(&self, pos: (usize, usize), result: &mut Vec<usize>) {
        self.value.styles_at(pos, result)
    }

    /// Insert a character at the cursor position.
    /// Removes the selection and inserts the char.
    pub fn insert_char(&mut self, c: char) -> bool {
        if self.value.has_selection() {
            self.value.remove(self.value.selection());
        }
        self.value.insert_char(self.value.cursor(), c);
        self.scroll_cursor_to_visible();
        true
    }

    /// Insert a line break at the cursor position.
    pub fn insert_newline(&mut self) -> bool {
        if self.value.has_selection() {
            self.value.remove(self.value.selection());
        }
        self.value.insert_newline(self.value.cursor());
        self.scroll_cursor_to_visible();
        true
    }

    ///
    pub fn delete_selection(&mut self) -> bool {
        self.delete_range(self.selection())
    }

    ///
    pub fn delete_range(&mut self, range: TextRange) -> bool {
        if !range.is_empty() {
            self.value.remove(range);
            self.scroll_cursor_to_visible();
            true
        } else {
            false
        }
    }

    /// Deletes the next char or the current selection.
    /// Returns true if there was any real change.
    pub fn delete_next_char(&mut self) -> bool {
        let range = if self.value.has_selection() {
            self.selection()
        } else {
            let (cx, cy) = self.value.cursor();
            let c_line_width = self.value.line_width(cy).expect("width");
            let c_last_line = self.value.len_lines() - 1;

            let (ex, ey) = if cy == c_last_line && cx == c_line_width {
                (c_line_width, c_last_line)
            } else if cy != c_last_line && cx == c_line_width {
                (0, cy + 1)
            } else {
                (cx + 1, cy)
            };
            TextRange::new((cx, cy), (ex, ey))
        };

        self.delete_range(range)
    }

    /// Deletes the previous char or the selection.
    /// Returns true if there was any real change.
    pub fn delete_prev_char(&mut self) -> bool {
        let range = if self.value.has_selection() {
            self.selection()
        } else {
            let (cx, cy) = self.value.cursor();
            let (sx, sy) = if cy == 0 && cx == 0 {
                (0, 0)
            } else if cy != 0 && cx == 0 {
                let prev_line_width = self.value.line_width(cy - 1).expect("line_width");
                (prev_line_width, cy - 1)
            } else {
                (cx - 1, cy)
            };

            TextRange::new((sx, sy), (cx, cy))
        };

        self.delete_range(range)
    }

    pub fn delete_next_word(&mut self) -> bool {
        if self.value.has_selection() {
            self.value
                .set_selection(TextRange::new(self.cursor(), self.cursor()));
        }

        let (cx, cy) = self.value.cursor();
        let (ex, ey) = self
            .value
            .next_word_boundary((cx, cy))
            .expect("valid_cursor");

        let range = TextRange::new((cx, cy), (ex, ey));
        if !range.is_empty() {
            self.value.remove(range);
            self.scroll_cursor_to_visible();
            true
        } else {
            false
        }
    }

    pub fn delete_prev_word(&mut self) -> bool {
        if self.value.has_selection() {
            self.value
                .set_selection(TextRange::new(self.cursor(), self.cursor()));
        }

        let (cx, cy) = self.value.cursor();
        let (sx, sy) = self
            .value
            .prev_word_boundary((cx, cy))
            .expect("valid_cursor");

        let range = TextRange::new((sx, sy), (cx, cy));
        if !range.is_empty() {
            self.value.remove(range);
            self.scroll_cursor_to_visible();
            true
        } else {
            false
        }
    }

    /// Move the cursor left. Scrolls the cursor to visible.
    /// Returns true if there was any real change.
    pub fn move_left(&mut self, n: usize, extend_selection: bool) -> bool {
        let (mut cx, mut cy) = self.value.cursor();

        if cx == 0 {
            if cy > 0 {
                cy = cy.saturating_sub(1);
                let Some(c_line_width) = self.value.line_width(cy) else {
                    panic!("invalid_cursor: {:?} value {:?}", (cx, cy), self.value);
                };
                cx = c_line_width;
            }
        } else {
            cx = cx.saturating_sub(n);
        }

        self.value.set_move_col(Some(cx));
        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Move the cursor right. Scrolls the cursor to visible.
    /// Returns true if there was any real change.
    pub fn move_right(&mut self, n: usize, extend_selection: bool) -> bool {
        let (mut cx, mut cy) = self.value.cursor();
        let Some(c_line_width) = self.value.line_width(cy) else {
            panic!("invalid_cursor: {:?} value {:?}", (cx, cy), self.value);
        };

        if cx == c_line_width {
            if cy + 1 < self.value.len_lines() {
                cy += 1;
                cx = 0;
            }
        } else {
            cx = min(cx + n, c_line_width)
        }

        self.value.set_move_col(Some(cx));
        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Move the cursor up. Scrolls the cursor to visible.
    /// Returns true if there was any real change.
    pub fn move_up(&mut self, n: usize, extend_selection: bool) -> bool {
        let (mut cx, mut cy) = self.value.cursor();
        let Some(c_line_width) = self.value.line_width(cy) else {
            panic!("invalid_cursor: {:?} value {:?}", (cx, cy), self.value);
        };

        cy = cy.saturating_sub(n);
        if let Some(xx) = self.value.move_col() {
            cx = min(xx, c_line_width);
        } else {
            cx = min(cx, c_line_width);
        }

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Move the cursor down. Scrolls the cursor to visible.
    /// Returns true if there was any real change.
    pub fn move_down(&mut self, n: usize, extend_selection: bool) -> bool {
        let (mut cx, mut cy) = self.value.cursor();
        let Some(c_line_width) = self.value.line_width(cy) else {
            panic!("invalid_cursor: {:?} value {:?}", (cx, cy), self.value);
        };

        cy = min(cy + n, self.value.len_lines() - 1);
        if let Some(xx) = self.value.move_col() {
            cx = min(xx, c_line_width);
        } else {
            cx = min(cx, c_line_width);
        }

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Move the cursor to the start of the line.
    /// Scrolls the cursor to visible.
    /// Returns true if there was any real change.
    pub fn move_to_line_start(&mut self, extend_selection: bool) -> bool {
        let (mut cx, cy) = self.value.cursor();

        cx = 'f: {
            if cx > 0 {
                let Some(line) = self.value.line(cy) else {
                    panic!("invalid_cursor: {:?} value {:?}", (cx, cy), self.value);
                };
                for (c, ch) in line.enumerate() {
                    if ch.as_str() != Some(" ") {
                        if cx != c {
                            break 'f c;
                        } else {
                            break 'f 0;
                        }
                    }
                }
            }
            0
        };

        self.value.set_move_col(Some(cx));
        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Move the cursor to the end of the line. Scrolls to visible, if
    /// necessary.
    /// Returns true if there was any real change.
    pub fn move_to_line_end(&mut self, extend_selection: bool) -> bool {
        let (cx, cy) = self.value.cursor();
        let Some(c_line_width) = self.value.line_width(cy) else {
            panic!("invalid_cursor: {:?} value {:?}", (cx, cy), self.value);
        };

        let cx = c_line_width;

        self.value.set_move_col(Some(cx));
        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Move the cursor to the document start.
    pub fn move_to_start(&mut self, extend_selection: bool) -> bool {
        let cx = 0;
        let cy = 0;

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Move the cursor to the document end.
    pub fn move_to_end(&mut self, extend_selection: bool) -> bool {
        let len = self.value.len_lines();

        let cx = 0;
        let cy = len - 1;

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Move the cursor to the start of the visible area.
    pub fn move_to_screen_start(&mut self, extend_selection: bool) -> bool {
        let (ox, oy) = self.value.offset();

        let cx = ox;
        let cy = oy;

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Move the cursor to the end of the visible area.
    pub fn move_to_screen_end(&mut self, extend_selection: bool) -> bool {
        let (ox, oy) = self.value.offset();
        let len = self.value.len_lines();

        let cx = ox;
        let cy = min(oy + self.vertical_page() - 1, len - 1);

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    pub fn move_to_next_word(&mut self, extend_selection: bool) -> bool {
        let (cx, cy) = self.value.cursor();

        let (px, py) = self
            .value
            .next_word_boundary((cx, cy))
            .expect("valid_cursor");

        let c = self.value.set_cursor((px, py), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    pub fn move_to_prev_word(&mut self, extend_selection: bool) -> bool {
        let (cx, cy) = self.value.cursor();

        let (px, py) = self
            .value
            .prev_word_boundary((cx, cy))
            .expect("valid_cursor");

        let c = self.value.set_cursor((px, py), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    /// Converts from a widget relative screen coordinate to a grapheme index.
    /// Row is a row-index into the value, not a screen-row.
    /// x is the relative screen position.
    pub fn from_screen_col(&self, row: usize, x: usize) -> Option<usize> {
        let (mut cx, cy) = (0usize, row);
        let (ox, _oy) = self.value.offset();

        let Some(line) = self.line(cy) else {
            return None;
        };
        let mut test = 0;
        for c in line.skip(ox).filter(|v| v != "\n") {
            if test >= x {
                break;
            }

            test += if let Some(c) = c.as_str() {
                unicode_display_width::width(c) as usize
            } else {
                unicode_display_width::width(c.to_string().as_str()) as usize
            };

            cx += 1;
        }

        Some(cx + ox)
    }

    /// Converts a grapheme based position to a screen position
    /// relative to the widget area.
    pub fn to_screen_col(&self, pos: (usize, usize)) -> Option<u16> {
        let (px, py) = pos;
        let (ox, _oy) = self.value.offset();

        let mut sx = 0;
        let Some(line) = self.line(py) else {
            return None;
        };
        for c in line.skip(ox).filter(|v| v != "\n").take(px - ox) {
            sx += if let Some(c) = c.as_str() {
                unicode_display_width::width(c) as usize
            } else {
                unicode_display_width::width(c.to_string().as_str()) as usize
            };
        }

        Some(sx as u16)
    }

    /// Cursor position on the screen.
    pub fn screen_cursor(&self) -> Option<(u16, u16)> {
        let (cx, cy) = self.value.cursor();
        let (ox, oy) = self.value.offset();

        if cy < oy {
            None
        } else if cy >= oy + self.inner.height as usize {
            None
        } else {
            let sy = cy - oy;
            if cx < ox {
                None
            } else if cx > ox + self.inner.width as usize {
                None
            } else {
                let sx = self.to_screen_col((cx, cy)).expect("valid_cursor");

                Some((self.inner.x + sx, self.inner.y + sy as u16))
            }
        }
    }

    /// Set the cursor position from screen coordinates.
    ///
    /// The cursor positions are relative to the inner rect.
    /// They may be negative too, this allows setting the cursor
    /// to a position that is currently scrolled away.
    pub fn set_screen_cursor(&mut self, cursor: (i16, i16), extend_selection: bool) -> bool {
        let (scx, scy) = (cursor.0 as isize, cursor.1 as isize);
        let (ox, oy) = self.value.offset();

        let cy = min(max(oy as isize + scy, 0) as usize, self.line_len() - 1);
        let cx = if scx < 0 {
            max(ox as isize + scx, 0) as usize
        } else {
            if let Some(c) = self.from_screen_col(cy, scx as usize) {
                c
            } else {
                self.line_width(cy).expect("valid_line")
            }
        };

        let c = self.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }
}

impl TextAreaState {
    /// Maximum offset that is accessible with scrolling.
    ///
    /// This is shorter than the length of the content by whatever fills the last page.
    /// This is the base for the scrollbar content_length.
    pub fn vertical_max_offset(&self) -> usize {
        self.value
            .len_lines()
            .saturating_sub(self.inner.height as usize)
    }

    /// Current vertical offset.
    pub fn vertical_offset(&self) -> usize {
        self.value.offset().1
    }

    /// Vertical page-size at the current offset.
    pub fn vertical_page(&self) -> usize {
        self.inner.height as usize
    }

    /// Suggested scroll per scroll-event.
    pub fn vertical_scroll(&self) -> usize {
        max(self.vertical_page() / 10, 1)
    }

    /// Maximum offset that is accessible with scrolling.
    ///
    /// This is currently set to usize::MAX.
    pub fn horizontal_max_offset(&self) -> usize {
        usize::MAX
    }

    /// Current horizontal offset.
    pub fn horizontal_offset(&self) -> usize {
        self.value.offset().0
    }

    /// Horizontal page-size at the current offset.
    pub fn horizontal_page(&self) -> usize {
        self.inner.width as usize
    }

    /// Suggested scroll per scroll-event.
    pub fn horizontal_scroll(&self) -> usize {
        max(self.horizontal_page() / 10, 1)
    }

    /// Change the vertical offset.
    ///
    /// Due to overscroll it's possible that this is an invalid offset for the widget.
    /// The widget must deal with this situation.
    ///
    /// The widget returns true if the offset changed at all.
    #[allow(unused_assignments)]
    pub fn set_vertical_offset(&mut self, row_offset: usize) -> bool {
        let (ox, mut oy) = self.value.offset();

        oy = min(row_offset, self.vertical_max_offset());

        self.value.set_offset((ox, oy))
    }

    /// Change the horizontal offset.
    ///
    /// Due to overscroll it's possible that this is an invalid offset for the widget.
    /// The widget must deal with this situation.
    ///
    /// The widget returns true if the offset changed at all.
    #[allow(unused_assignments)]
    pub fn set_horizontal_offset(&mut self, col_offset: usize) -> bool {
        let (mut ox, oy) = self.value.offset();

        ox = col_offset;

        self.value.set_offset((ox, oy))
    }

    /// Scroll up by n items.
    /// The widget returns true if the offset changed at all.
    pub fn scroll_up(&mut self, n: usize) -> bool {
        self.set_vertical_offset(self.vertical_offset().saturating_sub(n))
    }

    /// Scroll down by n items.
    /// The widget returns true if the offset changed at all.
    pub fn scroll_down(&mut self, n: usize) -> bool {
        self.set_vertical_offset(self.vertical_offset() + n)
    }

    /// Scroll up by n items.
    /// The widget returns true if the offset changed at all.
    pub fn scroll_left(&mut self, n: usize) -> bool {
        self.set_horizontal_offset(self.horizontal_offset().saturating_sub(n))
    }

    /// Scroll down by n items.
    /// The widget returns true if the offset changed at all.
    pub fn scroll_right(&mut self, n: usize) -> bool {
        self.set_horizontal_offset(self.horizontal_offset() + n)
    }

    /// Scroll that the cursor is visible.
    /// All move-fn do this automatically.
    pub fn scroll_cursor_to_visible(&mut self) -> bool {
        let old_offset = self.value.offset();

        let (cx, cy) = self.value.cursor();
        let (ox, oy) = self.value.offset();

        let noy = if cy < oy {
            cy
        } else if cy >= oy + self.inner.height as usize {
            cy.saturating_sub(self.inner.height as usize - 1)
        } else {
            oy
        };

        let nox = if cx < ox {
            cx
        } else if cx >= ox + self.inner.width as usize {
            cx.saturating_sub(self.inner.width as usize)
        } else {
            ox
        };

        self.value.set_offset((nox, noy));

        self.value.offset() != old_offset
    }
}

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

impl UsedEvent for TextOutcome {
    fn used_event(&self) -> bool {
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

impl HandleEvent<crossterm::event::Event, FocusKeys, TextOutcome> for TextAreaState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: FocusKeys) -> TextOutcome {
        let mut r = match event {
            ct_event!(key press c)
            | ct_event!(key press SHIFT-c)
            | ct_event!(key press CONTROL_ALT-c) => self.insert_char(*c).into(),
            ct_event!(keycode press Enter) => self.insert_newline().into(),
            ct_event!(keycode press Backspace) => self.delete_prev_char().into(),
            ct_event!(keycode press Delete) => self.delete_next_char().into(),
            ct_event!(keycode press CONTROL-Backspace) => self.delete_prev_word().into(),
            ct_event!(keycode press CONTROL-Delete) => self.delete_next_word().into(),

            ct_event!(key release _)
            | ct_event!(key release SHIFT-_)
            | ct_event!(key release CONTROL_ALT-_)
            | ct_event!(keycode release Enter)
            | ct_event!(keycode release Backspace)
            | ct_event!(keycode release Delete)
            | ct_event!(keycode release CONTROL-Backspace)
            | ct_event!(keycode release CONTROL-Delete) => TextOutcome::Unchanged,
            _ => TextOutcome::NotUsed,
        };

        if r == TextOutcome::NotUsed {
            r = self.handle(event, ReadOnly);
        }
        if r == TextOutcome::NotUsed {
            r = self.handle(event, MouseOnly);
        }
        r
    }
}

/// Runs only the navigation events, not any editing.
#[derive(Debug)]
pub struct ReadOnly;

impl HandleEvent<crossterm::event::Event, ReadOnly, TextOutcome> for TextAreaState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: ReadOnly) -> TextOutcome {
        let mut r = match event {
            ct_event!(keycode press Left) => self.move_left(1, false).into(),
            ct_event!(keycode press Right) => self.move_right(1, false).into(),
            ct_event!(keycode press Up) => self.move_up(1, false).into(),
            ct_event!(keycode press Down) => self.move_down(1, false).into(),
            ct_event!(keycode press PageUp) => self.move_up(self.vertical_page(), false).into(),
            ct_event!(keycode press PageDown) => self.move_down(self.vertical_page(), false).into(),
            ct_event!(keycode press Home) => self.move_to_line_start(false).into(),
            ct_event!(keycode press End) => self.move_to_line_end(false).into(),
            ct_event!(keycode press CONTROL-Left) => self.move_to_prev_word(false).into(),
            ct_event!(keycode press CONTROL-Right) => self.move_to_next_word(false).into(),
            ct_event!(keycode press CONTROL-Up) => false.into(),
            ct_event!(keycode press CONTROL-Down) => false.into(),
            ct_event!(keycode press CONTROL-PageUp) => self.move_to_screen_start(false).into(),
            ct_event!(keycode press CONTROL-PageDown) => self.move_to_screen_end(false).into(),
            ct_event!(keycode press CONTROL-Home) => self.move_to_start(false).into(),
            ct_event!(keycode press CONTROL-End) => self.move_to_end(false).into(),

            ct_event!(keycode press ALT-Left) => self.scroll_left(1).into(),
            ct_event!(keycode press ALT-Right) => self.scroll_right(1).into(),
            ct_event!(keycode press ALT-Up) => self.scroll_up(1).into(),
            ct_event!(keycode press ALT-Down) => self.scroll_down(1).into(),
            ct_event!(keycode press ALT-PageUp) => {
                self.scroll_up(max(self.vertical_page() / 2, 1)).into()
            }
            ct_event!(keycode press ALT-PageDown) => {
                self.scroll_down(max(self.vertical_page() / 2, 1)).into()
            }
            ct_event!(keycode press ALT_SHIFT-PageUp) => {
                self.scroll_left(max(self.horizontal_page() / 5, 1)).into()
            }
            ct_event!(keycode press ALT_SHIFT-PageDown) => {
                self.scroll_right(max(self.horizontal_page() / 5, 1)).into()
            }

            ct_event!(keycode press SHIFT-Left) => self.move_left(1, true).into(),
            ct_event!(keycode press SHIFT-Right) => self.move_right(1, true).into(),
            ct_event!(keycode press SHIFT-Up) => self.move_up(1, true).into(),
            ct_event!(keycode press SHIFT-Down) => self.move_down(1, true).into(),
            ct_event!(keycode press SHIFT-PageUp) => {
                self.move_up(self.vertical_page(), true).into()
            }
            ct_event!(keycode press SHIFT-PageDown) => {
                self.move_down(self.vertical_page(), true).into()
            }
            ct_event!(keycode press SHIFT-Home) => self.move_to_line_start(true).into(),
            ct_event!(keycode press SHIFT-End) => self.move_to_line_end(true).into(),
            ct_event!(keycode press CONTROL_SHIFT-Left) => self.move_to_prev_word(true).into(),
            ct_event!(keycode press CONTROL_SHIFT-Right) => self.move_to_next_word(true).into(),
            ct_event!(key press CONTROL-'a') => self.select_all().into(),

            ct_event!(keycode release Left)
            | ct_event!(keycode release Right)
            | ct_event!(keycode release Up)
            | ct_event!(keycode release Down)
            | ct_event!(keycode release PageUp)
            | ct_event!(keycode release PageDown)
            | ct_event!(keycode release Home)
            | ct_event!(keycode release End)
            | ct_event!(keycode release CONTROL-Left)
            | ct_event!(keycode release CONTROL-Right)
            | ct_event!(keycode release CONTROL-Up)
            | ct_event!(keycode release CONTROL-Down)
            | ct_event!(keycode release CONTROL-PageUp)
            | ct_event!(keycode release CONTROL-PageDown)
            | ct_event!(keycode release CONTROL-Home)
            | ct_event!(keycode release CONTROL-End)
            | ct_event!(keycode release ALT-Left)
            | ct_event!(keycode release ALT-Right)
            | ct_event!(keycode release ALT-Up)
            | ct_event!(keycode release ALT-Down)
            | ct_event!(keycode release ALT-PageUp)
            | ct_event!(keycode release ALT-PageDown)
            | ct_event!(keycode release ALT_SHIFT-PageUp)
            | ct_event!(keycode release ALT_SHIFT-PageDown)
            | ct_event!(keycode release SHIFT-Left)
            | ct_event!(keycode release SHIFT-Right)
            | ct_event!(keycode release SHIFT-Up)
            | ct_event!(keycode release SHIFT-Down)
            | ct_event!(keycode release SHIFT-PageUp)
            | ct_event!(keycode release SHIFT-PageDown)
            | ct_event!(keycode release SHIFT-Home)
            | ct_event!(keycode release SHIFT-End)
            | ct_event!(keycode release CONTROL_SHIFT-Left)
            | ct_event!(keycode release CONTROL_SHIFT-Right)
            | ct_event!(key release CONTROL-'a') => TextOutcome::Unchanged,
            _ => TextOutcome::NotUsed,
        };

        if r == TextOutcome::NotUsed {
            r = self.handle(event, MouseOnly);
        }
        r
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, TextOutcome> for TextAreaState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: MouseOnly) -> TextOutcome {
        let r = match event {
            ct_event!(scroll down for column,row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    self.scroll_down(self.vertical_scroll()).into()
                } else {
                    TextOutcome::NotUsed
                }
            }
            ct_event!(scroll up for column, row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    self.scroll_up(self.vertical_scroll()).into()
                } else {
                    TextOutcome::NotUsed
                }
            }
            ct_event!(scroll ALT down for column,row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    self.scroll_right(self.horizontal_scroll()).into()
                } else {
                    TextOutcome::NotUsed
                }
            }
            ct_event!(scroll ALT up for column, row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    self.scroll_left(self.horizontal_scroll()).into()
                } else {
                    TextOutcome::NotUsed
                }
            }
            ct_event!(mouse down Left for column,row) => {
                if self.inner.contains(Position::new(*column, *row)) {
                    self.mouse.set_drag();
                    let cx = (column - self.inner.x) as i16;
                    let cy = (row - self.inner.y) as i16;
                    self.set_screen_cursor((cx, cy), false).into()
                } else {
                    TextOutcome::NotUsed
                }
            }
            ct_event!(mouse drag Left for column, row) => {
                if self.mouse.do_drag() {
                    let cx = *column as i16 - self.inner.x as i16;
                    let cy = *row as i16 - self.inner.y as i16;
                    self.set_screen_cursor((cx, cy), true).into()
                } else {
                    TextOutcome::NotUsed
                }
            }
            ct_event!(mouse moved) => {
                self.mouse.clear_drag();
                TextOutcome::NotUsed
            }
            _ => TextOutcome::NotUsed,
        };

        r
    }
}

/// Handle all events.
/// Text events are only processed if focus is true.
/// Mouse events are processed if they are in range.
pub fn handle_events(
    state: &mut TextAreaState,
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
    state: &mut TextAreaState,
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
    state: &mut TextAreaState,
    event: &crossterm::event::Event,
) -> TextOutcome {
    state.handle(event, MouseOnly)
}

pub mod graphemes {
    use ropey::iter::Chunks;
    use ropey::RopeSlice;
    use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

    /// Length as grapheme count.
    pub fn rope_len(r: RopeSlice<'_>) -> usize {
        let it = RopeGraphemes::new(r);
        it.filter(|c| c != "\n").count()
    }

    /// An implementation of a graphemes iterator, for iterating over
    /// the graphemes of a RopeSlice.
    #[derive(Debug)]
    pub struct RopeGraphemes<'a> {
        text: RopeSlice<'a>,
        chunks: Chunks<'a>,
        cur_chunk: &'a str,
        cur_chunk_start: usize,
        cursor: GraphemeCursor,
    }

    impl<'a> RopeGraphemes<'a> {
        pub fn new(slice: RopeSlice<'a>) -> RopeGraphemes<'a> {
            let mut chunks = slice.chunks();
            let first_chunk = chunks.next().unwrap_or("");
            RopeGraphemes {
                text: slice,
                chunks,
                cur_chunk: first_chunk,
                cur_chunk_start: 0,
                cursor: GraphemeCursor::new(0, slice.len_bytes(), true),
            }
        }
    }

    impl<'a> Iterator for RopeGraphemes<'a> {
        type Item = RopeSlice<'a>;

        fn next(&mut self) -> Option<RopeSlice<'a>> {
            let a = self.cursor.cur_cursor();
            let b;
            loop {
                match self
                    .cursor
                    .next_boundary(self.cur_chunk, self.cur_chunk_start)
                {
                    Ok(None) => {
                        return None;
                    }
                    Ok(Some(n)) => {
                        b = n;
                        break;
                    }
                    Err(GraphemeIncomplete::NextChunk) => {
                        self.cur_chunk_start += self.cur_chunk.len();
                        self.cur_chunk = self.chunks.next().unwrap_or("");
                    }
                    Err(GraphemeIncomplete::PreContext(idx)) => {
                        let (chunk, byte_idx, _, _) =
                            self.text.chunk_at_byte(idx.saturating_sub(1));
                        self.cursor.provide_context(chunk, byte_idx);
                    }
                    _ => unreachable!(),
                }
            }

            if a < self.cur_chunk_start {
                let a_char = self.text.byte_to_char(a);
                let b_char = self.text.byte_to_char(b);

                Some(self.text.slice(a_char..b_char))
            } else {
                let a2 = a - self.cur_chunk_start;
                let b2 = b - self.cur_chunk_start;
                Some((&self.cur_chunk[a2..b2]).into())
            }
        }
    }

    /// An implementation of a graphemes iterator, for iterating over
    /// the graphemes of a RopeSlice.
    #[derive(Debug)]
    pub struct RopeGraphemesIdx<'a> {
        text: RopeSlice<'a>,
        chunks: Chunks<'a>,
        cur_chunk: &'a str,
        cur_chunk_start: usize,
        cursor: GraphemeCursor,
    }

    impl<'a> RopeGraphemesIdx<'a> {
        pub fn new(slice: RopeSlice<'a>) -> RopeGraphemesIdx<'a> {
            let mut chunks = slice.chunks();
            let first_chunk = chunks.next().unwrap_or("");
            RopeGraphemesIdx {
                text: slice,
                chunks,
                cur_chunk: first_chunk,
                cur_chunk_start: 0,
                cursor: GraphemeCursor::new(0, slice.len_bytes(), true),
            }
        }
    }

    impl<'a> Iterator for RopeGraphemesIdx<'a> {
        type Item = ((usize, usize), RopeSlice<'a>);

        fn next(&mut self) -> Option<((usize, usize), RopeSlice<'a>)> {
            let a = self.cursor.cur_cursor();
            let b;
            loop {
                match self
                    .cursor
                    .next_boundary(self.cur_chunk, self.cur_chunk_start)
                {
                    Ok(None) => {
                        return None;
                    }
                    Ok(Some(n)) => {
                        b = n;
                        break;
                    }
                    Err(GraphemeIncomplete::NextChunk) => {
                        self.cur_chunk_start += self.cur_chunk.len();
                        self.cur_chunk = self.chunks.next().unwrap_or("");
                    }
                    Err(GraphemeIncomplete::PreContext(idx)) => {
                        let (chunk, byte_idx, _, _) =
                            self.text.chunk_at_byte(idx.saturating_sub(1));
                        self.cursor.provide_context(chunk, byte_idx);
                    }
                    _ => unreachable!(),
                }
            }

            if a < self.cur_chunk_start {
                let a_char = self.text.byte_to_char(a);
                let b_char = self.text.byte_to_char(b);

                Some(((a, b), self.text.slice(a_char..b_char)))
            } else {
                let a2 = a - self.cur_chunk_start;
                let b2 = b - self.cur_chunk_start;
                Some(((a, b), (&self.cur_chunk[a2..b2]).into()))
            }
        }
    }
}

pub mod core {
    use crate::textarea::graphemes::{rope_len, RopeGraphemesIdx};
    use log::debug;
    use ropey::iter::Lines;
    use ropey::{Rope, RopeSlice};
    use std::cmp::{min, Ordering};
    use std::fmt::{Debug, Formatter};
    use std::iter::Skip;
    use std::mem;
    use std::slice::IterMut;

    pub use crate::textarea::graphemes::RopeGraphemes;

    /// Core for text editing.
    #[derive(Debug, Default, Clone)]
    pub struct InputCore {
        value: Rope,

        styles: StyleMap,

        /// Scroll offset
        offset: (usize, usize),

        /// Secondary column, remembered for moving up/down.
        move_col: Option<usize>,
        /// Cursor
        cursor: (usize, usize),
        /// Anchor for the selection.
        anchor: (usize, usize),
    }

    /// Range for text ranges.
    #[derive(Default, PartialEq, Eq, Clone, Copy)]
    pub struct TextRange {
        pub start: (usize, usize),
        pub end: (usize, usize),
    }

    #[derive(Debug, Default, Clone)]
    struct StyleMap {
        /// Vec of (range, style-idx)
        styles: Vec<(TextRange, usize)>,
    }

    #[derive(Debug)]
    pub struct ScrolledIter<'a> {
        lines: Lines<'a>,
        offset: usize,
    }

    impl Debug for TextRange {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "TextRange  {}|{}-{}|{}",
                self.start.0, self.start.1, self.end.0, self.end.1
            )
        }
    }

    impl TextRange {
        /// New text range.
        ///
        /// Panic
        /// Panics if start > end.
        pub fn new(start: (usize, usize), end: (usize, usize)) -> Self {
            // reverse the args, then it works.
            if (start.1, start.0) > (end.1, end.0) {
                panic!("start {:?} > end {:?}", start, end);
            }
            TextRange { start, end }
        }

        /// Start position
        #[inline]
        pub fn start(&self) -> (usize, usize) {
            self.start
        }

        /// End position
        #[inline]
        pub fn end(&self) -> (usize, usize) {
            self.end
        }

        /// Empty range
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.start == self.end
        }

        /// Range contains the given position.
        #[inline]
        pub fn contains(&self, pos: (usize, usize)) -> bool {
            self.ordering(pos) == Ordering::Equal
        }

        /// Range contains the other range.
        #[inline]
        pub fn contains_range(&self, range: TextRange) -> bool {
            self.ordering(range.start) == Ordering::Equal
                && self.ordering_inclusive(range.end) == Ordering::Equal
        }

        /// What place is the range respective to the given position.
        #[inline]
        pub fn ordering(&self, pos: (usize, usize)) -> Ordering {
            // reverse the args, then it works.
            let start = (self.start.1, self.start.0);
            let end = (self.end.1, self.end.0);

            let pos = (pos.1, pos.0);

            if pos < start {
                Ordering::Greater
            } else if pos < end {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        }

        /// What place is the range respective to the given position.
        /// This one includes the `range.end`.
        #[inline]
        pub fn ordering_inclusive(&self, pos: (usize, usize)) -> Ordering {
            // reverse the args, then it works.
            let start = (self.start.1, self.start.0);
            let end = (self.end.1, self.end.0);

            let pos = (pos.1, pos.0);

            if pos < start {
                Ordering::Greater
            } else if pos <= end {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        }

        /// Modify all positions in place.
        #[inline]
        pub fn expand_all(&self, it: Skip<IterMut<'_, (TextRange, usize)>>) {
            for (r, _s) in it {
                self._expand(&mut r.start);
                self._expand(&mut r.end);
            }
        }

        /// Return the modified position, as if this range expanded from its
        /// start to its full expansion.
        #[inline]
        pub fn expand(&self, pos: (usize, usize)) -> (usize, usize) {
            let mut tmp = pos;
            self._expand(&mut tmp);
            tmp
        }

        #[inline(always)]
        fn _expand(&self, pos: &mut (usize, usize)) {
            let delta_lines = self.end.1 - self.start.1;
            if *pos < self.start {
                // noop
            } else {
                if pos.1 > self.start.1 {
                    pos.1 += delta_lines;
                } else if pos.1 == self.start.1 {
                    if pos.0 >= self.start.0 {
                        pos.0 = pos.0 - self.start.0 + self.end.0;
                        pos.1 += delta_lines;
                    }
                }
            }
        }

        /// Modify all positions in place.
        #[inline]
        pub fn shrink_all(&self, it: Skip<IterMut<'_, (TextRange, usize)>>) {
            for (r, _s) in it {
                self._shrink(&mut r.start);
                self._shrink(&mut r.end);
            }
        }

        /// Return the modified position, if this range would shrink to nothing.
        #[inline]
        pub fn shrink(&self, pos: (usize, usize)) -> (usize, usize) {
            let mut tmp = pos;
            self._shrink(&mut tmp);
            tmp
        }

        /// Return the modified position, if this range would shrink to nothing.
        #[inline(always)]
        fn _shrink(&self, pos: &mut (usize, usize)) {
            let delta_lines = self.end.1 - self.start.1;
            match self.ordering_inclusive(*pos) {
                Ordering::Greater => {
                    // noop
                }
                Ordering::Equal => {
                    *pos = self.start;
                }
                Ordering::Less => {
                    if pos.1 > self.end.1 {
                        pos.1 -= delta_lines;
                    } else if pos.1 == self.end.1 {
                        if pos.0 >= self.end.0 {
                            pos.0 = pos.0 - self.end.0 + self.start.0;
                            pos.1 -= delta_lines;
                        }
                    }
                }
            }
        }
    }

    // This needs its own impl, because the order is exactly wrong.
    // For any sane range I'd need (row,col) but what I got is (col,row).
    // Need this to conform with the rest of ratatui ...
    impl PartialOrd for TextRange {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            // reverse the args, then it works.
            let start = (self.start.1, self.start.0);
            let end = (self.end.1, self.end.0);
            let ostart = (other.start.1, other.start.0);
            let oend = (other.end.1, other.end.0);

            if start < ostart {
                Some(Ordering::Less)
            } else if start > ostart {
                Some(Ordering::Greater)
            } else {
                if end < oend {
                    Some(Ordering::Less)
                } else if end > oend {
                    Some(Ordering::Greater)
                } else {
                    Some(Ordering::Equal)
                }
            }
        }
    }

    impl Ord for TextRange {
        fn cmp(&self, other: &Self) -> Ordering {
            self.partial_cmp(other).expect("order")
        }
    }

    impl StyleMap {
        /// New mapping
        pub(crate) fn new() -> Self {
            Self::default()
        }

        /// Remove all styles.
        pub(crate) fn clear_styles(&mut self) {
            self.styles.clear();
        }

        /// Add a text-style for a range.
        ///
        /// The same range can be added again with a different style.
        /// Overlapping regions get the merged style.
        pub(crate) fn add_style(&mut self, range: TextRange, style: usize) {
            let stylemap = (range, style);
            match self.styles.binary_search(&stylemap) {
                Ok(_) => {
                    // noop
                }
                Err(idx) => {
                    self.styles.insert(idx, stylemap);
                }
            }
        }

        /// Find all styles
        pub(crate) fn styles_after_mut(
            &mut self,
            pos: (usize, usize),
        ) -> Skip<IterMut<'_, (TextRange, usize)>> {
            let first = match self.styles.binary_search_by(|v| v.0.ordering(pos)) {
                Ok(mut i) => {
                    // binary-search found *some* matching style, we need all of them.
                    // this finds the first one.
                    loop {
                        if i == 0 {
                            break;
                        }
                        if !self.styles[i - 1].0.contains(pos) {
                            break;
                        }
                        i -= 1;
                    }
                    i
                }
                Err(i) => i,
            };

            self.styles.iter_mut().skip(first)
        }

        /// Find all styles for the given position.
        ///
        pub(crate) fn styles_at(&self, pos: (usize, usize), result: &mut Vec<usize>) {
            match self.styles.binary_search_by(|v| v.0.ordering(pos)) {
                Ok(mut i) => {
                    // binary-search found *some* matching style, we need all of them.
                    // this finds the first one.
                    loop {
                        if i == 0 {
                            break;
                        }
                        if !self.styles[i - 1].0.contains(pos) {
                            break;
                        }
                        i -= 1;
                    }

                    // collect all matching styles.
                    result.clear();
                    for i in i..self.styles.len() {
                        if self.styles[i].0.contains(pos) {
                            result.push(self.styles[i].1);
                        } else {
                            break;
                        }
                    }
                }
                Err(_) => result.clear(),
            }
        }
    }

    impl<'a> Iterator for ScrolledIter<'a> {
        type Item = Skip<RopeGraphemes<'a>>;

        fn next(&mut self) -> Option<Self::Item> {
            let Some(s) = self.lines.next() else {
                return None;
            };

            Some(RopeGraphemes::new(s).skip(self.offset))
        }
    }

    impl InputCore {
        pub fn new() -> Self {
            Self::default()
        }

        /// Set the text offset as (col,row).
        pub fn set_offset(&mut self, mut offset: (usize, usize)) -> bool {
            let old_offset = self.offset;

            let (ox, oy) = offset;
            let oy = min(oy, self.len_lines() - 1);
            offset = (ox, oy);

            self.offset = offset;

            self.offset != old_offset
        }

        /// Text offset as (col,row)
        #[inline]
        pub fn offset(&self) -> (usize, usize) {
            self.offset
        }

        /// Extra column information for cursor movement.
        /// The cursor position is capped to the current line length, so if you
        /// move up one row, you might end at a position left of the current column.
        /// If you move up once more you want to return to the original position.
        /// That's what is stored here.
        #[inline]
        pub fn set_move_col(&mut self, col: Option<usize>) {
            self.move_col = col;
        }

        /// Extra column information for cursor movement.
        #[inline]
        pub fn move_col(&mut self) -> Option<usize> {
            self.move_col
        }

        /// Set the cursor position.
        /// The value is capped to the number of text lines and the line-width.
        /// Returns true, if the cursor actually changed.
        pub fn set_cursor(&mut self, mut cursor: (usize, usize), extend_selection: bool) -> bool {
            let old_cursor = self.cursor;
            let old_anchor = self.anchor;

            let (mut cx, mut cy) = cursor;
            cy = min(cy, self.len_lines() - 1);
            cx = min(cx, self.line_width(cy).expect("valid_line"));

            cursor = (cx, cy);

            self.cursor = cursor;

            if !extend_selection {
                self.anchor = cursor;
            }

            old_cursor != self.cursor || old_anchor != self.anchor
        }

        /// Cursor position.
        #[inline]
        pub fn cursor(&self) -> (usize, usize) {
            self.cursor
        }

        /// Selection anchor.
        #[inline]
        pub fn anchor(&self) -> (usize, usize) {
            self.anchor
        }

        /// Set the text.
        /// Resets the selection and any styles.
        pub fn set_value<S: AsRef<str>>(&mut self, s: S) {
            self.value = Rope::from_str(s.as_ref());
            self.offset = (0, 0);
            self.cursor = (0, 0);
            self.anchor = (0, 0);
            self.move_col = None;
            self.styles.clear_styles();
        }

        /// Set the text value as a Rope.
        /// Resets all internal state.
        #[inline]
        pub fn set_rope(&mut self, s: Rope) {
            self.value = s;
            self.offset = (0, 0);
            self.cursor = (0, 0);
            self.anchor = (0, 0);
            self.move_col = None;
            self.styles.clear_styles();
        }

        /// Text value.
        #[inline]
        pub fn value(&self) -> String {
            String::from(&self.value)
        }

        /// A range of the text as RopeSlice.
        pub fn value_range(&self, range: TextRange) -> Option<RopeSlice<'_>> {
            let Some(s) = self.char_at(range.start) else {
                return None;
            };
            let Some(e) = self.char_at(range.end) else {
                return None;
            };

            Some(self.value.slice(s..e))
        }

        /// Value as Bytes iterator.
        pub fn value_as_bytes(&self) -> ropey::iter::Bytes<'_> {
            self.value.bytes()
        }

        /// Value as Chars iterator.
        pub fn value_as_chars(&self) -> ropey::iter::Chars<'_> {
            self.value.chars()
        }

        /// Clear styles.
        #[inline]
        pub fn clear_styles(&mut self) {
            self.styles.clear_styles();
        }

        /// Add a style for the given range.
        ///
        /// What is given here is the index into the Vec with the actual Styles.
        /// Those are set at the widget.
        #[inline]
        pub fn add_style(&mut self, range: TextRange, style: usize) {
            self.styles.add_style(range, style);
        }

        /// Style map.
        #[inline]
        pub fn styles(&self) -> &[(TextRange, usize)] {
            &self.styles.styles
        }

        /// Finds all styles for the given position.
        ///
        /// Returns the indexes into the style vec.
        #[inline]
        pub fn styles_at(&self, pos: (usize, usize), result: &mut Vec<usize>) {
            self.styles.styles_at(pos, result)
        }

        /// Returns a line as an iterator over the graphemes for the line.
        /// This contains the \n at the end.
        pub fn line(&self, n: usize) -> Option<RopeGraphemes<'_>> {
            let Some(mut lines) = self.value.get_lines_at(n) else {
                return None;
            };
            let line = lines.next();
            if let Some(line) = line {
                Some(RopeGraphemes::new(line))
            } else {
                Some(RopeGraphemes::new(RopeSlice::from("")))
            }
        }

        /// Returns a line as an iterator over the graphemes for the line.
        /// This contains the \n at the end.
        pub fn line_idx(&self, n: usize) -> Option<RopeGraphemesIdx<'_>> {
            let Some(mut lines) = self.value.get_lines_at(n) else {
                return None;
            };
            let line = lines.next();
            if let Some(line) = line {
                Some(RopeGraphemesIdx::new(line))
            } else {
                Some(RopeGraphemesIdx::new(RopeSlice::from("")))
            }
        }

        /// Line width as grapheme count.
        pub fn line_width(&self, n: usize) -> Option<usize> {
            let Some(mut lines) = self.value.get_lines_at(n) else {
                return None;
            };
            let line = lines.next();
            if let Some(line) = line {
                Some(rope_len(line))
            } else {
                Some(0)
            }
        }

        /// Number of lines.
        #[inline]
        pub fn len_lines(&self) -> usize {
            self.value.len_lines()
        }

        /// Reset.
        #[inline]
        pub fn clear(&mut self) {
            self.set_value("");
        }

        /// Empty.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.value.len_bytes() == 0
        }

        /// Any text selection.
        #[inline]
        pub fn has_selection(&self) -> bool {
            self.anchor != self.cursor
        }

        #[inline]
        pub fn set_selection(&mut self, range: TextRange) -> bool {
            let old_selection = self.selection();

            self.set_cursor(range.start, false);
            self.set_cursor(range.end, true);

            old_selection != self.selection()
        }

        #[inline]
        pub fn select_all(&mut self) -> bool {
            let old_selection = self.selection();

            self.set_cursor((0, 0), false);
            let last = self.len_lines() - 1;
            let last_width = self.line_width(last).expect("valid_last_line");
            self.set_cursor((last_width, last), true);

            old_selection != self.selection()
        }

        /// Returns the selection as TextRange.
        pub fn selection(&self) -> TextRange {
            let selection = if self.cursor.1 < self.anchor.1 {
                TextRange {
                    start: self.cursor,
                    end: self.anchor,
                }
            } else if self.cursor.1 > self.anchor.1 {
                TextRange {
                    start: self.anchor,
                    end: self.cursor,
                }
            } else {
                if self.cursor.0 < self.anchor.0 {
                    TextRange {
                        start: self.cursor,
                        end: self.anchor,
                    }
                } else {
                    TextRange {
                        start: self.anchor,
                        end: self.cursor,
                    }
                }
            };

            selection
        }

        /// Iterate over the text, shifted by the offset.
        #[inline]
        pub fn iter_scrolled(&self) -> ScrolledIter<'_> {
            let Some(l) = self.value.get_lines_at(self.offset.1) else {
                panic!("invalid offset {:?} value {:?}", self.offset, self.value);
            };
            ScrolledIter {
                lines: l,
                offset: self.offset.0,
            }
        }

        /// Find next word.
        pub fn next_word_boundary(&self, pos: (usize, usize)) -> Option<(usize, usize)> {
            let Some(mut char_pos) = self.char_at(pos) else {
                return None;
            };

            let chars_after = self.value.slice(char_pos..);
            let mut it = chars_after.chars_at(0);
            let mut init = true;
            loop {
                let Some(c) = it.next() else {
                    break;
                };

                if init {
                    if !c.is_whitespace() {
                        init = false;
                    }
                } else {
                    if c.is_whitespace() {
                        break;
                    }
                }

                char_pos += 1;
            }

            self.char_pos(char_pos)
        }

        /// Find next word.
        pub fn prev_word_boundary(&self, pos: (usize, usize)) -> Option<(usize, usize)> {
            let Some(mut char_pos) = self.char_at(pos) else {
                return None;
            };

            let chars_before = self.value.slice(..char_pos);
            let mut it = chars_before.chars_at(chars_before.len_chars());
            let mut init = true;
            loop {
                let Some(c) = it.prev() else {
                    break;
                };

                if init {
                    if !c.is_whitespace() {
                        init = false;
                    }
                } else {
                    if c.is_whitespace() {
                        break;
                    }
                }

                char_pos -= 1;
            }

            self.char_pos(char_pos)
        }

        /// Len in chars
        pub fn len_chars(&self) -> usize {
            self.value.len_chars()
        }

        /// Len in bytes
        pub fn len_bytes(&self) -> usize {
            self.value.len_bytes()
        }

        /// Char position to grapheme position.
        pub fn char_pos(&self, char_pos: usize) -> Option<(usize, usize)> {
            let Ok(byte_pos) = self.value.try_char_to_byte(char_pos) else {
                return None;
            };
            self.byte_pos(byte_pos)
        }

        /// Byte position to grapheme position.
        pub fn byte_pos(&self, byte: usize) -> Option<(usize, usize)> {
            let Ok(y) = self.value.try_byte_to_line(byte) else {
                return None;
            };
            let mut x = 0;
            let byte_y = self.value.try_line_to_byte(y).expect("valid_y");

            let mut it_line = self.line_idx(y).expect("valid_y");
            loop {
                let Some(((sb, _eb), _cc)) = it_line.next() else {
                    break;
                };
                if byte_y + sb >= byte {
                    break;
                }
                x += 1;
            }

            Some((x, y))
        }

        /// Grapheme position to byte position.
        /// This is the (start,end) position of the single grapheme after pos.
        pub fn byte_at(&self, pos: (usize, usize)) -> Option<(usize, usize)> {
            let Ok(line_byte) = self.value.try_line_to_byte(pos.1) else {
                return None;
            };

            let len_bytes = self.value.len_bytes();
            let mut it_line = self.line_idx(pos.1).expect("valid_line");
            let mut x = -1isize;
            let mut last_eb = 0;
            loop {
                let (sb, eb, last) = if let Some((v, _)) = it_line.next() {
                    x += 1;
                    last_eb = v.1;
                    (v.0, v.1, false)
                } else {
                    (last_eb, last_eb, true)
                };

                if pos.0 == x as usize {
                    return Some((line_byte + sb, line_byte + eb));
                }
                // one past the end is ok.
                if pos.0 == (x + 1) as usize && line_byte + eb == len_bytes {
                    return Some((line_byte + eb, line_byte + eb));
                }
                if last {
                    return None;
                }
            }
        }

        /// Grapheme position to char position.
        pub fn char_at(&self, pos: (usize, usize)) -> Option<usize> {
            let Some((byte_pos, _)) = self.byte_at(pos) else {
                return None;
            };
            Some(
                self.value
                    .try_byte_to_char(byte_pos)
                    .expect("valid_byte_pos"),
            )
        }

        /// Insert a character.
        pub fn insert_char(&mut self, pos: (usize, usize), c: char) {
            if c == '\n' {
                self.insert_newline(pos);
                return;
            }

            let Some(char_pos) = self.char_at(pos) else {
                return;
            };

            // no way to know if the new char combines with a surrounding char.
            // the difference of the graphem len seems safe though.
            let old_len = self.line_width(pos.1).expect("valid_pos");
            self.value.insert_char(char_pos, c);
            let new_len = self.line_width(pos.1).expect("valid_pos");

            let insert = TextRange::new((pos.0, pos.1), (pos.0 + new_len - old_len, pos.1));

            insert.expand_all(self.styles.styles_after_mut(pos));
            self.anchor = insert.expand(self.anchor);
            self.cursor = insert.expand(self.cursor);
        }

        /// Insert a line break.
        pub fn insert_newline(&mut self, pos: (usize, usize)) {
            let Some(char_pos) = self.char_at(pos) else {
                panic!("invalid pos {:?} value {:?}", pos, self.value);
            };

            self.value.insert_char(char_pos, '\n');

            let insert = TextRange::new((pos.0, pos.1), (0, pos.1 + 1));

            insert.expand_all(self.styles.styles_after_mut(pos));
            self.anchor = insert.expand(self.anchor);
            self.cursor = insert.expand(self.cursor);
        }

        pub fn remove(&mut self, range: TextRange) {
            let Some(start_pos) = self.char_at(range.start) else {
                panic!("invalid range {:?} value {:?}", range, self.value);
            };
            let Some(end_pos) = self.char_at(range.end) else {
                panic!("invalid range {:?} value {:?}", range, self.value);
            };

            self.value.remove(start_pos..end_pos);

            // remove deleted styles
            let styles = mem::take(&mut self.styles.styles);
            self.styles.styles = styles
                .into_iter()
                .filter(|(r, _)| !range.contains_range(*r))
                .collect();

            for (r, _) in self.styles.styles_after_mut(range.start) {
                r.start = range.shrink(r.start);
                r.end = range.shrink(r.end);
            }
            self.anchor = range.shrink(self.anchor);
            self.cursor = range.shrink(self.anchor);
        }
    }
}
