# Architecture

One crate, two halves: the state (`browser/`) and the widget (`cabinet/`), plus the few value types the app names. Dependencies point one way: `lib → cabinet → browser / column / style / icon / tool`; `browser` knows nothing about iced widgets.

- `src/lib.rs` — crate docs and the `pub use` list; the only place anything is re-exported.
- `src/browser/` — the [`Browser`](crate::Browser) state. Every file adds its own `impl Browser` block, so the fields stay private to `browser` while each topic works on them directly:
  - `mod.rs` — the struct, `new`/`open`, `update` (the whole event routing), and the settings that do not belong to one topic: columns (`add_column`, `set_columns`, `sort_by_column`) and their visibility (`hide_column`/`show_column`, `set_visible_columns`), the tool visibility ids and the clipboard flag behind the built-in Paste.
  - `node.rs` — the pure-data vocabulary: `NodeId` (derived ids), `Entry` (with `folder`/`file` builders), `Node` (carries its `id`), `EntryKind`, `Marker`, `Mode`, the selection enums, and the tree helpers `is_below` / `Node::id`.
  - `loading.rs` — the listing feed (`set_children`, `load_failed`, `request_load`) and removal (`remove`, `remove_selected`, `prune`).
  - `navigation.rs` — `activate`, `tap`, `goto`, `up`, mode, and the position queries (`current`, `path`, `node`, `name_chain`, `path_below_root`).
  - `selection.rs` — Material-style unit selection: `toggle`, `toggle_all`, `select_all`, `clear_selection`, markers, and the covered-entry rules.
  - `long_press.rs` — the hold that arms the boxes in `Checkboxes::OnLongPress` (`hold_started`, `hold_ended`, `disarm`, `LONG_PRESS`).
  - `rows.rs` — `browse_rows` / `tree_rows` and the sorting plus the visibility filters (`folders_only`, `file_extensions`) that decide which entries appear.
  - `tests/` — one file per topic module; the shared root listing is `Browser::sample`.
- `src/cabinet/` — the [`Cabinet`](crate::Cabinet) widget, rebuilt per frame:
  - `mod.rs` — the struct, builder setters (all settings in one place), `build`, and `Separators`.
  - `paint.rs` — geometry constants, the `ColourField` colour selector, flat-button, rule and background-layer helpers, the column `template` header and rows share (its `rules` argument says which rows carry the upright rules, hidden columns are skipped whole), and `stripe_of` for the zebra colour.
  - `toolbar.rs` — breadcrumbs, mode switch, tool groups; browser-hidden tools drop out before the pure separator-collapsing rule `prune_separators`, which has its own tests.
  - `header.rs` — the select-all box, sort buttons, selection count.
  - `list.rs` — browse/tree lists, one row, status lines, cells.
  - `action.rs` — the `ActionButton` (placement, `when` rule) and its `ActionPlacement`.
  - `labels.rs` — the `Labels` (status lines, count line) and `Cabinet::labels`.
  - `cut.rs` — the ellipsis label (a plain iced `Widget`).
- `src/column.rs` — the [`Column`](crate::Column) trait (header, `id`, width, cell, compare), the built-in Name/Modified/Size columns, the `BuiltInColumn` unit enum and the `ColumnKey` used by the visibility lists.
- `src/natural.rs` — natural ("File 2" < "File 10") comparison keys.
- `src/format.rs` — `format_bytes`, `format_epoch`.
- `src/tool.rs` — the toolbar [`Tool`](crate::Tool) trait (glyph, `id`, enabled, message), the closure-built [`SimpleTool`](crate::SimpleTool), the `BuiltInTool` unit enum and the `ToolKey` used by the visibility configuration.
- `src/icon.rs` — cached SVG handles, the glyph constants, and the [`RowIcons`](crate::RowIcons) hook with `TablerIcons`.
- `src/event.rs` — the [`Event`](crate::Event) enum the widget emits.
- `src/style.rs` — the [`Style`](crate::Style) palette hook (`Style::from_theme` is the default).

Until the first release the public API may change freely; decision history lives in the commit messages, not here. What the settings mean, in one place:

- `Browser` (state, persists): `Selection::{Multiple, Single, None}` how many entries can be selected — `None` is a pure view; `Checkboxes::{Shown, OnLongPress, Hidden}` where the boxes are — orthogonal to `Selection`, and while `Hidden` a file tap still selects silently (the boxes are presentation, not permission); `tap` ticks a file only when a `Selection` is possible and the boxes are not merely waiting for a revealing hold. Events all go into `Browser::update`, which hides the routing and returns the `Outcome` the app must act on; `open()` starts the root load.
- Entries arrive through `Entry::folder(..)` / `Entry::file(..)` (+ `.size()` / `.modified()`); `Node` carries its derived `id`; `path_below_root(id)` is the backend path without the root's display name.
- `Cabinet` (per frame): `action(ActionButton)` (placement + enable rule, default: enabled while something is selected), `labels(Labels)` for the status/count lines (headers belong to the columns), `style(Style::from_theme)` default, and the chain ends with `.into()` (no `build()` requirement, no `file_cabinet`).
