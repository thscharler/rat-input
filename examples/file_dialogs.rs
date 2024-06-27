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
use rat_event::{flow_ok, Dialog, Popup};
use rat_input::button::ButtonStyle;
use rat_input::event::{FocusKeys, HandleEvent, Outcome};
use rat_input::file_dialog::{FileOpen, FileOpenState, FileOutcome};
use rat_input::layout_grid::layout_middle;
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
use std::path::Path;
use std::time::{Duration, SystemTime};

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        menu: Default::default(),
        file_open: Default::default(),
        status: Default::default(),
        msg: Default::default(),
    };
    state.menu.menu.set_focused(true);

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
    pub(crate) menu: MenuBarState,
    pub(crate) file_open: FileOpenState,
    pub(crate) status: StatusLineState,
    pub(crate) msg: MsgDialogState,
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
        repaint_tui(frame, data, state);
    });

    Ok(())
}

fn repaint_tui(frame: &mut Frame<'_>, data: &mut Data, state: &mut State) {
    let t0 = SystemTime::now();
    let area = frame.size();

    let l1 = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(area);

    repaint_input(frame, l1[0], data, state);

    let status1 = StatusLine::new()
        .layout([
            Constraint::Fill(1),
            Constraint::Length(17),
            Constraint::Length(17),
        ])
        .styles([
            Style::default().black().on_dark_gray(),
            Style::default().white().on_blue(),
            Style::default().white().on_light_blue(),
        ]);

    if state.msg.active {
        let msgd = MsgDialog::default()
            .style(Style::default().white().on_blue())
            .button_style(ButtonStyle {
                style: Style::default().blue().on_white(),
                ..Default::default()
            });
        frame.render_stateful_widget(msgd, area, &mut state.msg);
    }

    let el = t0.elapsed().unwrap_or(Duration::from_nanos(0));
    state
        .status
        .status(1, format!("Render {:?}", el).to_string());
    frame.render_stateful_widget(status1, l1[1], &mut state.status);
}

fn handle_event(
    event: crossterm::event::Event,
    data: &mut Data,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    let t0 = SystemTime::now();

    let r = 'h: {
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

        if state.msg.active {
            let r = state.msg.handle(&event, FocusKeys);
            break 'h r;
        }

        let r = handle_input(&event, data, state)?;

        r
    };

    let el = t0.elapsed().unwrap_or(Duration::from_nanos(0));
    state
        .status
        .status(2, format!("Handle {:?}", el).to_string());

    Ok(r)
}

static MENU: StaticMenu = StaticMenu {
    menu: &[
        ("File", &["Open"]), //
        ("Quit", &[]),
    ],
};

fn repaint_input(frame: &mut Frame<'_>, area: Rect, _data: &mut Data, state: &mut State) {
    let l1 = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(area);

    MenuBar::new()
        .title("File|Dialogs")
        .menu(&MENU)
        .title_style(Style::default().black().on_yellow())
        .style(Style::default().black().on_dark_gray())
        .focus_style(Style::default().black().on_cyan())
        .render(l1[1], frame.buffer_mut(), &mut state.menu);

    if state.file_open.active {
        let l2 = layout_middle(
            l1[0],
            Constraint::Length(state.menu.menu.item_areas[0].x.saturating_sub(2)),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Length(0),
        );

        FileOpen::new()
            .style(Style::default().black().on_dark_gray())
            .select_style(Style::default().on_cyan())
            .focus_style(Style::default().on_yellow())
            .dir_style(Style::default().on_gray())
            .file_style(Style::default().on_gray())
            .button_style(Style::default().black().on_cyan())
            // .block(
            //     Block::bordered()
            //         .title("Open sesame ...")
            //         .style(Style::default().black().on_gray()),
            // )
            .render(l2, frame.buffer_mut(), &mut state.file_open);
        if let Some(c) = state.file_open.screen_cursor() {
            frame.set_cursor(c.0, c.1);
        }
    }

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
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    flow_ok!(match state.menu.handle(event, Popup) {
        MenuOutcome::MenuActivated(0, 0) => {
            state.menu.set_popup_active(false);
            state.file_open.open(&Path::new("."))?;
            Outcome::Changed
        }
        r => r.into(),
    });

    flow_ok!(match state.file_open.handle(event, Dialog)? {
        FileOutcome::Ok => {
            state
                .status
                .status(0, format!("File selected: {:?}", state.file_open.chosen));
            Outcome::Changed
        }
        FileOutcome::Cancel => {
            state.status.status(0, "File selection cancelled!");
            Outcome::Changed
        }
        r => r.into(),
    });

    flow_ok!(match state.menu.handle(event, FocusKeys) {
        MenuOutcome::Activated(1) => {
            return Err(anyhow!("quit"));
        }
        r => Outcome::from(r),
    });

    Ok(Outcome::NotUsed)
}
