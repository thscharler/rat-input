//!
//! Basic status line with multiple sections.
//!

use crate::_private::NonExhaustive;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{StatefulWidget, Widget};
use std::fmt::Debug;

/// Basic status line with multiple sections.
#[derive(Debug, Default, Clone)]
pub struct StatusLine {
    style: Vec<Style>,
    widths: Vec<Constraint>,
}

/// State for the status line.
#[derive(Debug, Clone)]
pub struct StatusLineState {
    /// Total area
    pub area: Rect,
    /// Areas for each section.
    pub areas: Vec<Rect>,
    /// Statustext for each section.
    pub status: Vec<String>,

    pub non_exhaustive: NonExhaustive,
}

impl StatusLine {
    /// New widget.
    pub fn new() -> Self {
        Self {
            style: Default::default(),
            widths: Default::default(),
        }
    }

    /// Layout for the sections.
    ///
    /// This layout determines the number of sections.
    /// If the styles or the statustext vec differ defaults are used.
    pub fn layout<It, Item>(mut self, widths: It) -> Self
    where
        It: IntoIterator<Item = Item>,
        Item: Into<Constraint>,
    {
        self.widths = widths.into_iter().map(|v| v.into()).collect();
        self
    }

    /// Styles for each section.
    pub fn styles(mut self, style: impl IntoIterator<Item = impl Into<Style>>) -> Self {
        self.style = style.into_iter().map(|v| v.into()).collect();
        self
    }
}

impl Default for StatusLineState {
    fn default() -> Self {
        Self {
            area: Default::default(),
            areas: Default::default(),
            status: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl StatusLineState {
    /// Clear all status text.
    pub fn clear_status(&mut self) {
        self.status.clear();
    }

    /// Set the specific status section.
    pub fn status<S: Into<String>>(&mut self, idx: usize, msg: S) {
        while self.status.len() <= idx {
            self.status.push("".to_string());
        }
        self.status[idx] = msg.into();
    }
}

impl StatefulWidget for StatusLine {
    type State = StatusLineState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;

        let layout = Layout::horizontal(self.widths).split(state.area);

        for (i, rect) in layout.iter().enumerate() {
            let style = self.style.get(i).copied().unwrap_or_default();
            let txt = state.status.get(i).map(|v| v.as_str()).unwrap_or("");

            buf.set_style(*rect, style);
            Span::from(txt).render(*rect, buf);
        }
    }
}
