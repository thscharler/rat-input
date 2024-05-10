//!
//! A simple menu. No submenus.
//!
//! Supports hot-keys with '_' in the item text.
//! The keys are trigger with Ctrl or the plain
//! key if the menu has focus.
//!
use crate::_private::NonExhaustive;
use crate::event::Outcome;
use crate::util::MouseFlags;
use crate::util::{next_opt, prev_opt, span_width};
use rat_event::{ct_event, FocusKeys, HandleEvent, MouseOnly};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::{Modifier, Span, Style, Widget};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::StatefulWidget;
use std::cmp::min;
use std::fmt::Debug;

/// Menu
#[derive(Debug)]
pub struct MenuLine<'a> {
    style: Style,
    title_style: Option<Style>,
    select_style: Option<Style>,
    focus_style: Option<Style>,
    title: Span<'a>,
    key: Vec<char>,
    menu: Vec<Vec<Span<'a>>>,
    focused: bool,
}

/// Combined styles.
#[derive(Debug)]
pub struct MenuStyle {
    pub style: Style,
    pub title: Option<Style>,
    pub select: Option<Style>,
    pub focus: Option<Style>,
    pub non_exhaustive: NonExhaustive,
}

impl<'a> Default for MenuLine<'a> {
    fn default() -> Self {
        Self {
            style: Default::default(),
            title_style: Default::default(),
            select_style: Default::default(),
            focus_style: Default::default(),
            title: Default::default(),
            key: vec![],
            menu: vec![],
            focused: false,
        }
    }
}

impl<'a> MenuLine<'a> {
    /// New
    pub fn new() -> Self {
        Default::default()
    }

    /// Combined style.
    pub fn styles(mut self, styles: MenuStyle) -> Self {
        self.style = styles.style;
        self.title_style = styles.title;
        self.select_style = styles.select;
        self.focus_style = styles.focus;
        self
    }

    /// Base style.
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.style = style.into();
        self
    }

    /// Menu-title style.
    pub fn title_style(mut self, style: impl Into<Style>) -> Self {
        self.title_style = Some(style.into());
        self
    }

    /// Selection
    pub fn select_style(mut self, style: impl Into<Style>) -> Self {
        self.select_style = Some(style.into());
        self
    }

    /// Selection + Focus
    pub fn select_style_focus(mut self, style: impl Into<Style>) -> Self {
        self.focus_style = Some(style.into());
        self
    }

    /// Title text.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Span::from(title);
        self
    }

    /// Renders the content differently if focused.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Add item.
    pub fn add(mut self, menu_item: &'a str) -> Self {
        let (key, item) = menu_span(menu_item);
        self.key.push(key);
        self.menu.push(item);
        self
    }
}

impl<'a> StatefulWidget for MenuLine<'a> {
    type State = MenuLineState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut row = area.y;
        let mut col = area.x;

        state.area = area;
        state.key = self.key;
        state.selected = min(state.selected, Some(self.menu.len() - 1));

        let select_style = if self.focused {
            if let Some(focus_style) = self.focus_style {
                focus_style
            } else {
                self.style.reversed()
            }
        } else {
            if let Some(select_style) = self.select_style {
                select_style
            } else {
                self.style.reversed()
            }
        };
        let title_style = if let Some(title_style) = self.title_style {
            title_style
        } else {
            self.style.underlined()
        };

        let mut text = Text::default();
        let mut line = Line::default();

        if !self.title.content.is_empty() {
            let title_width = self.title.width() as u16;

            line.spans.push(self.title.style(title_style));
            line.spans.push(" ".into());

            col += title_width + 1;
        }

        for (n, mut item) in self.menu.into_iter().enumerate() {
            let item_width = span_width(&item);
            if col + item_width > area.x + area.width {
                text.lines.push(line);
                line = Line::default();

                row += 1;
                col = area.x;
            }

            if state.selected == Some(n) {
                for v in &mut item {
                    v.style = v.style.patch(select_style)
                }
            }

            state.areas.push(Rect::new(col, row, item_width, 1));

            line.spans.extend(item);
            line.spans.push(" ".into());

            col += item_width + 1;
        }
        text.lines.push(line);

