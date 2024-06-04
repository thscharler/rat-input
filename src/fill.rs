use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Fill the area with a grapheme and a style.
/// Useful when overwriting an already rendered buffer
/// for overlays or windows.
#[derive(Debug)]
pub struct Fill<'a> {
    c: &'a str,
    style: Style,
}

impl<'a> Default for Fill<'a> {
    fn default() -> Self {
        Self {
            c: " ",
            style: Default::default(),
        }
    }
}

impl<'a> Fill<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the fill char as one graphem.
    pub fn fill_char(mut self, c: &'a str) -> Self {
        self.c = c;
        self
    }

    /// Set the fill style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> Widget for Fill<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let area = buf.area.intersection(area);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = buf.get_mut(x, y);
                cell.set_symbol(self.c);
                cell.set_style(self.style);
            }
        }
    }
}
