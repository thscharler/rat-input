use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::StatefulWidgetRef;

#[derive(Default)]
struct Button<'a> {
    text: Text<'a>,
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

#[derive(Default)]
struct Checkbox<'a> {
    text: Text<'a>,
}

struct CheckboxState {
    pub checked: bool,
}

impl<'a> StatefulWidgetRef for Checkbox<'a> {
    type State = CheckboxState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // ...
    }
}

pub fn test_dyn() {
    let w0: Box<dyn StatefulWidgetRef<State = ButtonState>> = Box::new(Button::default());
    let w1: Box<dyn StatefulWidgetRef<State = CheckboxState>> = Box::new(Checkbox::default());

    // fail
    let vec = vec![w0, w1];
}
