//!
//! A menubar widget with sub-menus.
//!
//! Combines [MenuLine] and [PopupMenu] and adds a [MenuStructure] trait
//! to bind all together.
//!
//! Rendering is split in two widgets [MenuBar] and [MenuPopup].
//! This should help with front/back rendering.
//!
//! Event-handling for the popup menu is split via the [Popup] qualifier.
//! All `Popup` event-handling should be called before the regular
//! `FocusKeys` handling.
//!
use crate::event::Popup;
use crate::menuline::{MenuLine, MenuLineState, MenuOutcome, MenuStyle};
use crate::popup_menu::{Placement, PopupMenu, PopupMenuState};
use crate::util::menu_str;
use rat_event::{flow, FocusKeys, HandleEvent, MouseOnly};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Line, StatefulWidget, Style};
use ratatui::widgets::{Block, StatefulWidgetRef};
use std::fmt::{Debug, Formatter};

/// Trait for the structural data of the MenuBar.
pub trait MenuStructure<'a> {
    /// Main menu.
    fn menus(&'a self) -> Vec<(Line<'a>, Option<char>)>;
    /// Submenus.
    fn submenu(&'a self, n: usize) -> Vec<(Line<'a>, Option<char>)>;
}

/// Static menu structure.
#[derive(Debug)]
pub struct StaticMenu {
    pub menu: &'static [(&'static str, &'static [&'static str])],
}

impl MenuStructure<'static> for StaticMenu {
    fn menus(&'static self) -> Vec<(Line<'static>, Option<char>)> {
        self.menu.iter().map(|v| menu_str(v.0)).collect()
    }

    fn submenu(&'static self, n: usize) -> Vec<(Line<'static>, Option<char>)> {
        self.menu[n].1.iter().map(|v| menu_str(v)).collect()
    }
}

/// MenuBar widget.
///
/// This is only half of the widget. For popup rendering there is the separate
/// [MenuPopup].
#[derive(Debug, Default, Clone)]
pub struct MenuBar<'a> {
    menu: MenuLine<'a>,
}

/// Menubar widget.
///
/// Separate renderer for the popup part of the menubar.
#[derive(Default, Clone)]
pub struct MenuPopup<'a> {
    structure: Option<&'a dyn MenuStructure<'a>>,
    popup: PopupMenu<'a>,
}

/// State for the menubar.
#[derive(Debug, Default, Clone)]
pub struct MenuBarState {
    /// State for the menu.
    pub menu: MenuLineState,
    /// Popups visible?
    pub popup_active: bool,
    /// State for the last rendered popup menu.
    pub popup: PopupMenuState,
}

impl<'a> MenuBar<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Title text.
    #[inline]
    pub fn title(mut self, title: impl Into<Line<'a>>) -> Self {
        self.menu = self.menu.title(title);
        self
    }

    /// Menu-Structure
    pub fn menu(mut self, structure: &'a dyn MenuStructure<'a>) -> Self {
        for (m, n) in structure.menus() {
            self.menu = self.menu.add(m, n);
        }
        self
    }

    /// Combined style.
    #[inline]
    pub fn styles(mut self, styles: MenuStyle) -> Self {
        self.menu = self.menu.styles(styles.clone());
        self
    }

    /// Base style.
    #[inline]
    pub fn style(mut self, style: Style) -> Self {
        self.menu = self.menu.style(style);
        self
    }

    /// Menu-title style.
    #[inline]
    pub fn title_style(mut self, style: Style) -> Self {
        self.menu = self.menu.title_style(style);
        self
    }

    /// Selection
    #[inline]
    pub fn select_style(mut self, style: Style) -> Self {
        self.menu = self.menu.select_style(style);
        self
    }

    /// Selection + Focus
    #[inline]
    pub fn focus_style(mut self, style: Style) -> Self {
        self.menu = self.menu.focus_style(style);
        self
    }
}

impl<'a> Debug for MenuPopup<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuPopup")
            .field("popup", &self.popup)
            .finish()
    }
}

