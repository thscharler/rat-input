//!
//! A widget for date-input using [crate::chrono]
//!

use crate::_private::NonExhaustive;
use crate::event::Outcome;
use crate::masked_input::{MaskedInput, MaskedInputState, MaskedInputStyle};
use chrono::format::{Fixed, Item, Numeric, Pad, StrftimeItems};
use chrono::{Datelike, Days, Local, Months, NaiveDate};
#[allow(unused_imports)]
use log::debug;
use rat_event::{ct_event, FocusKeys, HandleEvent, MouseOnly};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, StatefulWidget};
use std::fmt;
use std::fmt::Debug;
use unicode_segmentation::UnicodeSegmentation;

/// Widget for dates.
#[derive(Debug, Default, Clone)]
pub struct DateInput<'a> {
    widget: MaskedInput<'a>,
}

/// State.
///
/// Use `DateInputState::new(_pattern_)` to set the date pattern.
///
#[derive(Debug, Clone)]
pub struct DateInputState {
    /// uses MaskedInputState for the actual functionality.
    pub widget: MaskedInputState,
    /// The chrono format pattern.
    pattern: String,
    /// Locale
    locale: chrono::Locale,

    pub non_exhaustive: NonExhaustive,
}

impl<'a> DateInput<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the compact form, if the focus is not with this widget.
    #[inline]
    pub fn show_compact(mut self, show_compact: bool) -> Self {
        self.widget = self.widget.show_compact(show_compact);
        self
    }

    /// Set the combined style.
    #[inline]
    pub fn styles(mut self, style: MaskedInputStyle) -> Self {
        self.widget = self.widget.styles(style);
        self
    }

    /// Base text style.
    #[inline]
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.widget = self.widget.style(style);
        self
    }

    /// Style when focused.
    #[inline]
    pub fn focus_style(mut self, style: impl Into<Style>) -> Self {
        self.widget = self.widget.focus_style(style);
        self
    }

    /// Style for selection
    #[inline]
    pub fn select_style(mut self, style: impl Into<Style>) -> Self {
        self.widget = self.widget.select_style(style);
        self
    }

    /// Style for the invalid indicator.
    #[inline]
    pub fn invalid_style(mut self, style: impl Into<Style>) -> Self {
        self.widget = self.widget.invalid_style(style);
        self
    }

    #[inline]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.widget = self.widget.block(block);
        self
    }

    /// Renders the content differently if focused.
    ///
    /// * Selection is only shown if focused.
    ///
    #[inline]
    pub fn focused(mut self, focused: bool) -> Self {
        self.widget = self.widget.focused(focused);
        self
    }

    /// Renders the content differently if invalid.
    /// Uses the invalid style instead of the base style for rendering.
    #[inline]
    pub fn valid(mut self, valid: bool) -> Self {
        self.widget = self.widget.valid(valid);
        self
    }
}

impl<'a> StatefulWidget for DateInput<'a> {
    type State = DateInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.widget.render(area, buf, &mut state.widget);
    }
}

