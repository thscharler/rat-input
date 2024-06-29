use crate::mini_salsa::MiniSalsaState;
use anyhow::anyhow;
use rat_event::{flow_ok, Outcome};
use rat_input::menubar;
use rat_input::menubar::{MenuBar, MenuBarState, MenuPopup, StaticMenu};
use rat_input::menuline::MenuOutcome;
use rat_input::popup_menu::Placement;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, StatefulWidget};
use ratatui::Frame;

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
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<(), anyhow::Error> {
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

    Ok(())
}

fn handle_input(
    event: &crossterm::event::Event,
    _data: &mut Data,
    istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    flow_ok!(
        match menubar::handle_popup_events(&mut state.menu, true, event) {
            MenuOutcome::MenuSelected(v, w) => {
                istate.status.status(0, format!("Selected {}-{}", v, w));
                Outcome::Changed
            }
            MenuOutcome::MenuActivated(v, w) => {
                istate.status.status(0, format!("Activated {}-{}", v, w));
                state.menu.set_popup_active(false);
                Outcome::Changed
            }
            r => r.into(),
        }
    );

    flow_ok!(match menubar::handle_events(&mut state.menu, true, event) {
        MenuOutcome::Selected(v) => {
            istate.status.status(0, format!("Selected {}", v));
            Outcome::Changed
        }
        MenuOutcome::Activated(v) => {
            istate.status.status(0, format!("Activated {}", v));
            match v {
                3 => return Err(anyhow!("Quit")),
                _ => {}
            }
            Outcome::Changed
        }
        r => {
            r.into()
        }
    });

    Ok(Outcome::NotUsed)
}
