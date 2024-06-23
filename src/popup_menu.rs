//!
//! This widget draws a popup-menu.
//!
//! It diverges from other widgets as this widget doesn't draw
//! *inside* the given area but aims to stay *outside* of it.
//!
//! You can give a [Placement] where the popup-menu should appear
//! relative to the given area.
//!
//! If you want it to appear at a mouse-click position, use a
//! `Rect::new(mouse_x, mouse_y, 0,0)` area.
//! If you want it to appear next to a given widget, use
//! the widgets drawing area.
//!
//! ## Navigation keys
//! If you give plain-text strings as items, the underscore
//! designates a navigation key. If you hit the key, the matching
//! item is selected. On the second hit, the matching item is
//! activated.
//!

use crate::fill::Fill;
use crate::menuline::{MenuOutcome, MenuStyle};
use crate::util::menu_str;
use crossterm::event::Event;
use rat_event::util::item_at_clicked;
use rat_event::{ct_event, FocusKeys, HandleEvent};
use rat_focus::ZRect;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{StatefulWidget, Stylize};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Widget, WidgetRef};
use std::cmp::min;

/// Placement relative to the Rect given to render.
///
/// The popup-menu is always rendered outside the box,
/// and this gives the relative placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// On top of the given area. Placed slightly left, so that
    /// the menu text aligns with the left border.
    Top,
    /// Placed left-top of the given area.
    /// For a submenu opening to the left.
    Left,
    /// Placed right-top of the given area.
    /// For a submenu opening to the right.
    Right,
    /// Below the bottom of the given area. Placed slightly left,
    /// so that the menu text aligns with the left border.
    Bottom,
}

/// Popup menu.
#[derive(Debug)]
pub struct PopupMenu<'a> {
    items: Vec<Line<'a>>,
    navchar: Vec<Option<char>>,

    width: Option<u16>,
    placement: Placement,

    style: Style,
    focus_style: Option<Style>,
    block: Option<Block<'a>>,
}

/// State of the popup-menu.
#[derive(Debug, Default)]
pub struct PopupMenuState {
    /// Total area
    pub area: Rect,
    /// Area with z-index for the focus.
    pub z_area: [ZRect; 1],
    /// Areas for each item.
    pub item_areas: Vec<Rect>,
    /// Letter navigation
    pub navchar: Vec<Option<char>>,

    /// Selected item.
    pub selected: Option<usize>,
}

impl<'a> PopupMenu<'a> {
    fn layout(&self, area: Rect, fit_in: Rect, state: &mut PopupMenuState) {
        let width = if let Some(width) = self.width {
            width
        } else {
            let text_width = self.items.iter().map(|v| v.width()).max();
            ((text_width.unwrap_or(10) as u16) / 2) * 3
        };

        let vertical_margin = if self.block.is_some() { 1 } else { 0 };
        let horizontal_margin = if self.block.is_some() { 2 } else { 1 };
        let len = self.items.len() as u16;

        let mut area = match self.placement {
            Placement::Top => Rect::new(
                area.x.saturating_sub(horizontal_margin),
                area.y.saturating_sub(len + vertical_margin * 2),
                width + horizontal_margin * 2,
                len + vertical_margin * 2,
            ),
            Placement::Left => Rect::new(
                area.x.saturating_sub(width + horizontal_margin * 2),
                area.y,
                width + horizontal_margin * 2,
                len + vertical_margin * 2,
            ),
            Placement::Right => Rect::new(
                area.x + area.width,
                area.y,
                width + horizontal_margin * 2,
                len + vertical_margin * 2,
            ),
            Placement::Bottom => Rect::new(
                area.x.saturating_sub(horizontal_margin),
                area.y + area.height,
                width + horizontal_margin * 2,
                len + vertical_margin * 2,
            ),
        };

        if area.right() >= fit_in.right() {
            area.x -= area.right() - fit_in.right();
        }
        if area.bottom() >= fit_in.bottom() {
            area.y -= area.bottom() - fit_in.bottom();
        }

        state.area = area;
        state.z_area[0] = ZRect::from((1, area));

        state.item_areas.clear();
        let mut r = Rect::new(
            area.x + horizontal_margin,
            area.y + vertical_margin,
            width,
            1,
        );
        for _ in 0..len {
            state.item_areas.push(r);
            r.y += 1;
        }
    }
}

impl<'a> PopupMenu<'a> {
    /// New, empty.
    pub fn new() -> Self {
        Self {
            items: Default::default(),
            navchar: Default::default(),
            style: Default::default(),
            focus_style: None,
            block: None,
            placement: Placement::Top,
            width: None,
        }
    }

