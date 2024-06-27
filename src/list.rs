//!
//! Minimal adapter for ratatui::List
//!
//! Has the same widget api but a different state.
//! Can do the basic navigation within the list.
//!

use rat_event::util::MouseFlags;
use rat_event::{ct_event, flow, FocusKeys, HandleEvent, MouseOnly, Outcome};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{StatefulWidget, Style};
use ratatui::widgets::{Block, HighlightSpacing, StatefulWidgetRef};
use std::cmp::min;

pub use ratatui::widgets::{ListDirection, ListItem};

///
/// Minimal wrapper around ratatui List to get some event handling.
///
/// This currently works only for lists with row-height of 1.
///
#[derive(Debug, Clone, Default)]
pub struct List<'a> {
    widget: ratatui::widgets::List<'a>,
}

/// State for the list-adapter.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    /// List area
    pub area: Rect,

    /// Offset. Will be corrected to always show the selected item.
    pub offset: usize,
    /// Selection.
    pub selected: Option<usize>,
    /// Total len of the list.
    pub len: usize,

    /// Mouse helper.
    pub mouse: MouseFlags,
}

impl<'a> List<'a> {
    /// New with items.
    #[inline]
    pub fn new<T>(items: T) -> Self
    where
        T: IntoIterator,
        T::Item: Into<ListItem<'a>>,
    {
        Self {
            widget: ratatui::widgets::List::new(items),
        }
    }

    /// Set list items.
    #[inline]
    pub fn items<T>(mut self, items: T) -> Self
    where
        T: IntoIterator,
        T::Item: Into<ListItem<'a>>,
    {
        self.widget = self.widget.items(items);
        self
    }

    /// Set block for borders.
    #[inline]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.widget = self.widget.block(block);
        self
    }

    /// Base style.
    #[inline]
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self {
        self.widget = self.widget.style(style);
        self
    }

    /// List direction.
    pub fn direction(mut self, direction: ListDirection) -> Self {
        self.widget = self.widget.direction(direction);
        self
    }

    /// Sets the number of items around the currently selected item that should be kept visible
    pub fn scroll_padding(mut self, padding: usize) -> Self {
        self.widget = self.widget.scroll_padding(padding);
        self
    }

    /// Len of the items.
    pub fn len(&self) -> usize {
        self.widget.len()
    }

    /// Has any items.
    pub fn is_empty(&self) -> bool {
        self.widget.is_empty()
    }
}

impl<'a> StatefulWidget for List<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;
        state.len = self.widget.len();

        self.widget.render(
            area,
            buf,
            &mut ratatui::widgets::ListState::default()
                .with_offset(state.offset)
                .with_selected(state.selected),
        );
    }
}

impl<'a> StatefulWidgetRef for List<'a> {
    type State = ListState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;
        state.len = self.widget.len();

        self.widget.render_ref(
            area,
            buf,
            &mut ratatui::widgets::ListState::default()
                .with_offset(state.offset)
                .with_selected(state.selected),
        );
    }
}

impl ListState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Selected item.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Select item.
    /// Changes to offset to have the selection visible.
    pub fn select(&mut self, select: Option<usize>) -> bool {
        let old = self.selected;
        self.selected = select;
        self.limit_offset();
        old != self.selected
    }

    /// Current offset
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Change the offset. Will not work if this causes the selection to be
    /// out of view.
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Navigate to the first item.
    pub fn first(&mut self) -> bool {
        let old = self.selected;
        if self.len > 0 {
            self.selected = Some(0);
        } else {
            self.selected = None;
        }

        self.limit_offset();

        old != self.selected
    }

    /// Navigate to the last item.
    pub fn last(&mut self) -> bool {
        let old = self.selected;
        if self.len > 0 {
            self.selected = Some(self.len - 1);
        } else {
            self.selected = None;
        }

        self.limit_offset();

        old != self.selected
    }

    /// Navigate to a previous item.
    pub fn prev(&mut self, n: usize) -> bool {
        let old = self.selected;
        if let Some(selected) = self.selected {
            self.selected = Some(selected.saturating_sub(n));
        } else {
            self.selected = Some(self.len.saturating_sub(n));
        }

        self.limit_offset();

        old != self.selected
    }

    /// Navigate to a next item.
    pub fn next(&mut self, n: usize) -> bool {
        let old = self.selected;
        if let Some(selected) = self.selected {
            self.selected = Some(min(selected + n, self.len.saturating_sub(1)));
        } else {
            self.selected = Some(min(n, self.len.saturating_sub(1)));
        }

        self.limit_offset();

        old != self.selected
    }

    /// Row at given position.
    pub fn row_at_clicked(&self, pos: (u16, u16)) -> Option<usize> {
        if pos.1 >= self.area.top() && pos.1 < self.area.bottom() {
            Some(self.offset + pos.1 as usize - self.area.y as usize)
        } else {
            None
        }
    }

    /// Row when dragging. Can go outside the area.
    pub fn row_at_drag(&self, pos: (u16, u16)) -> usize {
        match rat_event::util::row_at_drag(self.area, &[self.area], pos.1) {
            Ok(_) => {
                let d = pos.1 - self.area.y;
                self.offset + d as usize
            }
            Err(v) if v <= 0 => self.offset.saturating_sub((-v) as usize),
            Err(v) => min(
                self.offset + self.area.height as usize + v as usize,
                self.len().saturating_sub(1),
            ),
        }
    }

    fn limit_offset(&mut self) {
        if let Some(selected) = self.selected {
            if self.offset > selected {
                self.offset = selected;
            }
            // TODO: works only for row-heights of 1.
            if (self.offset + self.area.height as usize) < selected {
                self.offset = selected - self.area.height as usize;
            }
        }
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, Outcome> for ListState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: FocusKeys) -> Outcome {
        flow!(match event {
            ct_event!(keycode press Up) => self.prev(1).into(),
            ct_event!(keycode press Down) => self.next(1).into(),
            ct_event!(keycode press PageUp) => self.prev(self.area.height as usize).into(),
            ct_event!(keycode press PageDown) => self.next(self.area.height as usize).into(),
            ct_event!(keycode press Home) => self.first().into(),
            ct_event!(keycode press End) => self.last().into(),
            _ => Outcome::NotUsed,
        });

        self.handle(event, MouseOnly)
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for ListState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: MouseOnly) -> Outcome {
        match event {
            ct_event!(mouse any for m) if self.mouse.drag(self.area, m) => self
                .select(Some(self.row_at_drag((m.column, m.row))))
                .into(),
            ct_event!(mouse down Left for col,row) if self.area.contains((*col, *row).into()) => {
                if let Some(row) = self.row_at_clicked((*col, *row)) {
                    self.select(Some(row)).into()
                } else {
                    Outcome::Unchanged
                }
            }
            ct_event!(scroll down for col, row) if self.area.contains((*col, *row).into()) => {
                self.next(1).into()
            }
            ct_event!(scroll up for col, row) if self.area.contains((*col, *row).into()) => {
                self.prev(1).into()
            }
            _ => Outcome::NotUsed,
        }
    }
}
