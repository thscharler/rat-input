//!
//! A button widget.
//!

use crate::_private::NonExhaustive;
use crate::event::Outcome;
use rat_event::{ct_event, FocusKeys, HandleEvent, MouseOnly, UsedEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::prelude::{BlockExt, Span, StatefulWidget};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, StatefulWidgetRef, WidgetRef};

/// Button widget.
#[derive(Debug, Default)]
pub struct Button<'a> {
    text: Text<'a>,
    style: Style,
    focus_style: Option<Style>,
    armed_style: Option<Style>,
    block: Option<Block<'a>>,
    focused: bool,
}

/// Composite style.
#[derive(Debug)]
pub struct ButtonStyle {
    pub style: Style,
    pub focus: Option<Style>,
    pub armed: Option<Style>,
    pub non_exhaustive: NonExhaustive,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            style: Default::default(),
            focus: Default::default(),
            armed: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl<'a> Button<'a> {
    #[inline]
    pub fn styles(mut self, styles: ButtonStyle) -> Self {
        self.style = styles.style;
        self.focus_style = styles.focus;
        self.armed_style = styles.armed;
        self
    }

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

    #[inline]
    pub fn armed_style(mut self, style: impl Into<Style>) -> Self {
        self.armed_style = Some(style.into());
        self
    }

    #[inline]
    pub fn text(mut self, text: impl Into<Text<'a>>) -> Self {
        self.text = text.into();
        self
    }

    #[inline]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Renders differently if focused.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl<'a> From<&'a str> for Button<'a> {
    fn from(value: &'a str) -> Self {
        Self {
            text: Text::from(value).centered(),
            ..Default::default()
        }
    }
}

impl<'a> From<String> for Button<'a> {
    fn from(value: String) -> Self {
        Self {
            text: Text::from(value).centered(),
            ..Default::default()
        }
    }
}

impl<'a> From<Span<'a>> for Button<'a> {
    fn from(value: Span<'a>) -> Self {
        Self {
            text: Text::from(value).centered(),
            ..Default::default()
        }
    }
}

impl<'a, const N: usize> From<[Span<'a>; N]> for Button<'a> {
    fn from(value: [Span<'a>; N]) -> Self {
        let mut text = Text::default();
        for value in value {
            text.push_span(value);
        }
        Self {
            text: text.centered(),
            ..Default::default()
        }
    }
}

impl<'a> From<Vec<Span<'a>>> for Button<'a> {
    fn from(value: Vec<Span<'a>>) -> Self {
        let mut text = Text::default();
        for value in value {
            text.push_span(value);
        }
        Self {
            text: text.centered(),
            ..Default::default()
        }
    }
}

impl<'a> From<Line<'a>> for Button<'a> {
    fn from(value: Line<'a>) -> Self {
        Self {
            text: Text::from(value).centered(),
            ..Default::default()
        }
    }
}

impl<'a> StatefulWidget for Button<'a> {
    type State = ButtonState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.render_ref(area, buf, state);
    }
}

impl<'a> StatefulWidgetRef for Button<'a> {
    type State = ButtonState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;
        state.inner_area = self.block.inner_if_some(area);

        self.block.render_ref(area, buf);

        let focus_style = if let Some(focus_style) = self.focus_style {
            focus_style
        } else {
            self.style
        };
        let armed_style = if let Some(armed_style) = self.armed_style {
            armed_style
        } else {
            self.style.reversed()
        };

        if state.armed {
            buf.set_style(state.inner_area, armed_style);
        } else {
            if self.focused {
                buf.set_style(state.inner_area, focus_style);
            } else {
                buf.set_style(state.inner_area, self.style);
            }
        }

        let layout = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(self.text.height() as u16),
            Constraint::Fill(1),
        ])
        .split(state.inner_area);

        self.text.render_ref(layout[1], buf);
    }
}

#[derive(Debug)]
pub struct ButtonState {
    pub area: Rect,
    pub inner_area: Rect,

    pub armed: bool,

    pub non_exhaustive: NonExhaustive,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self {
            area: Default::default(),
            inner_area: Default::default(),
            armed: false,
            non_exhaustive: NonExhaustive,
        }
    }
}

/// Result value for event-handling.
///
/// Adds `Pressed` to the general Outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonOutcome {
    /// The given event was not handled at all.
    NotUsed,
    /// The event was handled, no repaint necessary.
    Unchanged,
    /// The event was handled, repaint necessary.
    Changed,
    /// Button has been pressed.
    Pressed,
}

impl UsedEvent for ButtonOutcome {
    fn used_event(&self) -> bool {
        *self != ButtonOutcome::NotUsed
    }
}

impl From<ButtonOutcome> for Outcome {
    fn from(value: ButtonOutcome) -> Self {
        match value {
            ButtonOutcome::NotUsed => Outcome::NotUsed,
            ButtonOutcome::Unchanged => Outcome::Unchanged,
            ButtonOutcome::Changed => Outcome::Changed,
            ButtonOutcome::Pressed => Outcome::Changed,
        }
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, ButtonOutcome> for ButtonState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: FocusKeys) -> ButtonOutcome {
        let r = match event {
            ct_event!(keycode press Enter) => {
                self.armed = true;
                ButtonOutcome::Changed
            }
            ct_event!(keycode release Enter) => {
                self.armed = false;
                ButtonOutcome::Pressed
            }
            _ => ButtonOutcome::NotUsed,
        };

        if r == ButtonOutcome::NotUsed {
            HandleEvent::handle(self, event, MouseOnly)
        } else {
            r
        }
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, ButtonOutcome> for ButtonState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: MouseOnly) -> ButtonOutcome {
        match event {
            ct_event!(mouse down Left for column, row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    self.armed = true;
                    ButtonOutcome::Changed
                } else {
                    ButtonOutcome::NotUsed
                }
            }
            ct_event!(mouse up Left for column, row) => {
                if self.area.contains(Position::new(*column, *row)) {
                    self.armed = false;
                    ButtonOutcome::Pressed
                } else {
                    if self.armed {
                        self.armed = false;
                        ButtonOutcome::Changed
                    } else {
                        ButtonOutcome::NotUsed
                    }
                }
            }
            _ => ButtonOutcome::NotUsed,
        }
    }
}

/// Handle all events.
/// Text events are only processed if focus is true.
/// Mouse events are processed if they are in range.
pub fn handle_events(
    state: &mut ButtonState,
    focus: bool,
    event: &crossterm::event::Event,
) -> ButtonOutcome {
    if focus {
        HandleEvent::handle(state, event, FocusKeys)
    } else {
        HandleEvent::handle(state, event, MouseOnly)
    }
}

/// Handle only mouse-events.
pub fn handle_mouse_events(
    state: &mut ButtonState,
    event: &crossterm::event::Event,
) -> ButtonOutcome {
    HandleEvent::handle(state, event, MouseOnly)
}
