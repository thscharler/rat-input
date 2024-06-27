use std::cmp::max;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use crate::button::{Button, ButtonOutcome, ButtonState};
use crate::event::TextOutcome;
use rat_event::{ct_event, flow_ok, ConsumedEvent, FocusKeys, HandleEvent, MouseOnly, Outcome};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Margin, Rect};
use ratatui::prelude::{Alignment, BlockExt};
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::{Block, ListItem, StatefulWidget, Widget};

use crate::fill::Fill;
use crate::input::{TextInput, TextInputState};
use crate::layout_dialog::layout_dialog;
use crate::list::{List, ListState};
use crate::util::revert_style;

#[derive(Debug, Default, Clone)]
pub struct FileOpen<'a> {
    style: Style,
    dir_style: Option<Style>,
    file_style: Option<Style>,
    button_style: Option<Style>,

    block: Option<Block<'a>>,

    path_style: Option<Style>,
    select_style: Option<Style>,
    focus_style: Option<Style>,
}

#[derive(Debug, Default, Clone)]
pub struct FileOpenState {
    pub active: bool,
    pub focus: usize,

    pub chosen: Option<PathBuf>,

    path: PathBuf,
    dirs: Vec<OsString>,
    files: Vec<OsString>,

    path_state: TextInputState,
    dir_state: ListState,
    file_state: ListState,
    cancel_state: ButtonState,
    ok_state: ButtonState,
}

