use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Styled, Stylize};
use ratatui::text::Text;
use ratatui::widgets::StatefulWidgetRef;

#[derive(Default, Clone)]
struct Button<'a> {
    text: Text<'a>,
    style: Style,
}

struct ButtonState {
    pub armed: bool,
}

impl<'a> StatefulWidgetRef for Button<'a> {
    type State = ButtonState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // ...
    }
}

#[derive(Default, Clone)]
struct FocusableButton<'a> {
    widget: Button<'a>,
}

struct FocusableButtonState {
    pub widget: ButtonState,
    pub focus: bool,
}

impl<'a> StatefulWidgetRef for FocusableButton<'a> {
    type State = FocusableButtonState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let w = if state.focus {
            let mut w = self.widget.clone();
            w.style = w.style.on_light_blue();
            w
        } else {
            self.widget.clone()
        };
        w.render_ref(area, buf, &mut state.widget)
    }
}
