use ratatui::layout::{Constraint, Direction, Flex, Layout, Margin, Rect};

/// Layout produced by [layout_dialog]
#[derive(Debug)]
pub struct LayoutDialog<const N: usize> {
    pub dialog: Rect,
    pub area: Rect,
    pub button_area: Rect,
    pub buttons: [Rect; N],
}

impl<const N: usize> LayoutDialog<N> {
    pub fn button(&self, n: usize) -> Rect {
        self.buttons[n]
    }
}

/// Calculates a layout for a dialog with buttons.
pub fn layout_dialog<const N: usize>(
    area: Rect,
    width: Constraint,
    height: Constraint,
    insets: Margin,
    buttons: [Constraint; N],
    button_spacing: u16,
    button_flex: Flex,
) -> LayoutDialog<N> {
    let l_vertical = Layout::new(
        Direction::Vertical,
        [Constraint::Fill(1), width, Constraint::Fill(1)],
    )
    .split(area);
    let l_dialog = Layout::new(
        Direction::Horizontal,
        [Constraint::Fill(1), height, Constraint::Fill(1)],
    )
    .split(l_vertical[1])[1];

    let l_inner = l_dialog.inner(&insets);

    let l_content = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(l_inner);

    let l_buttons = Layout::horizontal(buttons)
        .spacing(button_spacing)
        .flex(button_flex)
        .areas(l_content[2]);

    LayoutDialog {
        dialog: l_dialog,
        area: l_content[0],
        button_area: l_content[2],
        buttons: l_buttons,
    }
}