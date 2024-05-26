#![allow(dead_code)]

use crate::_private::NonExhaustive;
use crate::textarea::core::{RopeGraphemes, TextRange};
use crate::util::MouseFlags;
use crossterm::event::Event;
use log::debug;
use rat_event::util::Outcome;
use rat_event::{ct_event, FocusKeys, HandleEvent, MouseOnly};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::{BlockExt, Stylize};
use ratatui::style::Style;
use ratatui::widgets::{Block, StatefulWidget};
use std::cmp::{max, min};
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Default, Clone)]
pub struct TextArea<'a> {
    block: Option<Block<'a>>,
    style: Style,
    focus_style: Option<Style>,
    select_style: Option<Style>,
    text_style: Vec<Style>,
    focused: bool,
}

#[derive(Debug, Clone)]
pub struct TextAreaState {
    pub area: Rect,
    pub inner: Rect,
    pub mouse: MouseFlags,

    pub value: core::InputCore,

    pub non_exhaustive: NonExhaustive,
}

impl<'a> TextArea<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn select_style(mut self, style: Style) -> Self {
        self.select_style = Some(style);
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = Some(style);
        self
    }

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
                loop {
                    if col >= area.width {
                        break;
                    }

                    let cell = buf.get_mut(area.x + col, area.y + row);

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
                    cell.set_symbol(ch);

                    // text based
                    let (ox, oy) = state.offset();
                    let tx = col as usize + ox;
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
                    cell.set_style(style);

                    col += ch.width() as u16;
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
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn set_offset(&mut self, offset: (usize, usize)) -> bool {
        self.value.set_offset(offset)
    }

    #[inline]
    pub fn offset(&self) -> (usize, usize) {
        self.value.offset()
    }

    #[inline]
    pub fn set_cursor(&mut self, cursor: (usize, usize), extend_selection: bool) -> bool {
        self.value.set_cursor(cursor, extend_selection)
    }

    #[inline]
    pub fn cursor(&self) -> (usize, usize) {
        self.value.cursor()
    }

    #[inline]
    pub fn anchor(&self) -> (usize, usize) {
        self.value.anchor()
    }

    #[inline]
    pub fn set_value<S: AsRef<str>>(&mut self, s: S) {
        self.value.set_value(s);
    }

    #[inline]
    pub fn value(&self) -> String {
        self.value.value()
    }

    #[inline]
    pub fn line(&self, n: usize) -> RopeGraphemes<'_> {
        self.value.line(n)
    }

    #[inline]
    pub fn line_width(&self, n: usize) -> usize {
        self.value.line_width(n)
    }

    #[inline]
    pub fn len_lines(&self) -> usize {
        self.value.len_lines()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.value.clear();
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    #[inline]
    pub fn has_selection(&self) -> bool {
        self.value.has_selection()
    }

    #[inline]
    pub fn selection(&self) -> TextRange {
        self.value.selection()
    }

    #[inline]
    pub fn set_selection(&mut self, range: TextRange) -> bool {
        self.value.set_selection(range)
    }

    #[inline]
    pub fn select_all(&mut self) -> bool {
        self.value.select_all()
    }

    #[inline]
    pub fn clear_styles(&mut self) {
        self.value.clear_styles();
    }

    #[inline]
    pub fn add_style(&mut self, range: TextRange, style: usize) {
        self.value.add_style(range, style);
    }

    #[inline]
    pub fn styles_at(&self, pos: (usize, usize), result: &mut Vec<usize>) {
        self.value.styles_at(pos, result)
    }

    pub fn insert_char(&mut self, c: char) -> bool {
        self.value.insert_char(c);
        true
    }

    pub fn move_left(&mut self, n: usize, extend_selection: bool) -> bool {
        let (mut cx, mut cy) = self.value.cursor();

        if cx == 0 {
            if cy > 0 {
                cy = cy.saturating_sub(1);
                cx = self.value.line_width(cy);
            }
        } else {
            cx = cx.saturating_sub(n);
        }

        self.value.set_move_col(Some(cx));
        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    pub fn move_right(&mut self, n: usize, extend_selection: bool) -> bool {
        let (mut cx, mut cy) = self.value.cursor();

        let width = self.value.line_width(cy);
        if cx == width {
            if cy + 1 < self.value.len_lines() {
                cy += 1;
                cx = 0;
            }
        } else {
            cx = min(cx + n, width)
        }

        self.value.set_move_col(Some(cx));
        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    pub fn move_up(&mut self, n: usize, extend_selection: bool) -> bool {
        let (mut cx, mut cy) = self.value.cursor();

        cy = cy.saturating_sub(n);
        if let Some(xx) = self.value.move_col() {
            cx = min(xx, self.value.line_width(cy));
        } else {
            cx = min(cx, self.value.line_width(cy));
        }

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    pub fn move_down(&mut self, n: usize, extend_selection: bool) -> bool {
        let (mut cx, mut cy) = self.value.cursor();

        cy = min(cy + n, self.value.len_lines() - 1);
        if let Some(xx) = self.value.move_col() {
            cx = min(xx, self.value.line_width(cy));
        } else {
            cx = min(cx, self.value.line_width(cy));
        }

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    pub fn move_to_line_start(&mut self, extend_selection: bool) -> bool {
        let (mut cx, cy) = self.value.cursor();

        cx = 'f: {
            if cx > 0 {
                let l = self.value.line(cy);
                for (c, ch) in l.enumerate() {
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

    pub fn move_to_line_end(&mut self, extend_selection: bool) -> bool {
        let (_, cy) = self.value.cursor();

        let cx = self.value.line_width(cy);

        self.value.set_move_col(Some(cx));
        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    pub fn move_to_start(&mut self, extend_selection: bool) -> bool {
        let cx = 0;
        let cy = 0;

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    pub fn move_to_end(&mut self, extend_selection: bool) -> bool {
        let len = self.value.len_lines();

        let cx = 0;
        let cy = len - 1;

        let c = self.value.set_cursor((cx, cy), extend_selection);
        let s = self.scroll_cursor_to_visible();
        c || s
    }

    pub fn move_to_screen_start(&mut self, extend_selection: bool) -> bool {
        let (ox, oy) = self.value.offset();

        let cx = ox;
        let cy = oy;

        self.value.set_cursor((cx, cy), extend_selection)
    }

    pub fn move_to_screen_end(&mut self, extend_selection: bool) -> bool {
        let (ox, oy) = self.value.offset();
        let len = self.value.len_lines();

        let cx = ox;
        let cy = min(oy + self.vertical_page() - 1, len - 1);

        self.value.set_cursor((cx, cy), extend_selection)
    }

    /// Converts from a widget relative screen coordinate to a grapheme index.
    /// Row is a row-index into the value, not a screen-row.
    /// x is the relative screen position.
    pub fn from_screen_col(&self, row: usize, x: usize) -> usize {
        let (mut cx, cy) = (0usize, row);
        let (ox, _oy) = self.value.offset();

        let mut test = 0;
        for c in self.line(cy).skip(ox) {
            if test >= x {
                break;
            }

            test += if let Some(c) = c.as_str() {
                c.width()
            } else {
                c.to_string().width()
            };

            cx += 1;
        }

        cx + ox
    }

    /// Converts a grapheme based position to a screen position relative to the
    /// widget area.
    pub fn to_screen_col(&self, pos: (usize, usize)) -> u16 {
        let (px, py) = pos;
        let (ox, _oy) = self.value.offset();

        let mut sx = 0;
        for c in self.line(py).skip(ox).take(px - ox) {
            sx += if let Some(c) = c.as_str() {
                c.width()
            } else {
                c.to_string().width()
            };
        }

        sx as u16
    }

    /// Cursor position on the screen.
    pub fn screen_cursor(&self) -> Option<Position> {
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
                let mut sx = self.to_screen_col((cx, cy));

                Some(Position::new(self.inner.x + sx, self.inner.y + sy as u16))
            }
        }
    }

    /// Set the cursor position from screen coordinates.
    pub fn set_screen_cursor(&mut self, cursor: (isize, isize), extend_selection: bool) -> bool {
        let (scx, scy) = cursor;
        let (ox, oy) = self.value.offset();

        let cy = max(oy as isize + scy, 0) as usize;
        let cx = if scx < 0 {
            max(ox as isize + scx, 0) as usize
        } else {
            self.from_screen_col(cy, scx as usize)
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
    /// This is shorter than the length of the content by whatever fills the last page.
    /// This is the base for the scrollbar content_length.
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

impl HandleEvent<crossterm::event::Event, FocusKeys, Outcome> for TextAreaState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: FocusKeys) -> Outcome {
        let r = 'f: {
            let change = match event {
                ct_event!(keycode press Left) => self.move_left(1, false),
                ct_event!(keycode press Right) => self.move_right(1, false),
                ct_event!(keycode press Up) => self.move_up(1, false),
                ct_event!(keycode press Down) => self.move_down(1, false),
                ct_event!(keycode press PageUp) => self.move_up(self.vertical_page(), false),
                ct_event!(keycode press PageDown) => self.move_down(self.vertical_page(), false),
                ct_event!(keycode press Home) => self.move_to_line_start(false),
                ct_event!(keycode press End) => self.move_to_line_end(false),

                ct_event!(keycode press CONTROL-Left) => {
                    // let pos = self.prev_word_boundary();
                    // self.set_cursor(pos, false);
                    false
                }
                ct_event!(keycode press CONTROL-Right) => {
                    // let pos = self.next_word_boundary();
                    // self.set_cursor(pos, false);
                    false
                }
                ct_event!(keycode press CONTROL-Up) => false,
                ct_event!(keycode press CONTROL-Down) => false,
                ct_event!(keycode press CONTROL-PageUp) => self.move_to_screen_start(false),
                ct_event!(keycode press CONTROL-PageDown) => self.move_to_screen_end(false),
                ct_event!(keycode press CONTROL-Home) => self.move_to_start(false),
                ct_event!(keycode press CONTROL-End) => self.move_to_end(false),

                ct_event!(keycode press ALT-Left) => self.scroll_left(1),
                ct_event!(keycode press ALT-Right) => self.scroll_right(1),
                ct_event!(keycode press ALT-Up) => self.scroll_up(1),
                ct_event!(keycode press ALT-Down) => self.scroll_down(1),
                ct_event!(keycode press ALT-PageUp) => {
                    self.scroll_up(max(self.vertical_page() / 2, 1))
                }
                ct_event!(keycode press ALT-PageDown) => {
                    self.scroll_down(max(self.vertical_page() / 2, 1))
                }
                ct_event!(keycode press ALT_SHIFT-PageUp) => {
                    self.scroll_left(max(self.horizontal_page() / 5, 1))
                }
                ct_event!(keycode press ALT_SHIFT-PageDown) => {
                    self.scroll_right(max(self.horizontal_page() / 5, 1))
                }

                ct_event!(keycode press SHIFT-Left) => self.move_left(1, true),
                ct_event!(keycode press SHIFT-Right) => self.move_right(1, true),
                ct_event!(keycode press SHIFT-Up) => self.move_up(1, true),
                ct_event!(keycode press SHIFT-Down) => self.move_down(1, true),
                ct_event!(keycode press SHIFT-PageUp) => self.move_up(self.vertical_page(), true),
                ct_event!(keycode press SHIFT-PageDown) => {
                    self.move_down(self.vertical_page(), true)
                }
                ct_event!(keycode press SHIFT-Home) => self.move_to_line_start(true),
                ct_event!(keycode press SHIFT-End) => self.move_to_line_end(true),
                // ct_event!(keycode press CONTROL_SHIFT-Left) => {
                //     let pos = self.prev_word_boundary();
                //     self.set_cursor(pos, true);
                // }
                // ct_event!(keycode press CONTROL_SHIFT-Right) => {
                //     let pos = self.next_word_boundary();
                //     self.set_cursor(pos, true);
                // }
                ct_event!(key press CONTROL-'a') => self.select_all(),
                // ct_event!(keycode press Backspace) => self.delete_prev_char(),
                // ct_event!(keycode press Delete) => self.delete_next_char(),
                // ct_event!(keycode press CONTROL-Backspace) => {
                //     let prev = self.prev_word_boundary();
                //     self.remove(prev..self.cursor());
                // }
                // ct_event!(keycode press CONTROL-Delete) => {
                //     let next = self.next_word_boundary();
                //     self.remove(self.cursor()..next);
                // }
                // ct_event!(key press CONTROL-'d') => self.set_value(""),
                // ct_event!(keycode press CONTROL_SHIFT-Backspace) => self.remove(0..self.cursor()),
                // ct_event!(keycode press CONTROL_SHIFT-Delete) => {
                //     self.remove(self.cursor()..self.len())
                // }
                ct_event!(key press c)
                | ct_event!(key press SHIFT-c)
                | ct_event!(key press CONTROL_ALT-c) => self.insert_char(*c),
                ct_event!(keycode press Enter) => self.insert_char('\n'),
                Event::Key(k) => {
                    debug!("key {:?}", k);
                    break 'f Outcome::NotUsed;
                }
                _ => break 'f Outcome::NotUsed,
            };

            if change {
                Outcome::Changed
            } else {
                Outcome::Unchanged
            }
        };

        match r {
            Outcome::NotUsed => HandleEvent::handle(self, event, MouseOnly),
            v => v,
        }
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for TextAreaState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: MouseOnly) -> Outcome {
        match event {
            ct_event!(scroll down for column,row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    if self.scroll_down(self.vertical_scroll()) {
                        Outcome::Changed
                    } else {
                        Outcome::Unchanged
                    }
                } else {
                    Outcome::NotUsed
                }
            }
            ct_event!(scroll up for column, row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    if self.scroll_up(self.vertical_scroll()) {
                        Outcome::Changed
                    } else {
                        Outcome::Unchanged
                    }
                } else {
                    Outcome::NotUsed
                }
            }
            ct_event!(scroll ALT down for column,row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    if self.scroll_right(self.horizontal_scroll()) {
                        Outcome::Changed
                    } else {
                        Outcome::Unchanged
                    }
                } else {
                    Outcome::NotUsed
                }
            }
            ct_event!(scroll ALT up for column, row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    if self.scroll_left(self.horizontal_scroll()) {
                        Outcome::Changed
                    } else {
                        Outcome::Unchanged
                    }
                } else {
                    Outcome::NotUsed
                }
            }
            ct_event!(mouse down Left for column,row) => {
                if self.inner.contains(Position::new(*column, *row)) {
                    self.mouse.set_drag();
                    let cx = column - self.inner.x;
                    let cy = row - self.inner.y;
                    if self.set_screen_cursor((cx as isize, cy as isize), false) {
                        Outcome::Changed
                    } else {
                        Outcome::Unchanged
                    }
                } else {
                    Outcome::NotUsed
                }
            }
            ct_event!(mouse drag Left for column, row) => {
                if self.mouse.do_drag() {
                    let cx = *column as isize - self.inner.x as isize;
                    let cy = *row as isize - self.inner.y as isize;
                    if self.set_screen_cursor((cx, cy), true) {
                        Outcome::Changed
                    } else {
                        Outcome::Unchanged
                    }
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

mod graphemes {
    use ropey::iter::Chunks;
    use ropey::RopeSlice;
    use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

    /// Length as grapheme count.
    pub(crate) fn rope_len(r: RopeSlice<'_>) -> usize {
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
        type Item = (usize, RopeSlice<'a>);

        fn next(&mut self) -> Option<(usize, RopeSlice<'a>)> {
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

                Some((a, self.text.slice(a_char..b_char)))
            } else {
                let a2 = a - self.cur_chunk_start;
                let b2 = b - self.cur_chunk_start;
                Some((a, (&self.cur_chunk[a2..b2]).into()))
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
            assert!(start <= end);
            TextRange { start, end }
        }

        /// Start position
        pub fn start(&self) -> (usize, usize) {
            self.start
        }

        /// End position
        pub fn end(&self) -> (usize, usize) {
            self.end
        }

        /// Range contains the given position.
        pub fn contains(&self, pos: (usize, usize)) -> bool {
            self.ordering(pos) == Ordering::Equal
        }

        /// The given position is before/within/after the range.
        pub fn ordering(&self, pos: (usize, usize)) -> Ordering {
            let (sx, sy) = self.start;
            let (ex, ey) = self.end;
            let (x, y) = pos;

            if y < sy {
                Ordering::Greater
            } else if y == sy {
                if y < ey {
                    if x < sx {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                } else if y == ey {
                    if x < sx {
                        Ordering::Greater
                    } else if x < ex {
                        Ordering::Equal
                    } else {
                        Ordering::Less
                    }
                } else {
                    // ey < sy
                    unreachable!()
                }
            } else {
                if y < ey {
                    Ordering::Equal
                } else if y == ey {
                    if x < ex {
                        Ordering::Equal
                    } else {
                        Ordering::Less
                    }
                } else {
                    Ordering::Less
                }
            }
        }
    }

    // This needs its own impl, because the order is exactly wrong.
    // For any sane range I'd need (row,col) but what I got is (col,row).
    // Need this to conform with the rest of ratatui ...
    impl PartialOrd for TextRange {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            let (sx, sy) = self.start;
            let (ex, ey) = self.end;
            let (osx, osy) = other.start;
            let (oex, oey) = other.end;

            if sy < osy {
                Some(Ordering::Less)
            } else if sy > osy {
                Some(Ordering::Greater)
            } else {
                if sx < osx {
                    Some(Ordering::Less)
                } else if sx > osx {
                    Some(Ordering::Greater)
                } else {
                    if ey < oey {
                        Some(Ordering::Less)
                    } else if ey > oey {
                        Some(Ordering::Greater)
                    } else {
                        if ex < oex {
                            Some(Ordering::Less)
                        } else if ex > oex {
                            Some(Ordering::Greater)
                        } else {
                            Some(Ordering::Equal)
                        }
                    }
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
        pub fn new() -> Self {
            Self::default()
        }

        /// Remove all styles.
        pub fn clear_styles(&mut self) {
            self.styles.clear();
        }

        /// Add a text-style for a range.
        ///
        /// The same range can be added again with a different style.
        /// Overlapping regions get the merged style.
        pub fn add_style(&mut self, range: TextRange, style: usize) {
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
        pub fn styles_after_mut(
            &mut self,
            pos: (usize, usize),
        ) -> Skip<IterMut<(TextRange, usize)>> {
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
        pub fn styles_at(&self, pos: (usize, usize), result: &mut Vec<usize>) {
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
            cx = min(cx, self.line_width(cy));

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

        /// Text value.
        #[inline]
        pub fn value(&self) -> String {
            String::from(&self.value)
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
        pub fn line(&self, n: usize) -> RopeGraphemes<'_> {
            let line = self.value.lines_at(n).next();
            if let Some(line) = line {
                RopeGraphemes::new(line)
            } else {
                RopeGraphemes::new(RopeSlice::from(""))
            }
        }

        /// Returns a line as an iterator over the graphemes for the line.
        pub fn line_idx(&self, n: usize) -> RopeGraphemesIdx<'_> {
            let line = self.value.lines_at(n).next();
            if let Some(line) = line {
                RopeGraphemesIdx::new(line)
            } else {
                RopeGraphemesIdx::new(RopeSlice::from(""))
            }
        }

        /// Line width as grapheme count.
        pub fn line_width(&self, n: usize) -> usize {
            let line = self.value.lines_at(n).next();
            if let Some(line) = line {
                rope_len(line)
            } else {
                0
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
            let last_width = self.line_width(last);
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
                unreachable!()
            };
            ScrolledIter {
                lines: l,
                offset: self.offset.0,
            }
        }

        fn byte_of(&self, pos: (usize, usize)) -> usize {
            let line_byte = self.value.line_to_byte(pos.1);
            let mut it = self.line_idx(pos.1);
            for (col, (byte, _cc)) in it.enumerate() {
                if pos.0 == col {
                    return line_byte + byte;
                }
            }
            panic!("byte_of");
        }

        fn char_of(&self, pos: (usize, usize)) -> usize {
            let byte_pos = self.byte_of(pos);
            self.value.byte_to_char(byte_pos)
        }

        pub fn insert_char(&mut self, c: char) {
            let char_pos = self.char_of(self.cursor);
            self.value.insert_char(char_pos, c);

            for (r, _) in self.styles.styles_after_mut(self.cursor) {
                if r.start.1 == self.cursor.1 {
                    if r.start.0 >= self.cursor.0 {
                        r.start.0 += 1;
                    }
                }
                if r.end.1 == self.cursor.1 {
                    if r.end.0 >= self.cursor.0 {
                        r.end.0 += 1;
                    }
                }
            }

            if self.anchor.0 >= self.cursor.0 {
                self.anchor.0 += 1;
            }
            self.cursor.0 += 1;
        }

        pub fn insert_newline(&mut self) {
            let char_pos = self.char_of(self.cursor);
        }
    }
}
