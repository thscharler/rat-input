use anyhow::anyhow;
use crossterm::cursor::{DisableBlinking, EnableBlinking, SetCursorStyle};
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, Event,
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use log::debug;
use rat_event::{ct_event, ConsumedEvent, HandleEvent};
use rat_input::event::{Outcome, Popup};
use rat_input::layout_grid::layout_grid;
use rat_input::menuline::MenuOutcome;
use rat_input::popup_menu::{Placement, PopupMenu, PopupMenuState};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, StatefulWidget};
use ratatui::{Frame, Terminal};
use std::fs;
use std::io::{stdout, Stdout};
use std::time::Duration;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        area: Default::default(),
        left: Default::default(),
        right: Default::default(),
        placement: Placement::Top,
        popup_active: false,
        popup_area: Default::default(),
        popup: PopupMenuState::default(),
    };

    run_ui(&mut data, &mut state)
}

fn setup_logging() -> Result<(), anyhow::Error> {
    _ = fs::remove_file("log.log");
    fern::Dispatch::new()
        .format(|out, message, _record| out.finish(format_args!("{}", message)))
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file("log.log")?)
        .apply()?;
    Ok(())
}

struct Data {}

struct State {
    pub(crate) area: Rect,
    pub(crate) left: Rect,
    pub(crate) right: Rect,

    pub(crate) placement: Placement,
    pub(crate) popup_active: bool,
    pub(crate) popup_area: Rect,
    pub(crate) popup: PopupMenuState,
}

fn run_ui(data: &mut Data, state: &mut State) -> Result<(), anyhow::Error> {
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(EnableMouseCapture)?;
    stdout().execute(EnableBlinking)?;
    stdout().execute(SetCursorStyle::BlinkingBar)?;
    stdout().execute(EnableBracketedPaste)?;
    enable_raw_mode()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    repaint_ui(&mut terminal, data, state)?;

    let r = 'l: loop {
        let o = match crossterm::event::poll(Duration::from_millis(10)) {
            Ok(true) => {
                let event = match crossterm::event::read() {
                    Ok(v) => v,
                    Err(e) => break 'l Err(anyhow!(e)),
                };
                match handle_event(event, data, state) {
                    Ok(v) => v,
                    Err(e) => break 'l Err(e),
                }
            }
            Ok(false) => continue,
            Err(e) => break 'l Err(anyhow!(e)),
        };

        match o {
            Outcome::Changed => {
                match repaint_ui(&mut terminal, data, state) {
                    Ok(_) => {}
                    Err(e) => break 'l Err(e),
                };
            }
            _ => {
                // noop
            }
        }
    };

    disable_raw_mode()?;
    stdout().execute(DisableBracketedPaste)?;
    stdout().execute(SetCursorStyle::DefaultUserShape)?;
    stdout().execute(DisableBlinking)?;
    stdout().execute(DisableMouseCapture)?;
    stdout().execute(LeaveAlternateScreen)?;

    r
}

fn repaint_ui(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    data: &mut Data,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    terminal.hide_cursor()?;

    _ = terminal.draw(|frame| {
        debug!("{:?}", repaint_tui(frame, data, state));
    });

    Ok(())
}

fn repaint_tui(
    frame: &mut Frame<'_>,
    data: &mut Data,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    let area = frame.size();
    let buffer = frame.buffer_mut();

    repaint_stuff(area, buffer, data, state)?;
    Ok(())
}

fn handle_event(
    event: Event,
    data: &mut Data,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    use crossterm::event::Event;
    match event {
        Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            ..
        }) => {
            return Err(anyhow!("quit"));
        }
        Event::Resize(_, _) => return Ok(Outcome::Changed),
        _ => {}
    }

    let r = handle_stuff(&event, data, state)?;

    Ok(r)
}

fn repaint_stuff(
    area: Rect,
    buf: &mut Buffer,
    _data: &mut Data,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    let l = layout_grid::<4, 3>(
        area,
        Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(3),
        ]),
        Layout::vertical([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ]),
    );

    state.area = l[1][1];
    state.left = l[0][0].union(l[2][2]);
    state.right = l[3][0].union(l[3][2]);

    buf.set_style(l[1][1], Style::new().on_blue());
    buf.set_style(l[3][0].union(l[3][2]), Style::new().on_dark_gray());

    if state.popup_active {
        PopupMenu::new()
            .style(Style::new().black().on_cyan())
            .block(Block::bordered().title("Nice popup"))
            .placement(state.placement)
            .add_str("Item _1")
            .add_str("Item _2")
            .add_str("Item _3")
            .add_str("Item _4")
            .render(state.popup_area, buf, &mut state.popup);
    }

    Ok(())
}

fn handle_stuff(
    event: &Event,
    _data: &mut Data,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    let r1 = if state.popup_active {
        match state.popup.handle(event, Popup) {
            MenuOutcome::Selected(_) => Outcome::Changed,
            MenuOutcome::Activated(_) => {
                state.popup_active = false;
                Outcome::Changed
            }
            r => r.into(),
        }
    } else {
        Outcome::NotUsed
    };
    if r1.is_consumed() {
        return Ok(r1);
    }

    let r2 = match event {
        ct_event!(mouse down Left for x,y) if state.left.contains((*x, *y).into()) => {
            state.popup_area = state.area;
            state.popup_active = true;

            if *x < state.area.left() {
                state.placement = Placement::Left;
            } else if *x >= state.area.right() {
                state.placement = Placement::Right;
            } else if *y < state.area.top() {
                state.placement = Placement::Top;
            } else if *y >= state.area.bottom() {
                state.placement = Placement::Bottom;
            }
            Outcome::Changed
        }
        ct_event!(mouse down Left for x,y) if state.right.contains((*x, *y).into()) => {
            state.popup_area = Rect::new(*x, *y, 0, 0);
            state.popup_active = true;
            state.placement = Placement::Right;
            Outcome::Changed
        }
        _ => Outcome::NotUsed,
    };
    Ok(r2)
}
