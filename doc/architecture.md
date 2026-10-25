# Architecture

One crate, two halves: the state (`browser/`) and the widget (`cabinet/`), plus the few value types the app names. Dependencies point one way: `lib → cabinet → browser / column / style / icon / tool`; `browser` knows nothing about iced widgets.

- `src/lib.rs` — crate docs and the `pub use` list; the only place anything is re-exported.
- `src/browser/` — the [`Browser`](crate::Browser) state. Every file adds its own `impl Browser` block, so the fields stay private to `browser` while each topic works on them directly:
  - `mod.rs` — the struct, `new`, and the settings that do not belong to one topic: columns (`add_column`, `sort_by_column`) and the formatters.
  - `node.rs` — the pure-data vocabulary: `NodeId` (derived ids), `Entry`, `Node`, `EntryKind`, `Marker`, `Mode`, the selection enums, and the tree helpers `is_below` / `Node::id`.
  - `loading.rs` — the listing feed (`set_children`, `load_failed`, `request_load`) and removal (`remove`, `remove_selected`, `prune`).
  - `navigation.rs` — `activate`, `tap`, `goto`, `up`, mode, and the position queries (`current`, `path`, `node`, `name_chain`).
  - `selection.rs` — Material-style unit selection: `toggle`, `toggle_all`, `select_all`, `clear_selection`, markers, and the covered-entry rules.
  - `long_press.rs` — the hold that arms the selection in `SelectionUi::LongPress` (`hold_started`, `hold_ended`, `disarm`, `LONG_PRESS`).
  - `rows.rs` — `browse_rows` / `tree_rows` and the sorting plus the visibility filters (`folders_only`, `file_extensions`) that decide which entries appear.
  - `tests/` — one file per topic module; the shared root listing is `Browser::sample`.
- `src/cabinet/` — the [`Cabinet`](crate::Cabinet) widget, rebuilt per frame:
  - `mod.rs` — the struct, builder setters (all settings in one place), `build`, and `Separators`.
  - `paint.rs` — geometry constants, the `ColourField` colour selector, flat-button and rule helpers, and the column `template` header and rows share.
  - `toolbar.rs` — breadcrumbs, mode switch, tool groups; the separator-collapsing rule is a pure `prune_separators` with its own tests.
  - `header.rs` — the select-all box, sort buttons, selection count.
  - `list.rs` — browse/tree lists, one row, status lines, cells.
  - `action.rs` — the action button and its `ActionPlacement`.
  - `cut.rs` — the ellipsis label (a plain iced `Widget`).
- `src/column.rs` — the [`Column`](crate::Column) trait and the built-in Name/Modified/Size columns.
- `src/natural.rs` — natural ("File 2" < "File 10") comparison keys.
- `src/format.rs` — `format_bytes`, `format_epoch`.
- `src/tool.rs` — the toolbar [`Tool`](crate::Tool) trait and the closure-built [`SimpleTool`](crate::SimpleTool).
- `src/icon.rs` — cached SVG handles, the glyph constants, and the [`RowIcons`](crate::RowIcons) hook with `TablerIcons`.
- `src/event.rs` — the [`Event`](crate::Event) enum the widget emits.
- `src/style.rs` — the [`Style`](crate::Style) palette hook and `default_style`.

Until the first release the public API may change freely; decision history lives in the commit messages, not here.
