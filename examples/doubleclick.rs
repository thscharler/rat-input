use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use chrono::{Local, NaiveTime};
use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use format_num_pattern::NumberFormat;
use rat_event::ct_event;
use rat_event::util::MouseFlags;
use rat_input::event::Outcome;
use rat_input::layout::layout_grid;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::{Style, Stylize};
use ratatui::text::Span;
use ratatui::Frame;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {
        journal: Default::default(),
    };

    let mut state = State {
        area: Default::default(),
        mouse: Default::default(),
        flip: false,
        flip2: false,
        drag_pos: None,
    };

    run_ui(handle_buttons, repaint_buttons, &mut data, &mut state)
}

struct Data {
    pub(crate) journal: Vec<(NaiveTime, MouseEvent)>,
}

struct State {
    pub(crate) area: Rect,
    pub(crate) mouse: MouseFlags,
    pub(crate) flip: bool,
    pub(crate) flip2: bool,
    pub(crate) drag_pos: Option<(u16, u16)>,
}

fn repaint_buttons(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    let l = layout_grid::<4, 5>(
        area,
        Layout::horizontal([
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ]),
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Length(5),
            Constraint::Length(1),
        ]),
    );

    if state.flip2 {
        if state.flip {
            frame
                .buffer_mut()
                .set_style(l[1][2], Style::new().on_white());
        } else {
            frame.buffer_mut().set_style(l[1][2], Style::new().on_red());
        }
    } else {
        if state.flip {
            frame
                .buffer_mut()
                .set_style(l[1][2], Style::new().on_green());
        } else {
            frame
                .buffer_mut()
                .set_style(l[1][2], Style::new().on_blue());
        }
    }
    state.area = l[1][2];

    if state.mouse.drag.get() {
        if let Some((c, r)) = state.drag_pos {
            let numf = NumberFormat::new("###")?;
            let drag = Span::from(
                format!(
                    " DRAG: {}:{}",
                    numf.fmt_u(c as isize - state.area.x as isize),
                    numf.fmt_u(r as isize - state.area.y as isize)
                )
                .to_string(),
            );
            drag.render(l[3][2], frame.buffer_mut());
        }
    }

    if data.journal.len() > 0 {
        let numf = NumberFormat::new("##,###,###")?;

        let off = data.journal.len().saturating_sub(l[2][2].height as usize);
        let journal = &data.journal[off..];

        let zero = off.saturating_sub(1);
        let mut prev_time = data.journal[zero].0.clone();

        for (n, (time, event)) in journal.iter().enumerate() {
            let row_area = Rect::new(l[2][2].x, l[2][2].y + n as u16, l[2][2].width, 1);

            let dur = time.signed_duration_since(prev_time);

            let msg = Span::from(
                format!(
                    "{:>20} {:02}:{:02} {:25?}",
                    numf.fmt_u(dur.num_microseconds().expect("duration")),
                    event.column,
                    event.row,
                    event.kind
                )
                .to_string(),
            );
            msg.render(row_area, frame.buffer_mut());

            prev_time = time.clone();
        }
    }

    Ok(())
}

fn handle_buttons(
    event: &Event,
    data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    let r1 = match event {
        ct_event!(mouse any for m) if state.mouse.doubleclick(state.area, m) => {
            state.flip = !state.flip;
            Outcome::Changed
        }
        ct_event!(mouse any for m)
            if state
                .mouse
                .doubleclick2(state.area, m, KeyModifiers::CONTROL) =>
        {
            state.flip2 = !state.flip2;
            Outcome::Changed
        }
        ct_event!(mouse any for m) if state.mouse.drag(state.area, m) => {
            state.drag_pos = Some(state.mouse.pos_of(m));
            Outcome::Changed
        }
        _ => Outcome::NotUsed,
    };

    let r2 = match event {
        Event::Mouse(
            m @ MouseEvent {
                kind: MouseEventKind::Up(_) | MouseEventKind::Down(_) | MouseEventKind::Drag(_),
                ..
            },
        ) => {
            if state.area.contains((m.column, m.row).into()) {
                data.journal.push((Local::now().time(), m.clone()));
                Outcome::Changed
            } else {
                Outcome::Unchanged
            }
        }
        _ => Outcome::NotUsed,
    };

    Ok(r1 | r2)
}