impl<'a> FileOpen<'a> {
    pub fn new() -> Self {
        Self {
            style: Default::default(),
            dir_style: None,
            file_style: None,
            button_style: None,
            block: None,
            path_style: Default::default(),
            select_style: None,
            focus_style: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn dir_style(mut self, style: Style) -> Self {
        self.dir_style = Some(style);
        self
    }

    pub fn file_style(mut self, style: Style) -> Self {
        self.file_style = Some(style);
        self
    }

    pub fn button_style(mut self, style: Style) -> Self {
        self.button_style = Some(style);
        self
    }

    pub fn path_style(mut self, style: Style) -> Self {
        self.path_style = Some(style);
        self
    }

    pub fn select_style(mut self, style: Style) -> Self {
        self.select_style = Some(style);
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = Some(style);
        self
    }
}

impl<'a> StatefulWidget for FileOpen<'a> {
    type State = FileOpenState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let layout = layout_dialog(
            area,
            Constraint::Fill(1),
            Constraint::Fill(1),
            Margin::new(1, 1),
            [Constraint::Length(10), Constraint::Length(10)],
            1,
            Flex::End,
        );

        let l_vert = Layout::new(
            Direction::Vertical,
            [Constraint::Length(1), Constraint::Fill(1)],
        )
        .split(layout.area);

        let l_hor = Layout::new(
            Direction::Horizontal,
            [Constraint::Percentage(39), Constraint::Percentage(61)],
        )
        .spacing(1)
        .split(l_vert[1]);

        //
        let select_style = if let Some(select) = self.select_style {
            select
        } else {
            revert_style(self.style)
        };
        let focus_style = if let Some(focus) = self.focus_style {
            focus
        } else {
            revert_style(self.style)
        };

        //
        let inner = if self.block.is_some() {
            let inner = self.block.inner_if_some(area);
            self.block.render(area, buf);
            inner
        } else {
            let block = Block::new().title(" Open ...").style(self.style);
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        };

        Fill::new().style(self.style).render(inner, buf);

        state.focus.to_string().render(layout.button_area, buf);

        state.path_state.focus.focus.set(state.focus == 4);
        let mut path = TextInput::new();
        if let Some(style) = self.path_style {
            path = path.style(style);
        }
        if let Some(style) = self.select_style {
            path = path.select_style(style);
        }
        if let Some(style) = self.focus_style {
            path = path.focus_style(style);
        } else {
            let style = revert_style(self.style);
            path = path.focus_style(style);
        }
        path.render(l_vert[0], buf, &mut state.path_state);

        let mut dir = List::default()
            .items(state.dirs.iter().map(|v| {
                let s = v.to_string_lossy();
                ListItem::from(s)
            }))
            .highlight_style(if state.focus == 0 {
                focus_style
            } else {
                select_style
            });
        if let Some(dir_style) = self.dir_style {
            dir = dir.style(dir_style);
        } else {
            dir = dir.block(Block::bordered());
        }
        StatefulWidget::render(dir, l_hor[0], buf, &mut state.dir_state);
        state.dir_state.area = l_hor[0];

        let mut file = List::default()
            .items(state.files.iter().map(|v| {
                let s = v.to_string_lossy();
                ListItem::from(s)
            }))
            .highlight_style(if state.focus == 1 {
                focus_style
            } else {
                select_style
            });
        if let Some(file_style) = self.file_style {
            file = file.style(file_style);
        } else {
            file = file.block(Block::bordered());
        }
        StatefulWidget::render(file, l_hor[1], buf, &mut state.file_state);
        state.file_state.area = l_hor[1];

        state.cancel_state.focus.set(state.focus == 3);
        let mut cancel = Button::new().text(Text::from("Cancel").alignment(Alignment::Center));
        if let Some(style) = self.button_style {
            cancel = cancel.style(style);
        }
        if let Some(style) = self.select_style {
            cancel = cancel.armed_style(style);
        }
        if let Some(style) = self.focus_style {
            cancel = cancel.focus_style(style);
        } else {
            let style = revert_style(self.style);
            cancel = cancel.focus_style(style);
        }
        cancel.render(layout.button(0), buf, &mut state.cancel_state);

        state.ok_state.focus.set(state.focus == 2);
        let mut ok = Button::new().text(Text::from("Ok").alignment(Alignment::Center));
        if let Some(style) = self.button_style {
            ok = ok.style(style);
        }
        if let Some(style) = self.select_style {
            ok = ok.armed_style(style);
        }
        if let Some(style) = self.focus_style {
            ok = ok.focus_style(style);
        } else {
            let style = revert_style(self.style);
            ok = ok.focus_style(style);
        }
        ok.render(layout.button(1), buf, &mut state.ok_state);
    }
}

impl FileOpenState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, path: &Path) -> Result<(), io::Error> {
        self.active = true;
        self.set_path(path.into())?;
        Ok(())
    }

    pub fn set_path(&mut self, path: &Path) -> Result<FileOutcome, io::Error> {
        let old = self.path.clone();
        let path = path.to_path_buf();

        if old != path {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for r in path.read_dir()? {
                let Ok(r) = r else {
                    continue;
                };

                if let Ok(meta) = r.metadata() {
                    if meta.is_dir() {
                        dirs.push(r.file_name());
                    } else if meta.is_file() {
                        files.push(r.file_name());
                    }
                }
            }

            self.path = path;
            self.dirs = dirs;
            self.files = files;
            self.path_state.set_value(self.path.to_string_lossy());

            if self.dirs.len() > 0 {
                self.dir_state.select(Some(0));
            } else {
                self.dir_state.select(None);
            }
            if self.files.len() > 0 {
                self.file_state.select(Some(0));
            } else {
                self.file_state.select(None);
            }

            Ok(FileOutcome::Changed)
        } else {
            Ok(FileOutcome::Unchanged)
        }
    }

    pub fn chdir(&mut self, dir: &OsString) -> Result<FileOutcome, io::Error> {
        self.set_path(&self.path.join(dir))
    }

    pub fn chdir_selected(&mut self) -> Result<FileOutcome, io::Error> {
        if let Some(select) = self.dir_state.selected {
            if let Some(dir) = self.dirs.get(select).cloned() {
                self.chdir(&dir)?;
                return Ok(FileOutcome::Changed);
            }
        }
        Ok(FileOutcome::Unchanged)
    }

    /// Cancel the dialog.
    pub fn close_cancel(&mut self) -> FileOutcome {
        self.active = false;
        self.chosen = None;
        FileOutcome::Cancel
    }

    /// Choose the selected and close the dialog.
    pub fn choose_selected(&mut self) -> FileOutcome {
        if let Some(select) = self.file_state.selected {
            if let Some(file) = self.files.get(select).cloned() {
                self.active = false;
                self.chosen = Some(self.path.join(file));
                return FileOutcome::Ok;
            }
        }
        FileOutcome::Unchanged
    }

    pub fn screen_cursor(&self) -> Option<(u16, u16)> {
        if self.focus == 4 {
            self.path_state.screen_cursor()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileOutcome {
    /// The given event has not been used at all.
    NotUsed,
    /// The event has been recognized, but the result was nil.
    /// Further processing for this event may stop.
    Unchanged,
    /// The event has been recognized and there is some change
    /// due to it.
    /// Further processing for this event may stop.
    /// Rendering the ui is advised.
    Changed,
    /// Ok
    Ok,
    /// Cancel
    Cancel,
}

impl FileOutcome {
    pub fn then_max(self, other: Self) -> Self {
        if self.is_consumed() {
            max(self, other)
        } else {
            FileOutcome::NotUsed
        }
    }
}

impl From<FileOutcome> for Outcome {
    fn from(value: FileOutcome) -> Self {
        match value {
            FileOutcome::NotUsed => Outcome::NotUsed,
            FileOutcome::Unchanged => Outcome::Unchanged,
            FileOutcome::Changed => Outcome::Changed,
            FileOutcome::Ok => Outcome::Changed,
            FileOutcome::Cancel => Outcome::Changed,
        }
    }
}

impl From<Outcome> for FileOutcome {
    fn from(value: Outcome) -> Self {
        match value {
            Outcome::NotUsed => FileOutcome::NotUsed,
            Outcome::Unchanged => FileOutcome::Unchanged,
            Outcome::Changed => FileOutcome::Changed,
        }
    }
}

impl From<TextOutcome> for FileOutcome {
    fn from(value: TextOutcome) -> Self {
        match value {
            TextOutcome::NotUsed => FileOutcome::NotUsed,
            TextOutcome::Unchanged => FileOutcome::Unchanged,
            TextOutcome::Changed => FileOutcome::Changed,
            TextOutcome::TextChanged => FileOutcome::Changed,
        }
    }
}

impl From<ButtonOutcome> for FileOutcome {
    fn from(value: ButtonOutcome) -> Self {
        match value {
            ButtonOutcome::NotUsed => FileOutcome::NotUsed,
            ButtonOutcome::Unchanged => FileOutcome::Unchanged,
            ButtonOutcome::Changed => FileOutcome::Changed,
            ButtonOutcome::Pressed => FileOutcome::Changed,
        }
    }
}

impl ConsumedEvent for FileOutcome {
    fn is_consumed(&self) -> bool {
        !matches!(self, FileOutcome::NotUsed)
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, Result<FileOutcome, io::Error>>
    for FileOpenState
{
    fn handle(
        &mut self,
        event: &crossterm::event::Event,
        _qualifier: FocusKeys,
    ) -> Result<FileOutcome, io::Error> {
        if !self.active {
            return Ok(FileOutcome::NotUsed);
        }

        let old_focus = self.focus;

        let mut focus_outcome = match event {
            ct_event!(keycode press Tab) => {
                self.focus = (self.focus + 1) % 5;
                FileOutcome::Changed
            }
            ct_event!(keycode press SHIFT-BackTab) => {
                self.focus = if self.focus > 0 { self.focus - 1 } else { 4 };
                FileOutcome::Changed
            }
            ct_event!(mouse down Left for col,row) => {
                if self.path_state.area.contains((*col, *row).into()) {
                    self.focus = 4;
                } else if self.cancel_state.area.contains((*col, *row).into()) {
                    self.focus = 3;
                } else if self.ok_state.area.contains((*col, *row).into()) {
                    self.focus = 2;
                } else if self.file_state.area.contains((*col, *row).into()) {
                    self.focus = 1;
                } else if self.dir_state.area.contains((*col, *row).into()) {
                    self.focus = 0;
                }
                FileOutcome::Changed
            }
            _ => FileOutcome::NotUsed,
        };

        if old_focus == 4 && self.focus != 4 {
            self.set_path(&PathBuf::from(self.path_state.value()))?;
            focus_outcome = FileOutcome::Changed;
        }

        if self.focus == 4 {
            let path_changed = match event {
                ct_event!(keycode press Enter) => {
                    self.set_path(&PathBuf::from(self.path_state.value()))?;
                    FileOutcome::Changed
                }
                _ => self.path_state.handle(event, FocusKeys).into(),
            };
            if path_changed.is_consumed() {
                return Ok(max(path_changed, focus_outcome));
            }
        } else {
            flow_ok!(self.path_state.handle(event, MouseOnly));
        };

        let cancel_outcome = if self.focus == 3 {
            self.cancel_state.handle(event, FocusKeys)
        } else {
            match event {
                ct_event!(keycode press Esc) => ButtonOutcome::Pressed,
                _ => self.cancel_state.handle(event, MouseOnly),
            }
        };
        flow_ok!(match cancel_outcome {
            ButtonOutcome::Pressed => {
                self.close_cancel()
            }
            r => r.into(),
        });

        let ok_outcome = if self.focus == 2 {
            self.ok_state.handle(event, FocusKeys)
        } else {
            self.ok_state.handle(event, MouseOnly)
        };
        flow_ok!(match ok_outcome {
            ButtonOutcome::Pressed => {
                self.choose_selected()
            }
            r => r.into(),
        });

        flow_ok!(match event {
            ct_event!(mouse any for m)
                if self.file_state.mouse.doubleclick(self.file_state.area, m) =>
            {
                self.choose_selected()
            }
            _ => FileOutcome::NotUsed,
        });
        let file_changed = if self.focus == 1 {
            match event {
                ct_event!(keycode press Enter) => self.choose_selected(),
                _ => self.file_state.handle(event, FocusKeys).into(),
            }
        } else {
            self.file_state.handle(event, MouseOnly).into()
        };
        if file_changed.is_consumed() {
            return Ok(max(file_changed, focus_outcome));
        }

        flow_ok!(match event {
            ct_event!(mouse any for m)
                if self.dir_state.mouse.doubleclick(self.dir_state.area, m) =>
            {
                self.chdir_selected()?
            }
            _ => FileOutcome::NotUsed,
        });
        let dir_changed = if self.focus == 0 {
            match event {
                ct_event!(keycode press Enter) => self.chdir_selected()?,
                _ => self.dir_state.handle(event, FocusKeys).into(),
            }
        } else {
            self.dir_state.handle(event, MouseOnly).into()
        };
        if dir_changed.is_consumed() {
            return Ok(max(dir_changed, focus_outcome));
        }

        Ok(focus_outcome)
    }
}
