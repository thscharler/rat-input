[![crates.io](https://img.shields.io/crates/v/rat-input.svg)](https://crates.io/crates/rat-input)
[![Documentation](https://docs.rs/rat-input/badge.svg)](https://docs.rs/rat-input)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![License](https://img.shields.io/badge/license-APACHE-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
![](https://tokei.rs/b1/github/thscharler/rat-input)

## Data input widgets for ratatui.

These widgets are plain vanilla ratatui widgets.

Eventhandling is currently crossterm only.
In practice event-handling is calling 1 or 2 functions on the state, so this
should be easy to map to other systems. (Contributions welcome :)

# [TextArea](crate::textarea)

Editable text area.

* Range based text styles.
* Text selection with keyboard + mouse
* Possible states as style: Focused
* Emoji supported.

![image](https://github.com/thscharler/rat-input/blob/master/textarea.gif?raw=true)

# [TextInput](crate::input)

Basic text input field.

* Text selection with keyboard + mouse
* Possible states as styles: Focused, Invalid

# [MaskedInput](crate::masked_input)

Text input with an input mask.

* Text selection with keyboard + mouse
* Possible states as styles: Focused, Invalid
* Pattern based input -> "##,###,##0.00"
    * number patterns: `09#-+.,`
    * numeric text: `HhOoDd`
    * text: `lac_`
    * arbitrary separators between sub-fields
* info-overlay for sub-fields without value
* Localization with [rat-input::NumberSymbols] based on [pure-rust-locales](pure-rust-locales)

# [Button](crate::button::Button)

Simple button widget.

# [DateInput](crate::date_input), [NumberInput](crate::number_input)

Date input with format strings parsed by [chrono](https://docs.rs/chrono/latest/chrono/).
Number input with format strings parsed
by [format_num_pattern](https://docs.rs/format_num_pattern/latest/format_num_pattern/)

# [Month](crate::calendar)

Widget for calender display.

# [MenuLine](crate::menuline), [PopupMenu](crate::popup_menu) and [MenuBar](crate::menubar)

Menu widgets.

# [StatusLine](crate::statusline)

Statusline with multiple segments.

## TODO

... more widgets 