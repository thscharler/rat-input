use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use std::iter::once;
use std::ops::Range;
use std::time::{Duration, SystemTime};
use unicode_segmentation::UnicodeSegmentation;

// clear, set new style
pub(crate) fn clear_area(buf: &mut Buffer, area: Rect, style: Style) {
    let area = buf.area.intersection(area);
    for x in area.left()..area.right() {
        for y in area.top()..area.bottom() {
            buf.get_mut(x, y).reset();
            buf.get_mut(x, y).set_style(style);
        }
    }
}

/// Constrains the range to the visible range and shifts the result by offset.
pub(crate) fn clamp_shift(range: Range<usize>, offset: usize, width: usize) -> Range<usize> {
    let start = if range.start < offset {
        offset
    } else {
        range.start
    };
    let end = if range.end > offset + width {
        offset + width
    } else {
        range.end
    };

    start - offset..end - offset
}

/// Split off selection
pub(crate) fn split3(value: &str, selection: Range<usize>) -> (&str, &str, &str) {
    let mut byte_selection_start = None;
    let mut byte_selection_end = None;

    for (cidx, (idx, _c)) in value
        .grapheme_indices(true)
        .chain(once((value.len(), "")))
        .enumerate()
    {
        if cidx == selection.start {
            byte_selection_start = Some(idx);
        }
        if cidx == selection.end {
            byte_selection_end = Some(idx)
        }
    }

    let byte_selection_start = byte_selection_start.expect("byte_selection_start_not_found");
    let byte_selection_end = byte_selection_end.expect("byte_selection_end_not_found");

    (
        &value[0..byte_selection_start],
        &value[byte_selection_start..byte_selection_end],
        &value[byte_selection_end..value.len()],
    )
}

/// Is the first char alphanumeric?
pub(crate) fn is_alphanumeric(s: &str) -> bool {
    if let Some(c) = s.chars().next() {
        c.is_alphanumeric()
    } else {
        false
    }
}

/// Small helper for handling mouse-events.
///
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MouseFlags {
    pub armed: Option<SystemTime>,
    pub drag: bool,
}

impl MouseFlags {
    /// Handling mouse drag events for this widget is enabled.
    /// It may make sense for a component to track mouse events outside its area.
    /// But usually with some limitations. This flag signals that those limits
    /// have been met, and drag event should be processed.
    pub fn do_drag(&self) -> bool {
        self.drag
    }

    /// Enable handling mouse drag events for the widget.
    pub fn set_drag(&mut self) {
        self.drag = true;
    }

    /// Clear the do-drag flag.
    pub fn clear_drag(&mut self) {
        self.drag = false;
    }

    /// Reset the double-click trigger.
    pub fn reset_trigger(&mut self) {
        self.armed = None;
    }

    /// Unconditionally set a new time for the trigger.
    pub fn arm_trigger(&mut self) {
        self.armed = Some(SystemTime::now());
    }

    /// Pull the trigger, returns true if the action is triggered.
    pub fn pull_trigger(&mut self, time_out: u64) -> bool {
        match &self.armed {
            None => {
                self.armed = Some(SystemTime::now());
                false
            }
            Some(armed) => {
                let elapsed = armed.elapsed().expect("timeout");
                if elapsed > Duration::from_millis(time_out) {
                    self.armed = None;
                    false
                } else {
                    self.armed = None;
                    true
                }
            }
        }
    }
}

/// Result value for event-handling.
pub enum Outcome {
    /// The given event was not handled at all.
    Unused,
    /// The event was handled, no repaint necessary.
    Unchanged,
    /// The event was handled, repaint necessary.
    Changed,
}
