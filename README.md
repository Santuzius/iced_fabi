# iced_fabi

iced File cABInet: an iced GUI widget for browsing directories and sorting or selecting items.

Two views over one state:

- **Browse** — one directory per screen with a breadcrumb bar (Android Documents-App style).
- **Tree** — the whole tree in place, every branch expandable; selection can span levels.

Every row carries a Material-style selection marker: an empty box, a tick once the item is selected as a unit, and a dot while only some of its content is selected. The column header repeats that box as a select-all for the whole list (not under single selection, where there is no "all"), and while anything is checked it reads "N selected" in place of the column titles. The row under the pointer lights up weaker than a selection; folder glyphs, header row and hover all take their own [`Style`] colours.

Listings are **supplied by the application** — local disk, rclone, an HTTP API: the widget reports which nodes need loading, the app answers with listings tagged to the node, and stale responses simply no-op.

```rust
use iced_fabi::{Browser, Entry, Event, NodeId, Outcome, tool};

#[derive(Clone)]
enum MyMessage { Cabinet(Event), Copy }

// In your boot(): configure the behaviour, then ask for the first listing.
let mut browser = Browser::new("My files");
browser.add_column(MyExtensionColumn); // any impl of the `Column` trait: header, cell, sort order
let outcome = browser.open();          // -> Outcome::NeedsLoad(NodeId::ROOT): go fetch

// In your update(): one call, one match — no routing knowledge on the app side.
match message {
    MyMessage::Cabinet(event) => match browser.update(event) {
        Outcome::NeedsLoad(id) => { /* list `id` (browser.path_below_root(id) gives the backend path), later: browser.set_children(id, entries) */ }
        Outcome::FileOpened(id) => { /* open it */ }
        Outcome::None | Outcome::SelectionChanged => {}
    },
    // Your own buttons can drive the state directly:
    MyMessage::Copy => { for id in browser.selected().collect::<Vec<_>>() { /* put `id` on the app's clipboard */ } }
}

// In your view(): the toolbar takes your `Tool`s (glyph, enabled-ness, message) and `tool_separator` divides them into groups; one `ActionButton` (label, message, placement, enable rule) sits where you choose; `style` maps the theme to your colours (folder glyphs, the header row, the `row_hover` of the row under the pointer, a zebra `stripe` behind every second row, the surface it sits on and the hover tint of its flat buttons); `labels` localises the status and count lines; `text_size`, `font` and `scrollbar` match your app's type family, size and scrollbar; `icons` swaps the file/folder glyphs; and `separators` draws a line grid. Overlong labels are cut to their column with an ellipsis, measured at layout time — nothing to configure. On a narrow screen the columns fit themselves to the width: each keeps the room its values need (`Column::min_width`), the Name column gives way, and only where even that does not fit are the rightmost columns left out.
iced_fabi::cabinet(&browser, MyMessage::Cabinet)
    .tool(tool(icondata::TbCopyOutline, |_| MyMessage::Copy).when(|b| b.has_selection()))
    .tool_separator()
    .action(iced_fabi::ActionButton::new("Download", MyMessage::Copy))
    .style(|theme| MyStyle::from_app_design(theme)) // any Style, not just Style::from_theme
    .icons(&MyIconTheme) // impl `RowIcons`: monochrome packs get tinted, multicolour ones keep their fills
    .separators(iced_fabi::Separators::Both)
    .into() // or .build()
```

As a picker in a modal — "Browse…" instead of a typed path:

```rust
use iced_fabi::{ActionButton, ActionPlacement, Browser, Pick};

// Opening: one folder (Pick::Folder), any folders (Pick::Folders) or any entries (Pick::Entries).
let mut picker = Browser::picker("My drive", Pick::Folders);
picker.select_paths(&saved_paths); // ticked before their folders are listed; parents show the dot
let outcome = picker.open();       // NeedsLoad(ROOT), as always

// View: a confirm / Cancel footer in the look of the cabinet's own dialogs, and a fixed height for the modal card.
iced_fabi::cabinet(&picker, Message::Picker)
    .action(ActionButton::new("Save", Message::Confirm).placement(ActionPlacement::Footer))
    .cancel(Message::Cancel)
    .height(440)

// On Message::Confirm: `picker.picked()` is the selection as `/`-joined paths below the root —
// a folder picker with nothing ticked hands over the folder it has open ("" is the root).
```

Three examples:

```sh
cargo run --example minimal       # the whole API surface on ~100 lines, straight from std::fs
cargo run --example cloud_browse --features material-icon-theme  # a fake cloud drive answering one tick late
cargo run --example picker        # a settings form filled through folder, exclusion and .nomedia pickers
```

The cloud demo's bar switches every setting live: theme (including a reproduction of Syncative's palette and [`Style`]), icon pack, separator grid, zebra stripe, which entries are listed (all or folders only), selection (`multiple` / `single` / `none`), how the boxes appear (`shown` / `on long press` / `hidden`), where the action button sits (or that the cabinet gets none), and what happens to inapplicable toolbar tools (greyed or hidden).

Icons: [Tabler Icons](https://tabler.io/icons) (MIT) and the three selection checkboxes from [Boxicons](https://boxicons.com) (MIT) via [`icondata`](https://crates.io/crates/icondata). The optional feature `material-icon-theme` adds [`MaterialIconTheme`], the file and folder glyphs of the [Material Icon Theme](https://github.com/material-extensions/vscode-material-icon-theme) (MIT, © Material Extensions, v5.39.0), for `.icons(&iced_fabi::MaterialIconTheme)`; the crate carries a cut of its SVGs under `assets/material-icon-theme/` with the upstream license text, provenance in [NOTICE](NOTICE).

License: MIT.