    /// Add a formatted item.
    /// The navchar is optional, any markup for it is your problem.
    pub fn add(mut self, item: Line<'a>, navchar: Option<char>) -> Self {
        self.items.push(item);
        self.navchar.push(navchar);
        self
    }

    /// Add a text-item.
    /// The first underscore is used to denote the navchar.
    pub fn add_str(mut self, txt: &'a str) -> Self {
        let (navchar, item) = menu_str(txt);
        self.items.push(item);
        self.navchar.push(navchar);
        self
    }

    /// Fixed width for the menu.
    /// If not set it uses 1.5 times the length of the longest item.
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Placement relative to the render-area.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Take a style-set.
    pub fn styles(mut self, styles: MenuStyle) -> Self {
        self.style = styles.style;
        self.focus_style = styles.focus;
        self
    }

    /// Base style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Focus/Selection style.
    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = Some(style);
        self
    }

    /// Block for borders.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'a> StatefulWidget for PopupMenu<'a> {
    type State = PopupMenuState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.navchar = self.navchar.clone();

        self.layout(area, buf.area, state);

        Fill::new().style(self.style).render(state.area, buf);
        self.block.render_ref(state.area, buf);

        for (n, txt) in self.items.iter().enumerate() {
            let style = if state.selected == Some(n) {
                if let Some(focus) = self.focus_style {
                    focus
                } else {
                    Style::default().on_yellow()
                }
            } else {
                self.style
            };

            buf.set_style(state.item_areas[n], style);
            txt.render(state.item_areas[n], buf);
        }
    }
}

impl PopupMenuState {
    /// New
    pub fn new() -> Self {
        Self {
            area: Default::default(),
            z_area: Default::default(),
            item_areas: vec![],
            navchar: vec![],
            selected: None,
        }
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.item_areas.len()
    }

    /// Any items.
    pub fn is_empty(&self) -> bool {
        self.item_areas.is_empty()
    }

    /// Selected item.
    pub fn select(&mut self, select: Option<usize>) {
        self.selected = select;
    }

    /// Selected item.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Select the previous item.
    pub fn prev(&mut self) -> bool {
        let old = self.selected;

        self.selected = if let Some(selected) = self.selected {
            Some(selected.saturating_sub(1))
        } else {
            Some(self.len().saturating_sub(1))
        };

        old != self.selected
    }

    /// Select the next item.
    pub fn next(&mut self) -> bool {
        let old = self.selected;

        self.selected = if let Some(selected) = self.selected {
            Some(min(selected + 1, self.len().saturating_sub(1)))
        } else {
            Some(0)
        };

        old != self.selected
    }

    /// Select by navigation key.
    pub fn navigate(&mut self, c: char) -> MenuOutcome {
        for (i, cc) in self.navchar.iter().enumerate() {
            if *cc == Some(c) {
                if self.selected == Some(i) {
                    return MenuOutcome::Activated(i);
                } else {
                    self.selected = Some(i);
                    return MenuOutcome::Selected(i);
                }
            }
        }
        MenuOutcome::NotUsed
    }

    /// Select item at position
    pub fn select_at(&mut self, x: u16, y: u16) -> bool {
        if let Some(idx) = item_at_clicked(&self.item_areas, *x, *y) {
            self.selected = Some(idx);
            true
        } else {
            false
        }
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, MenuOutcome> for PopupMenuState {
    fn handle(&mut self, event: &Event, _qualifier: FocusKeys) -> MenuOutcome {
        match event {
            ct_event!(keycode press Up) => {
                if self.prev() {
                    MenuOutcome::Selected(self.selected.expect("selected"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            ct_event!(keycode press Down) => {
                if self.next() {
                    MenuOutcome::Selected(self.selected.expect("selected"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            ct_event!(keycode press Enter) => {
                if let Some(select) = self.selected {
                    MenuOutcome::Activated(select)
                } else {
                    MenuOutcome::NotUsed
                }
            }
            ct_event!(key press ANY-c) => self.navigate(*c),
            ct_event!(mouse moved for x,y) if self.area.contains((*x, *y).into()) => {
                if self.select_at(*x, *y) {
                    MenuOutcome::Selected(self.selected().expect("selection"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            ct_event!(mouse down Left for x,y) if self.area.contains((*x, *y).into()) => {
                if self.select_at(*x, *y) {
                    MenuOutcome::Activated(self.selected().expect("selection"))
                } else {
                    MenuOutcome::Unchanged
                }
            }
            _ => MenuOutcome::NotUsed,
        }
    }
}
