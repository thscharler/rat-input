//!
//! A simple menu. No submenus.
//!
//! Supports hot-keys with '_' in the item text.
//!
use crate::_private::NonExhaustive;
use crate::util::{next_opt, prev_opt, revert_style, span_width};
#[allow(unused_imports)]
use log::debug;
use rat_event::util::MouseFlags;
use rat_event::{ct_event, ConsumedEvent, FocusKeys, HandleEvent, MouseOnly, Outcome};
use rat_focus::FocusFlag;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{StatefulWidget, StatefulWidgetRef, Widget};
use std::cmp::min;
use std::fmt::Debug;

///
/// Menu widget.
///
/// If the text exceeds the area width it wraps around.
#[derive(Debug, Default, Clone)]
pub struct MenuLine<'a> {
    style: Style,
    title_style: Option<Style>,
    select_style: Option<Style>,
    focus_style: Option<Style>,
    title: Span<'a>,
    key: Vec<char>,
    menu: Vec<Vec<Span<'a>>>,
}

/// Combined styles.
#[derive(Debug, Clone)]
pub struct MenuStyle {
    pub style: Style,
    pub title: Option<Style>,
    pub select: Option<Style>,
    pub focus: Option<Style>,
    pub non_exhaustive: NonExhaustive,
}

///
/// State for the menu widget
///
#[derive(Debug, Clone)]
pub struct MenuLineState {
    /// Current focus state.
    pub focus: FocusFlag,
    /// Focus
    pub area: Rect,
    /// Areas for each item.
    pub areas: Vec<Rect>,
    /// Hot keys
    pub key: Vec<char>,
    /// Selected item.
    pub selected: Option<usize>,

    /// Flags for mouse handling.
    pub mouse: MouseFlags,

    pub non_exhaustive: NonExhaustive,
}

impl<'a> MenuLine<'a> {
    /// New
    pub fn new() -> Self {
        Default::default()
    }

    /// Combined style.
    #[inline]
    pub fn styles(mut self, styles: MenuStyle) -> Self {
        self.style = styles.style;
        self.title_style = styles.title;
        self.select_style = styles.select;
        self.focus_style = styles.focus;
        self
    }

    /// Base style.
    #[inline]
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.style = style.into();
        self
    }

    /// Menu-title style.
    #[inline]
    pub fn title_style(mut self, style: impl Into<Style>) -> Self {
        self.title_style = Some(style.into());
        self
    }

    /// Selection
    #[inline]
    pub fn select_style(mut self, style: impl Into<Style>) -> Self {
        self.select_style = Some(style.into());
        self
    }

    /// Selection + Focus
    #[inline]
    pub fn select_style_focus(mut self, style: impl Into<Style>) -> Self {
        self.focus_style = Some(style.into());
        self
    }

    /// Title text.
    #[inline]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Span::from(title);
        self
    }

    /// Add item.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, menu_item: &'a str) -> Self {
        let (key, item) = menu_span(menu_item);
        self.key.push(key);
        self.menu.push(item);
        self
    }
}

impl<'a> StatefulWidgetRef for MenuLine<'a> {
    type State = MenuLineState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_ref(self, area, buf, state);
    }
}

impl<'a> StatefulWidget for MenuLine<'a> {
    type State = MenuLineState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_ref(&self, area, buf, state);
    }
}

fn render_ref(widget: &MenuLine<'_>, area: Rect, buf: &mut Buffer, state: &mut MenuLineState) {
    let mut row = area.y;
    let mut col = area.x;

    state.area = area;
    state.areas.clear();
    state.key.clear();
    state.key.extend(widget.key.iter());
    state.selected = min(state.selected, Some(widget.menu.len().saturating_sub(1)));

    let select_style = if state.focus.get() {
        if let Some(focus_style) = widget.focus_style {
            focus_style
        } else {
            revert_style(widget.style)
        }
    } else {
        if let Some(select_style) = widget.select_style {
            select_style
        } else {
            revert_style(widget.style)
        }
    };
    let title_style = if let Some(title_style) = widget.title_style {
        title_style
    } else {
        widget.style.underlined()
    };

    buf.set_style(area, widget.style);

    let mut text = Text::default();
    let mut line = Line::default();

    if !widget.title.content.is_empty() {
        let title_width = widget.title.width() as u16;

        line.spans.push(widget.title.clone().style(title_style));
        line.spans.push(" ".into());

        col += title_width + 1;
    }

    'f: {
        for (n, item) in widget.menu.iter().enumerate() {
            let item_width = span_width(item);

            // line breaks
            if col + item_width > area.x + area.width {
                text.lines.push(line);

                if row + 1 >= area.y + area.height {
                    break 'f;
                }

                line = Line::default();

                row += 1;
                col = area.x;
            }

            state
                .areas
                .push(Rect::new(col, row, item_width, 1).intersection(area));

            if state.selected == Some(n) {
                for mut v in item.iter().cloned() {
                    v.style = v.style.patch(select_style);
                    line.spans.push(v);
                }
            } else {
                line.spans.extend(item.iter().cloned());
            }
            line.spans.push(" ".into());

            col += item_width + 1;
        }
        // for-else
        text.lines.push(line);
    }

    text.render(area, buf);
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

