use crate::mini_salsa::InternState;
use anyhow::anyhow;
use crossterm::cursor::{DisableBlinking, EnableBlinking, SetCursorStyle};
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, KeyCode,
    KeyEvent, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use log::debug;
use rat_event::Popup;
use rat_input::button::ButtonStyle;
use rat_input::event::{FocusKeys, HandleEvent, Outcome};
use rat_input::menubar::{MenuBar, MenuBarState, MenuPopup, StaticMenu};
use rat_input::menuline::MenuOutcome;
use rat_input::msgdialog::{MsgDialog, MsgDialogState};
use rat_input::popup_menu::Placement;
use rat_input::statusline::{StatusLine, StatusLineState};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, StatefulWidget};
use ratatui::{Frame, Terminal};
use std::fs;
use std::io::{stdout, Stdout};
use std::time::{Duration, SystemTime};

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    mini_salsa::setup_logging()?;

    let mut data = Data::default();
    let mut state = State::default();
    state.menu.menu.focus.set(true);

    mini_salsa::run_ui(handle_input, repaint_input, &mut data, &mut state)
}

#[derive(Default)]
struct Data {}

#[derive(Default)]
struct State {
    pub(crate) menu: MenuBarState,
}

static MENU: StaticMenu = StaticMenu {
    menu: &[
        ("Alpha", &["One", "Two", "Three"]),
        ("Beta", &["Ex", "Why", "Sed"]),
        ("Gamma", &["No", "more", "ideas"]),
        ("Quit", &[]),
    ],
};

fn repaint_input(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &mut Data,
    _istate: &mut InternState,
    state: &mut State,
) {
    let l1 = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(area);

    MenuBar::new()
        .title("Sample")
        .menu(&MENU)
        .title_style(Style::default().black().on_yellow())
        .style(Style::default().black().on_dark_gray())
        .focus_style(Style::default().black().on_cyan())
        .render(l1[1], frame.buffer_mut(), &mut state.menu);

    // todo: render something for the background ...

    MenuPopup::new()
        .menu(&MENU)
        .block(Block::bordered())
        .width(15)
        .style(Style::default().black().on_dark_gray())
        .focus_style(Style::default().black().on_cyan())
        .placement(Placement::Top)
        .render(l1[1], frame.buffer_mut(), &mut state.menu);
}

fn handle_input(
    event: &crossterm::event::Event,
    _data: &mut Data,
    istate: &mut InternState,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    let r = HandleEvent::handle(&mut state.menu, event, Popup);
    debug!("{:?}", r);
    match r {
        MenuOutcome::MenuSelected(v, w) => {
            istate.status.status(0, format!("Selected {}-{}", v, w));
        }
        MenuOutcome::MenuActivated(v, w) => {
            istate.status.status(0, format!("Activated {}-{}", v, w));
            state.menu.set_popup_active(false);
        }
        _ => {}
    };

    let s = HandleEvent::handle(&mut state.menu, event, FocusKeys);
    debug!("{:?}", s);
    match s {
        MenuOutcome::Selected(v) => {
            istate.status.status(0, format!("Selected {}", v));
        }
        MenuOutcome::Activated(v) => {
            istate.status.status(0, format!("Activated {}", v));
            match v {
                3 => return Err(anyhow!("Quit")),
                _ => {}
            }
        }
        _ => {}
    };

    Ok(Outcome::from(s) | Outcome::from(r))
}
