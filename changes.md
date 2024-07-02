# 0.17.0

Discontinued. Moved everything to rat-widget as the original reason for
this split is no longer valid.

# 0.16.6

* Add PopupMenu, MenuBar widgets. Synchronize APIs with MenuLine.

# 0.16.5

* refactor: moved focus and invalid from the widget to the state.
  when using StatefulWidgetRef this was the wrong place.
* impl StatefulWidgetRef
* DateInput, NumberInput: add all functions from the underlying MaskedInput.

* fix MsgDialog: must consume all events.
* fix TextInput replace text.
* fix Button + Enter
* fix Button + orphaned release Enter

# 0.16.4

* add NumberInput

* rename new_localized() to new_loc()
* fix: TextInput shouldn't render selection if not focused.
*

# 0.16.3

* add Fill widget. Clears an area.

* fix menuline panic by `- 1`
* fix strange but when a menu is selected at startup. reacted to Release-Enter
  of starting the program on the command line.

# 0.16.2

* add label_at, widget_at to LayoutEdit.

# 0.16.1

* rat-event got a reorg. mirror this.

# 0.16.0

* Use new MouseFlags.

# 0.15.0

* Add TextArea.
* Add support for 2-wide Emojis. Works ok. Input in Windows-Terminal
  seems somewhat broken? Alacritty does better, so I think its Windows-Terminal.
  Or somebody mixes up the events? Simple emojis work though, but the
  combined ones are jittery and break rendering sometimes ....
    * Added for TextArea, TextInput and MaskedInput
* API cleanup between the three text input widgets.

# 0.14.0

* Remove StatefulWidgetRef

# 0.13.3

* Add optimization when dragging the cursor to select text.
  Only return Changed if the selection changed.

# 0.13.2

* Use rat-event::Outcome

# 0.13.1

* Add missing Clone, Debug, Default.

# 0.13.0

* Use new trait UsedEvent.

# 0.12.0

* Add layout_edit() and layout_dialog()

# 0.11.0

* Add calender widget `Month`
* Add menu widget `MenuLine`
* Add basic `MsgDialog`
* Add widget `StatusLine`

# 0.10.1

Fix some docs.

# 0.10.0

* Move HandleEvent trait to separate crate and reexport.
* Add Button and DateInput

# 0.9.0

Initial release with TextInput and MaskedInput