impl<'a> MenuPopup<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Menu.
    pub fn menu(mut self, structure: &'a dyn MenuStructure<'a>) -> Self {
        self.structure = Some(structure);
        self
    }

    /// Fixed width for the menu.
    /// If not set it uses 1.5 times the length of the longest item.
    pub fn width(mut self, width: u16) -> Self {
        self.popup = self.popup.width(width);
        self
    }

    /// Placement relative to the render-area.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.popup = self.popup.placement(placement);
        self
    }

    /// Combined style.
    #[inline]
    pub fn styles(mut self, styles: MenuStyle) -> Self {
        self.popup = self.popup.styles(styles.clone());
        self
    }

    /// Base style.
    pub fn style(mut self, style: Style) -> Self {
        self.popup = self.popup.style(style);
        self
    }

    /// Focus/Selection style.
    pub fn focus_style(mut self, style: Style) -> Self {
        self.popup = self.popup.focus_style(style);
        self
    }

    /// Block for borders.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.popup = self.popup.block(block);
        self
    }
}

impl<'a> StatefulWidgetRef for MenuBar<'a> {
    type State = MenuBarState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.menu.render_ref(area, buf, &mut state.menu);
    }
}

impl<'a> StatefulWidget for MenuBar<'a> {
    type State = MenuBarState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.menu.render(area, buf, &mut state.menu);
    }
}

impl<'a> StatefulWidgetRef for MenuPopup<'a> {
    type State = MenuBarState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_menu_popup(self, area, buf, state);
    }
}

impl<'a> StatefulWidget for MenuPopup<'a> {
    type State = MenuBarState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_menu_popup(&self, area, buf, state);
    }
}

fn render_menu_popup(
    widget: &MenuPopup<'_>,
    _area: Rect,
    buf: &mut Buffer,
    state: &mut MenuBarState,
) {
    if state.popup_active {
        if let Some(selected) = state.menu.selected {
            if let Some(structure) = widget.structure {
                let mut len = 0;
                let mut popup = widget.popup.clone(); // TODO:???
                for (item, navchar) in structure.submenu(selected) {
                    popup = popup.add(item, navchar);
                    len += 1;
                }

                if len > 0 {
                    let area = state.menu.item_areas[selected];
                    popup.render(area, buf, &mut state.popup);
                }
            } else {
                // no menu structure? ok.
                state.popup = Default::default();
            }
        } else {
            // no selection. ok.
            state.popup = Default::default();
        }
    } else {
        state.popup = Default::default();
    }
}

impl MenuBarState {
    /// State.
    /// For the specifics use the public fields `menu` and `popup`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Submenu visible/active.
    pub fn popup_active(&self) -> bool {
        self.popup_active
    }

    /// Submenu visible/active.
    pub fn set_popup_active(&mut self, active: bool) {
        self.popup_active = active;
    }
}

impl HandleEvent<crossterm::event::Event, Popup, MenuOutcome> for MenuBarState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Popup) -> MenuOutcome {
        if let Some(selected) = self.menu.selected {
            if self.popup_active {
                match self.popup.handle(event, Popup) {
                    MenuOutcome::Selected(n) => MenuOutcome::MenuSelected(selected, n),
                    MenuOutcome::Activated(n) => MenuOutcome::MenuActivated(selected, n),
                    r => r,
                }
            } else {
                MenuOutcome::NotUsed
            }
        } else {
            MenuOutcome::NotUsed
        }
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, MenuOutcome> for MenuBarState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: FocusKeys) -> MenuOutcome {
        if self.menu.is_focused() {
            match self.menu.handle(event, FocusKeys) {
                r @ MenuOutcome::Selected(_) => {
                    self.popup_active = !self.popup_active;
                    r
                }
                r => r,
            }
        } else {
            self.menu.handle(event, MouseOnly)
        }
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, MenuOutcome> for MenuBarState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: MouseOnly) -> MenuOutcome {
        flow!(if let Some(selected) = self.menu.selected {
            if self.popup_active {
                match self.popup.handle(event, MouseOnly) {
                    MenuOutcome::Selected(n) => MenuOutcome::MenuSelected(selected, n),
                    MenuOutcome::Activated(n) => MenuOutcome::MenuActivated(selected, n),
                    r => r,
                }
            } else {
                MenuOutcome::NotUsed
            }
        } else {
            MenuOutcome::NotUsed
        });

        flow!(match self.menu.handle(event, MouseOnly) {
            r @ MenuOutcome::Selected(_) => {
                self.popup_active = !self.popup_active;
                r
            }
            r => r,
        });

        MenuOutcome::NotUsed
    }
}