#[allow(clippy::len_without_is_empty)]
impl MenuLineState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders the widget in focused style.
    ///
    /// This flag is not used for event-handling.
    #[inline]
    pub fn set_focused(&mut self, focus: bool) {
        self.focus.focus.set(focus);
    }

    /// Renders the widget in focused style.
    ///
    /// This flag is not used for event-handling.
    #[inline]
    pub fn is_focused(&mut self) -> bool {
        self.focus.focus.get()
    }

    /// Number of items.
    #[inline]
    pub fn len(&self) -> usize {
        self.areas.len()
    }

    /// Selected index
    #[inline]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Select
    #[inline]
    pub fn select(&mut self, select: Option<usize>) -> bool {
        let old_selected = self.selected;
        self.selected = select;
        old_selected != self.selected
    }

    /// Select by hotkey
    #[inline]
    pub fn select_by_key(&mut self, cc: char) -> bool {
        let old_selected = self.selected;
        let cc = cc.to_ascii_lowercase();
        for (i, k) in self.key.iter().enumerate() {
            if cc == *k {
                self.selected = Some(i);
                break;
            }
        }
        old_selected != self.selected
    }

    /// Item at position.
    #[inline]
    pub fn item_at(&self, pos: (u16, u16)) -> Option<usize> {
        for (i, r) in self.areas.iter().enumerate() {
            if r.contains(pos.into()) {
                return Some(i);
            }
        }
        None
    }

    /// Next item.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        let old_selected = self.selected;
        self.selected = next_opt(self.selected, 1, self.len());
        old_selected != self.selected
    }

    /// Previous item.
    #[inline]
    pub fn prev(&mut self) -> bool {
        let old_selected = self.selected;
        self.selected = prev_opt(self.selected, 1);
        old_selected != self.selected
    }
}

impl Default for MenuLineState {
    fn default() -> Self {
        Self {
            focus: Default::default(),
            key: Default::default(),
            mouse: Default::default(),
            selected: Default::default(),
            areas: Default::default(),
            area: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

/// Outcome for menuline.
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

impl ConsumedEvent for MenuOutcome {
    fn is_consumed(&self) -> bool {
        *self != MenuOutcome::NotUsed
    }
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
                if self.select_by_key(*cc) {
                    MenuOutcome::Activated(self.selected.expect("selected"))
                } else {
                    MenuOutcome::Unchanged
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
                if self.select_by_key(*cc) {
                    MenuOutcome::Activated(self.selected.expect("selected"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            _ => MenuOutcome::NotUsed,
        }
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, MenuOutcome> for MenuLineState {
    #[allow(clippy::redundant_closure)]
    fn handle(&mut self, event: &crossterm::event::Event, _: FocusKeys) -> MenuOutcome {
        let res = match event {
            ct_event!(key press cc) => {
                if self.select_by_key(*cc) {
                    MenuOutcome::Activated(self.selected.expect("selected"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            ct_event!(keycode press Left) => {
                if self.prev() {
                    MenuOutcome::Selected(self.selected.expect("selected"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            ct_event!(keycode press Right) => {
                if self.next() {
                    MenuOutcome::Selected(self.selected.expect("selected"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            ct_event!(keycode press Home) => {
                if self.select(Some(0)) {
                    MenuOutcome::Selected(self.selected.expect("selected"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            ct_event!(keycode press End) => {
                if self.select(Some(self.len().saturating_sub(1))) {
                    MenuOutcome::Selected(self.selected.expect("selected"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            ct_event!(keycode press Enter) => self
                .selected
                .map_or(MenuOutcome::Unchanged, |v| MenuOutcome::Activated(v)),

            ct_event!(key release _)
            | ct_event!(keycode release Left)
            | ct_event!(keycode release Right)
            | ct_event!(keycode release Home)
            | ct_event!(keycode release End)
            | ct_event!(keycode release Enter) => MenuOutcome::Unchanged,

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
            ct_event!(mouse any for m) if self.mouse.doubleclick(self.area, m) => {
                let idx = self.item_at(self.mouse.pos_of(m));
                if self.selected() == idx {
                    match self.selected {
                        Some(a) => MenuOutcome::Activated(a),
                        None => MenuOutcome::NotUsed,
                    }
                } else {
                    MenuOutcome::NotUsed
                }
            }
            ct_event!(mouse any for m) if self.mouse.drag(self.area, m) => {
                if let Some(i) = self.item_at(self.mouse.pos_of(m)) {
                    self.select(Some(i));
                    match self.selected {
                        Some(a) => MenuOutcome::Selected(a),
                        None => unreachable!(),
                    }
                } else {
                    MenuOutcome::NotUsed
                }
            }
            ct_event!(mouse down Left for col, row) => {
                if let Some(i) = self.item_at((*col, *row)) {
                    self.select(Some(i));
                    match self.selected {
                        Some(a) => MenuOutcome::Selected(a),
                        None => unreachable!(),
                    }
                } else {
                    MenuOutcome::NotUsed
                }
            }
            _ => MenuOutcome::NotUsed,
        }
    }
}
