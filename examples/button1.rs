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
use rat_input::button::{Button, ButtonOutcome, ButtonState};
use rat_input::event::{HandleEvent, MouseOnly, Outcome};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::{Style, Stylize};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, StatefulWidget};
use ratatui::{Frame, Terminal};
use std::fs;
use std::io::{stdout, Stdout};
use std::time::Duration;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {
        p0: false,
        p1: false,
        p2: false,
    };

    let mut state = State {
        button1: Default::default(),
        button2: Default::default(),
        button3: Default::default(),
    };

    run_ui(&mut data, &mut state)
}

fn setup_logging() -> Result<(), anyhow::Error> {
    _ = fs::remove_file("log.log");
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}]\n",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file("log.log")?)
        .apply()?;
    Ok(())
}

struct Data {
    pub(crate) p0: bool,
    pub(crate) p1: bool,
    pub(crate) p2: bool,
}

struct State {
    pub(crate) button1: ButtonState,
    pub(crate) button2: ButtonState,
    pub(crate) button3: ButtonState,
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
    let area = frame.size();
    let buffer = frame.buffer_mut();

    repaint_buttons(area, buffer, data, state);
}

fn handle_event(
    event: crossterm::event::Event,
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

    let r = handle_buttons(&event, data, state)?;

    Ok(r)
}

fn repaint_buttons(area: Rect, buf: &mut Buffer, data: &mut Data, state: &mut State) {
    let l0 = Layout::horizontal([
        Constraint::Length(14),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .split(area);

    let l1 = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Fill(1),
    ])
    .split(l0[0]);

    let mut button1 = Button::from("Button");
    button1 = button1.block(Block::bordered().border_type(BorderType::Rounded));
    button1 = button1.style(Style::new().on_black().green());
    button1.render(l1[1], buf, &mut state.button1);

    let mut button2 = Button::from("Button\nnottuB");
    button2 = button2.block(Block::bordered().border_type(BorderType::Plain));
    button2 = button2.style(Style::new().on_black().blue());
    button2.render(l1[3], buf, &mut state.button2);

    let mut button3 = Button::from("Button").style(Style::new().white().on_red());
    button3 = button3.block(Block::bordered().border_type(BorderType::QuadrantInside));
    button3 = button3.style(Style::new().white().on_red());
    button3.render(l1[5], buf, &mut state.button3);

    let l2 = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Fill(1),
    ])
    .split(l0[1]);

    let label1 = Span::from(format!("=> {}", data.p0));
    label1.render(l2[1], buf);

    let label2 = Span::from(format!("=> {:?}", data.p1));
    label2.render(l2[3], buf);

    let label3 = if !data.p0 && !data.p1 && data.p2 {
        Span::from("of course")
    } else {
        Span::from(format!("=> {}", data.p2))
    };
    label3.render(l2[5], buf);
}

fn handle_buttons(
    event: &crossterm::event::Event,
    data: &mut Data,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    match HandleEvent::handle(&mut state.button1, event, MouseOnly) {
        ButtonOutcome::Pressed => {
            data.p0 = !data.p0;
            return Ok(Outcome::Changed);
        }
        ButtonOutcome::NotUsed => {}
        v => return Ok(v.into()),
    };
    match HandleEvent::handle(&mut state.button2, event, MouseOnly) {
        ButtonOutcome::Pressed => {
            data.p1 = !data.p1;
            return Ok(Outcome::Changed);
        }
        ButtonOutcome::NotUsed => {}
        v => return Ok(v.into()),
    };
    match HandleEvent::handle(&mut state.button3, event, MouseOnly) {
        ButtonOutcome::Pressed => {
            data.p2 = !data.p2;
            return Ok(Outcome::Changed);
        }
        ButtonOutcome::NotUsed => {}
        v => return Ok(v.into()),
    };

    Ok(Outcome::NotUsed)
}