impl Default for DateInputState {
    fn default() -> Self {
        Self {
            widget: Default::default(),
            pattern: Default::default(),
            locale: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl DateInputState {
    pub fn new<S: AsRef<str>>(pattern: S) -> Result<Self, fmt::Error> {
        let mut s = Self::default();
        s.set_format(pattern)?;
        Ok(s)
    }

    #[inline]
    pub fn new_localized<S: AsRef<str>>(
        pattern: S,
        locale: chrono::Locale,
    ) -> Result<Self, fmt::Error> {
        let mut s = Self::default();
        s.set_format_loc(pattern, locale)?;
        Ok(s)
    }

    /// Reset to empty.
    #[inline]
    pub fn reset(&mut self) {
        self.widget.reset();
    }

    /// chrono format string.
    #[inline]
    pub fn format(&self) -> &str {
        self.pattern.as_str()
    }

    /// chrono locale.
    #[inline]
    pub fn locale(&self) -> chrono::Locale {
        self.locale
    }

    /// chrono format string.
    ///
    /// generates a mask according to the format and overwrites whatever
    /// set_mask() did.
    #[inline]
    pub fn set_format<S: AsRef<str>>(&mut self, pattern: S) -> Result<(), fmt::Error> {
        self.set_format_loc(pattern, chrono::Locale::default())
    }

    /// chrono format string.
    ///
    /// generates a mask according to the format and overwrites whatever
    /// set_mask() did.
    #[inline]
    pub fn set_format_loc<S: AsRef<str>>(
        &mut self,
        pattern: S,
        locale: chrono::Locale,
    ) -> Result<(), fmt::Error> {
        let mut mask = String::new();
        let items = StrftimeItems::new_with_locale(pattern.as_ref(), locale)
            .parse()
            .map_err(|_| fmt::Error)?;
        for t in &items {
            match t {
                Item::Literal(s) => {
                    for c in s.graphemes(true) {
                        mask.push('\\');
                        mask.push_str(c);
                    }
                }
                Item::OwnedLiteral(s) => {
                    for c in s.graphemes(true) {
                        mask.push('\\');
                        mask.push_str(c);
                    }
                }
                Item::Space(s) => {
                    for c in s.graphemes(true) {
                        mask.push_str(c);
                    }
                }
                Item::OwnedSpace(s) => {
                    for c in s.graphemes(true) {
                        mask.push_str(c);
                    }
                }
                Item::Numeric(v, Pad::None | Pad::Space) => match v {
                    Numeric::Year | Numeric::IsoYear => mask.push_str("9999"),
                    Numeric::YearDiv100
                    | Numeric::YearMod100
                    | Numeric::IsoYearDiv100
                    | Numeric::IsoYearMod100
                    | Numeric::Month
                    | Numeric::Day
                    | Numeric::WeekFromSun
                    | Numeric::WeekFromMon
                    | Numeric::IsoWeek
                    | Numeric::Hour
                    | Numeric::Hour12
                    | Numeric::Minute
                    | Numeric::Second => mask.push_str("99"),
                    Numeric::NumDaysFromSun | Numeric::WeekdayFromMon => mask.push('9'),
                    Numeric::Ordinal => mask.push_str("999"),
                    Numeric::Nanosecond => mask.push_str("999999999"),
                    Numeric::Timestamp => mask.push_str("###########"),
                    _ => return Err(fmt::Error),
                },
                Item::Numeric(v, Pad::Zero) => match v {
                    Numeric::Year | Numeric::IsoYear => mask.push_str("0000"),
                    Numeric::YearDiv100
                    | Numeric::YearMod100
                    | Numeric::IsoYearDiv100
                    | Numeric::IsoYearMod100
                    | Numeric::Month
                    | Numeric::Day
                    | Numeric::WeekFromSun
                    | Numeric::WeekFromMon
                    | Numeric::IsoWeek
                    | Numeric::Hour
                    | Numeric::Hour12
                    | Numeric::Minute
                    | Numeric::Second => mask.push_str("00"),
                    Numeric::NumDaysFromSun | Numeric::WeekdayFromMon => mask.push('0'),
                    Numeric::Ordinal => mask.push_str("000"),
                    Numeric::Nanosecond => mask.push_str("000000000"),
                    Numeric::Timestamp => mask.push_str("#0000000000"),
                    _ => return Err(fmt::Error),
                },
                Item::Fixed(v) => match v {
                    Fixed::ShortMonthName => mask.push_str("___"),
                    Fixed::LongMonthName => mask.push_str("_________"),
                    Fixed::ShortWeekdayName => mask.push_str("___"),
                    Fixed::LongWeekdayName => mask.push_str("________"),
                    Fixed::LowerAmPm => mask.push_str("__"),
                    Fixed::UpperAmPm => mask.push_str("__"),
                    Fixed::Nanosecond => mask.push_str(".#########"),
                    Fixed::Nanosecond3 => mask.push_str(".###"),
                    Fixed::Nanosecond6 => mask.push_str(".######"),
                    Fixed::Nanosecond9 => mask.push_str(".#########"),
                    Fixed::TimezoneName => mask.push_str("__________"),
                    Fixed::TimezoneOffsetColon | Fixed::TimezoneOffset => mask.push_str("+##:##"),
                    Fixed::TimezoneOffsetDoubleColon => mask.push_str("+##:##:##"),
                    Fixed::TimezoneOffsetTripleColon => mask.push_str("+##"),
                    Fixed::TimezoneOffsetColonZ | Fixed::TimezoneOffsetZ => return Err(fmt::Error),
                    Fixed::RFC2822 => {
                        // 01 Jun 2016 14:31:46 -0700
                        return Err(fmt::Error);
                    }
                    Fixed::RFC3339 => {
                        // not supported, for now
                        return Err(fmt::Error);
                    }
                    _ => return Err(fmt::Error),
                },
                Item::Error => return Err(fmt::Error),
            }
        }

        self.locale = locale;
        self.pattern = pattern.as_ref().to_string();
        self.widget.set_mask(mask)?;
        Ok(())
    }

    /// Parses the text according to the given pattern.
    #[inline]
    pub fn value(&self) -> Result<NaiveDate, chrono::ParseError> {
        NaiveDate::parse_from_str(self.widget.compact_value().as_str(), self.pattern.as_str())
    }

    /// Set the date value.
    #[inline]
    pub fn set_value(&mut self, date: NaiveDate) {
        let v = date.format(self.pattern.as_str()).to_string();
        self.widget.set_value(v);
    }

    /// Select all text.
    #[inline]
    pub fn select_all(&mut self) {
        self.widget.select_all()
    }

    /// Screen position of the cursor for rendering.
    #[inline]
    pub fn screen_cursor(&self) -> Option<Position> {
        self.widget.screen_cursor()
    }
}

/// Add convenience keys:
/// * `h` - today
/// * `a` - January, 1st
/// * `e` - December, 31st
/// * `l` - first of last month
/// * `L` - last of last month
/// * `m` - first of this month
/// * `M` - last of this month
/// * `n` - first of next month
/// * `N` - last of next month
/// * `j` - add month
/// * `k` - subtract month
/// * `J` - add year
/// * `K` - subtract year
///
/// Calls handle(FocusKeys) afterwards.
#[derive(Debug)]
pub struct ConvenientKeys;

impl HandleEvent<crossterm::event::Event, ConvenientKeys, Outcome> for DateInputState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: ConvenientKeys) -> Outcome {
        let r = {
            match event {
                ct_event!(key press 'h') => {
                    self.set_value(Local::now().date_naive());
                    Outcome::Changed
                }
                ct_event!(key press 'l') => {
                    let date = Local::now()
                        .date_naive()
                        .checked_sub_months(Months::new(1))
                        .expect("month")
                        .with_day(1)
                        .expect("day");
                    self.set_value(date);
                    Outcome::Changed
                }
                ct_event!(key press SHIFT-'L') => {
                    let date = Local::now()
                        .date_naive()
                        .with_day(1)
                        .expect("month")
                        .checked_sub_days(Days::new(1))
                        .expect("day");
                    self.set_value(date);
                    Outcome::Changed
                }

                ct_event!(key press 'm') => {
                    let date = Local::now().date_naive().with_day(1).expect("day");
                    self.set_value(date);
                    Outcome::Changed
                }
                ct_event!(key press SHIFT-'M') => {
                    let date = Local::now()
                        .date_naive()
                        .checked_add_months(Months::new(1))
                        .expect("month")
                        .with_day(1)
                        .expect("day")
                        .checked_sub_days(Days::new(1))
                        .expect("day");
                    self.set_value(date);
                    Outcome::Changed
                }

                ct_event!(key press 'n') => {
                    let date = Local::now()
                        .date_naive()
                        .checked_add_months(Months::new(1))
                        .expect("month")
                        .with_day(1)
                        .expect("day");
                    self.set_value(date);
                    Outcome::Changed
                }
                ct_event!(key press SHIFT-'N') => {
                    let date = Local::now()
                        .date_naive()
                        .checked_add_months(Months::new(2))
                        .expect("month")
                        .with_day(1)
                        .expect("day")
                        .checked_sub_days(Days::new(1))
                        .expect("day");
                    self.set_value(date);
                    Outcome::Changed
                }

                ct_event!(key press 'j') => {
                    if let Ok(date) = self.value() {
                        let date = date.checked_add_months(Months::new(1)).expect("month");
                        self.set_value(date);
                    }
                    Outcome::Changed
                }
                ct_event!(key press SHIFT-'J') => {
                    if let Ok(date) = self.value() {
                        let date = date.with_year(date.year() + 1).expect("year");
                        self.set_value(date);
                    }
                    Outcome::Changed
                }

                ct_event!(key press 'k') => {
                    if let Ok(date) = self.value() {
                        let date = date.checked_sub_months(Months::new(1)).expect("month");
                        self.set_value(date);
                    }
                    Outcome::Changed
                }
                ct_event!(key press SHIFT-'K') => {
                    if let Ok(date) = self.value() {
                        let date = date.with_year(date.year() - 1).expect("year");
                        self.set_value(date);
                    }
                    Outcome::Changed
                }

                ct_event!(key press 'a'|'b') => {
                    if let Ok(date) = self.value() {
                        let date = date.with_month(1).expect("month").with_day(1).expect("day");
                        self.set_value(date);
                    }
                    Outcome::Changed
                }
                ct_event!(key press 'e') => {
                    if let Ok(date) = self.value() {
                        let date = date
                            .with_month(12)
                            .expect("month")
                            .with_day(31)
                            .expect("day");
                        self.set_value(date);
                    }
                    Outcome::Changed
                }
                _ => Outcome::NotUsed,
            }
        };

        if r == Outcome::NotUsed {
            self.handle(event, FocusKeys)
        } else {
            r
        }
    }
}

impl HandleEvent<crossterm::event::Event, FocusKeys, Outcome> for DateInputState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: FocusKeys) -> Outcome {
        self.widget.handle(event, FocusKeys)
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for DateInputState {
    fn handle(&mut self, event: &crossterm::event::Event, _keymap: MouseOnly) -> Outcome {
        self.widget.handle(event, MouseOnly)
    }
}

/// Handle all events.
/// Text events are only processed if focus is true.
/// Mouse events are processed if they are in range.
pub fn handle_events(
    state: &mut DateInputState,
    focus: bool,
    event: &crossterm::event::Event,
) -> Outcome {
    if focus {
        HandleEvent::handle(state, event, FocusKeys)
    } else {
        HandleEvent::handle(state, event, MouseOnly)
    }
}

/// Handle only mouse-events.
pub fn handle_mouse_events(state: &mut DateInputState, event: &crossterm::event::Event) -> Outcome {
    HandleEvent::handle(state, event, MouseOnly)
}