        text.style = self.style;
        text.render(area, buf);
    }
}

fn menu_span(txt: &str) -> (char, Vec<Span<'_>>) {
    let mut key = char::default();
    let mut menu = Vec::new();

    let mut it = txt.split('_');
    if let Some(p) = it.next() {
        if !p.is_empty() {
            menu.push(Span::from(p));
        }
    }

    for t in it {
        let mut cit = t.char_indices();
        // mark first char
        cit.next();
        if let Some((i, _)) = cit.next() {
            let (t0, t1) = t.split_at(i);

            key = t0.chars().next().expect("char");
            key = key.to_lowercase().next().expect("char");

            menu.push(Span::styled(t0, Style::from(Modifier::UNDERLINED)));
            menu.push(Span::from(t1));
        } else {
            menu.push(Span::from(t));
        }
    }

    (key, menu)
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self {
            style: Default::default(),
            title: Default::default(),
            select: Default::default(),
            focus: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

/// State for the menu.
#[derive(Debug)]
pub struct MenuLineState {
    /// Focus
    pub area: Rect,
    pub areas: Vec<Rect>,
    pub key: Vec<char>,
    pub selected: Option<usize>,
    pub mouse: MouseFlags,
    pub non_exhaustive: NonExhaustive,
}

#[allow(clippy::len_without_is_empty)]
impl MenuLineState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.areas.len()
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select(&mut self, select: Option<usize>) {
        self.selected = select;
    }

    pub fn select_by_key(&mut self, cc: char) {
        let cc = cc.to_ascii_lowercase();
        for (i, k) in self.key.iter().enumerate() {
            if cc == *k {
                self.selected = Some(i);
                break;
            }
        }
    }

    pub fn item_at(&self, pos: Position) -> Option<usize> {
        for (i, r) in self.areas.iter().enumerate() {
            if r.contains(pos) {
                return Some(i);
            }
        }
        None
    }

    pub fn next(&mut self) {
        self.selected = next_opt(self.selected, 1, self.len() + 1);
    }

    pub fn prev(&mut self) {
        self.selected = prev_opt(self.selected, 1);
    }
}

impl Default for MenuLineState {
    fn default() -> Self {
        Self {
            key: Default::default(),
            mouse: Default::default(),
            selected: Default::default(),
            areas: Default::default(),
            area: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

/// Outcome for menuline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuOutcome {
    /// The given event was not handled at all.
    NotUsed,
    /// The event was handled, no repaint necessary.
    Unchanged,
    /// The event was handled, repaint necessary.
    Changed,
    /// The menuitem was selected.
    Selected(usize),
    /// The menuitem was selected and activated.
    Activated(usize),
}

impl From<MenuOutcome> for Outcome {
    fn from(value: MenuOutcome) -> Self {
        match value {
            MenuOutcome::NotUsed => Outcome::NotUsed,
            MenuOutcome::Unchanged => Outcome::Unchanged,
            MenuOutcome::Changed => Outcome::Changed,
            MenuOutcome::Selected(_) => Outcome::Changed,
            MenuOutcome::Activated(_) => Outcome::Changed,
        }
    }
}

/// React to ctrl + menu shortcut.
#[derive(Debug)]
pub struct HotKeyCtrl;

impl HandleEvent<crossterm::event::Event, HotKeyCtrl, MenuOutcome> for MenuLineState {
    fn handle(&mut self, event: &crossterm::event::Event, _: HotKeyCtrl) -> MenuOutcome {
        match event {
            ct_event!(key release CONTROL-cc) => {
                self.select_by_key(*cc);
                match self.selected {
                    Some(a) => MenuOutcome::Activated(a),
                    None => MenuOutcome::NotUsed,
                }
            }
            _ => MenuOutcome::NotUsed,
        }
    }
}

/// React to alt + menu shortcut.
#[derive(Debug)]
pub struct HotKeyAlt;

impl HandleEvent<crossterm::event::Event, HotKeyAlt, MenuOutcome> for MenuLineState {
    fn handle(&mut self, event: &crossterm::event::Event, _: HotKeyAlt) -> MenuOutcome {
        match event {
            ct_event!(key release ALT-cc) => {
                self.select_by_key(*cc);
                match self.selected {
                    Some(a) => MenuOutcome::Activated(a),
                    None => MenuOutcome::NotUsed,
                }
            }
            _ => MenuOutcome::NotUsed,
        }
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, MenuOutcome> for MenuLineState {
    fn handle(&mut self, event: &crossterm::event::Event, _: FocusKeys) -> MenuOutcome {
        let res = match event {
            ct_event!(key release cc) => {
                self.select_by_key(*cc);
                match self.selected {
                    Some(a) => MenuOutcome::Activated(a),
                    None => MenuOutcome::NotUsed,
                }
            }
            ct_event!(keycode release Left) => {
                self.prev();
                match self.selected {
                    Some(a) => MenuOutcome::Selected(a),
                    None => unreachable!(),
                }
            }
            ct_event!(keycode release Right) => {
                self.next();
                match self.selected {
                    Some(a) => MenuOutcome::Selected(a),
                    None => unreachable!(),
                }
            }
            ct_event!(keycode release Home) => {
                self.select(Some(0));
                match self.selected {
                    Some(a) => MenuOutcome::Selected(a),
                    None => unreachable!(),
                }
            }
            ct_event!(keycode release End) => {
                self.select(Some(self.len() - 1));
                match self.selected {
                    Some(a) => MenuOutcome::Selected(a),
                    None => unreachable!(),
                }
            }
            ct_event!(keycode release Enter) => match self.selected {
                Some(a) => MenuOutcome::Activated(a),
                None => MenuOutcome::NotUsed,
            },
            _ => MenuOutcome::NotUsed,
        };

        if res == MenuOutcome::NotUsed {
            self.handle(event, MouseOnly)
        } else {
            res
        }
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, MenuOutcome> for MenuLineState {
    fn handle(&mut self, event: &crossterm::event::Event, _: MouseOnly) -> MenuOutcome {
        match event {
            ct_event!(mouse down Left for col, row) => {
                if let Some(i) = self.item_at(Position::new(*col, *row)) {
                    self.mouse.set_drag();
                    self.select(Some(i));
                    match self.selected {
                        Some(a) => MenuOutcome::Selected(a),
                        None => unreachable!(),
                    }
                } else {
                    MenuOutcome::NotUsed
                }
            }
            ct_event!(mouse drag Left for col, row) => {
                if self.mouse.do_drag() {
                    if let Some(i) = self.item_at(Position::new(*col, *row)) {
                        self.mouse.set_drag();
                        self.select(Some(i));
                        match self.selected {
                            Some(a) => MenuOutcome::Selected(a),
                            None => unreachable!(),
                        }
                    } else {
                        MenuOutcome::NotUsed
                    }
                } else {
                    MenuOutcome::NotUsed
                }
            }
            ct_event!(mouse up Left for col,row) => {
                let idx = self.item_at(Position::new(*col, *row));
                if self.selected() == idx && self.mouse.pull_trigger(500) {
                    match self.selected {
                        Some(a) => MenuOutcome::Activated(a),
                        None => MenuOutcome::NotUsed,
                    }
                } else {
                    MenuOutcome::NotUsed
                }
            }
            ct_event!(mouse moved) => {
                self.mouse.clear_drag();
                MenuOutcome::NotUsed
            }
            _ => MenuOutcome::NotUsed,
        }
    }
}
