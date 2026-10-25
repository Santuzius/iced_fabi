# iced_fabi

iced File cABInet: an iced GUI widget for browsing directories and sorting or selecting items.

Two views over one state:

- **Browse** — one directory per screen with a breadcrumb bar (Android Documents-App style).
- **Tree** — the whole tree in place, every branch expandable; selection can span levels.

Every row carries a Material-style selection marker: an empty box, a tick once the item is selected as a unit, and a dot while only some of its content is selected. The column header repeats that box as a select-all for the whole list, and while anything is checked it reads "N selected" in place of the column titles. The row under the pointer lights up weaker than a selection; folder glyphs, header row and hover all take their own [`Style`] colours.

Listings are **supplied by the application** — local disk, rclone, an HTTP API: the widget reports which nodes need loading, the app answers with listings tagged to the node, and stale responses simply no-op.

```rust
let mut browser = Browser::new("My files");
browser.add_column(MyExtensionColumn); // any impl of the `Column` trait: header, cell, sort order

// In your update():
match event {
    Event::Activate(id) => match browser.tap(id) {
        Outcome::NeedsLoad(id) => { /* list `id`, later: browser.set_children(id, entries); */ }
        Outcome::FileActivated(id) => { /* open it */ }
        Outcome::None => {}
    },
    Event::Toggle(id) => browser.toggle(id),
    Event::ToggleAll => browser.toggle_all(),
    Event::GoTo(id) => browser.goto(id),
    Event::Sort(column) => browser.sort_by_column(column),
    Event::ToggleMode => browser.set_mode(match browser.mode() { Mode::Browse => Mode::Tree, Mode::Tree => Mode::Browse }),
}

// In your view(): the toolbar takes your `Tool`s (glyph, enabled-ness, message) and `tool_separator` divides them into groups; `style` maps the theme to your colours (folder glyphs, the header row, the `row_hover` of the row under the pointer, a zebra `stripe` behind every second row, the surface it sits on and the hover tint of its flat buttons); `text_size`, `font` and `scrollbar` match your app's type family, size and scrollbar; `icons` swaps the file/folder glyphs; and `separators` draws a line grid. Overlong labels are cut to their column with an ellipsis, measured at layout time — nothing to configure.
iced_fabi::cabinet(&browser, &MyMessage::Cabinet)
    .tool(tool(icondata::TbListCheckOutline, |_| MyMessage::SelectAll).when(|b| b.has_selection()))
    .tool_separator()
    .tool(tool(icondata::TbCopyOutline, |_| MyMessage::Copy).when(|b| b.has_selection()))
    .style(|theme| MyStyle::from_app_design(theme)) // any Style, not just the palette default
    .icons(&MyIconTheme) // impl `RowIcons`: monochrome packs get tinted, multicolour ones keep their fills
    .separators(iced_fabi::Separators::Both)
    .build()
```

Run the demo (a fake cloud drive that answers listings one tick late; its bar switches theme — including a reproduction of Syncative's palette and widget colours —, icon pack, separator grid, zebra stripe and inactive-tool behaviour):

```sh
cargo run --example cloud_browse
```

Icons: [Tabler Icons](https://tabler.io/icons) (MIT) via [`icondata`](https://crates.io/crates/icondata). The demo also carries a cut of the [Material Icon Theme](https://github.com/material-extensions/vscode-material-icon-theme) (MIT, © Material Extensions, v5.39.0) under `examples/assets/material-icon-theme/` with the upstream license text; provenance in [NOTICE](NOTICE).

License: MIT.
