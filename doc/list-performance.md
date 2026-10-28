# Big listings: what iced 0.14 offers, and what exists elsewhere

Measured with [`examples/large.rs`](../examples/large.rs) (`cargo run --release --example large`), idle h7510, fastest of 100 runs per scenario, two runs agreeing within 1 % (2026-10-28):

- browse 10 000 files: 55 ms, so about 5.5 µs per row.
- browse 10 000 files with two columns hidden: 38 ms. Sorted by Size instead of Name: 47 ms.
- tree 30×30×30, 27 931 rows: 184–190 ms, about 6.6 µs per row.
- tree 1000 folders deep, 1002 rows: 2.3 ms, 2.3 µs per row.
- `set_children` with 10 000 entries: 0.85 ms — 65× cheaper than one build of that listing, so the per-frame cost is in building the row `Element`s, not in the model or the sort.
- empty browser: 0.002 ms, the fixed cost of toolbar, header and scrollable.

Only compare runs from the same session: the same fixtures measured more than twice as slow while the machine was busy (browse 10 000 files 110–127 ms, the 1000-deep chain 69 ms), and the mean runs 20–40 % above the fastest run.

Two per-row walks are already gone: each name key is built once per listing rather than once per comparison (33c6b00), and the selection markers are collected once per frame rather than by walking every row's ancestor chain and subtree (c9d3142, which took the 1000-deep chain from 35 ms to 2.3 ms). What is left per row is its `Element` tree.

Where those microseconds go, as far as the harness can tell:

- Three columns cost about 5.6 µs per row, one column and two hidden about 3.8 µs. The difference is the visible columns' layout: the `container` per cell and the rule before it.
- Building a hidden column's cell (its `Column::cell`, plus the text cut for its content) is only about 4 % of a row: not building it at all moved the two-hidden scenario 39.0 ms → 37.6 ms, and that is the whole saving.
- Sorting by Size leaves about 103 rows per size value, so nearly every comparison falls through to the name tie-break; the sort and the row's own scaffolding (stack, background button, marker button, caret) carry the microseconds, not the cell text.

The harness cannot settle a difference below a few percent. Within one binary, `fastest` of two runs agrees within 1 %, but semantically equal code rebuilt twice differed by up to 10 % in one scenario: the sorted-by-size run measured 47.9–48.0 ms before that cabinet change and 51.6–53.5 ms in three variants of it, with nothing in that path changed semantically. A change counts as real only when one binary repeats it and the affected scenario moves more than the untouched ones do — the hidden-cell change did not meet that bar and was left out.

## What iced 0.14 already does

- Drawing off-screen rows is already skipped: `column::draw` keeps only children whose bounds intersect the viewport (`iced_widget-0.14.2/src/column.rs`: `.filter(|(_, layout)| layout.bounds().intersects(viewport))`). Building and laying out every row is what our measurement covers, so that filtering cannot help us.
- The built-in `table` builds one `Element` per cell for every row up front (`iced_widget-0.14.2/src/table.rs`: `Table::new` fills `cells: Vec<Element>` with capacity `columns × rows`), and has no selection or state API. `grid` is the same shape (`grid.rs`: `children: Vec<Element>`).
- `Lazy` rebuilds its content only when a hashed `Dependency` changes, and is behind an opt-in feature (`iced-0.14.0/Cargo.toml`: `lazy = ["iced_widget/lazy"]`, not in `default`). Its content must be `Element<'static>` (`iced_widget-0.14.2/src/lazy.rs`: `View: Into<Element<'static, …>>`), so rows borrowing `&Browser` need owned or cloned data to go in. The crash reported in [iced#3383](https://github.com/iced-rs/iced/issues/3383) (`size` before `diff`) is on master, closed 2026-07-08 under milestone 0.15.
- The parts for hand-built windowing are there: `scrollable::on_scroll(Fn(Viewport) -> Message)` (`iced_widget-0.14.2/src/scrollable.rs`: `on_scroll`, `Viewport::absolute_offset`, `relative_offset`), the `sensor` widget (`on_show` / `on_hide` / `anticipate` / `delay`), and `keyed::Column` for state continuity across a changing window. `keyed` keeps state; it does not build fewer rows.

## No virtualised list upstream

- Nothing named `Recycle` exists in the 0.14 crates, in the full iced clone (6561 commits, branch `android-0.14`), or in iced's history (`git log -S "trait Recycle"`). Upstream `widget/src` on master has no list/virtual/recycle module ([contents API](https://api.github.com/repos/iced-rs/iced/contents/widget/src?ref=master), read 2026-10-27).
- The tracking issue is [iced#160 `InfiniteList` widget](https://github.com/iced-rs/iced/issues/160): open since 2020-01-15, labels `feature`, `help wanted`, last touched 2026-01-25. Its last substantive comments are from 2024-10: the branch `feature/list-widget-reloaded` "works pretty well" in a third-party app. That branch is 9 commits ahead and 2437 behind master ([compare API](https://api.github.com/repos/iced-rs/iced/compare/master...feature/list-widget-reloaded)); the app that praised it (nednoodlehead/punge) now depends on released iced 0.14.0, so nothing there proves the branch works on 0.14.
- `discourse.iced.rs` does not resolve from this machine (DNS), so the "virtualized lists" thread there was not read.

## COSMIC: the one real windowed list in this ecosystem

- [pop-os/libcosmic](https://github.com/pop-os/libcosmic) carries its iced fork as a submodule at `iced/` → [pop-os/iced](https://github.com/pop-os/iced), path dependency (`[dependencies.iced] path = "./iced"`). That fork has `widget/src/list.rs` (`pub mod list; pub use list::List;`), a genuine windowed list: state holds `visible_layouts: Vec<(usize, layout::Node, Tree)>` and per-item `offsets`, and `layout`/`build` binary-search the viewport to pick the first and last visible item. It came in 2024-05-04 from iced's `feature/list-widget-reloaded` branch; one later commit touches it ("Fix List widget not updating children because iterator not consumed", 2026-06-10).
- It is not published: crates.io lists only normal `iced` versions (newest 0.14.0), and the fork's `iced_widget` is 0.14.2. Reaching it means a git/path dependency or `[patch.crates-io]` — which our `Cargo.toml` already allows for apps, but the crate itself cannot name `list::List` while also compiling against crates.io 0.14.
- COSMIC's own list widgets do not window: `libcosmic/src/widget/list/list_column.rs` collects `children: Vec<ListEntry>` and pushes one column entry per item, and cosmic-files builds plain columns.

## Third-party crates on iced 0.14

- `iced_table` (tarkah, 0.14.0, May 2026, MIT, ~108 downloads/month) and `iced_table2` (0.14.2, March 2026, GPL-3.0, ~26 downloads/month): header, body and footer scrollables kept in sync, row-press and header-press callbacks, column resize from caller-tracked offsets. Every cell is an `Element` each frame; `iced_table2`'s GPL licence rules it out for this MIT crate regardless.
- `oxiui-table` (cool-japan/oxiui, Apache-2.0, 0.2.2, August 2026, 361 downloads): a `RowSource` trait with viewport windowing ("only rows visible in the current scroll window plus a small overscan are materialized per frame"), height cache, pagination, sort and selection, and an iced backend. Its iced backend takes a caller-tracked `scroll_offset: usize` and its module documentation says iced exposes no scroll offset — that part is wrong for 0.14.2, which has `scrollable::on_scroll`. It renders flat rows, no tree, and the whole project is a facade over egui and iced with nine stars.
- `iced-flatlist` (luizvidoto): last push 2023-04-14, 8 stars, no releases — predates 0.14.
- `virtualizer` (Latias94, 0.4.0, January 2026, ~93 downloads/month): TanStack-style index math, no iced integration. Useful only if rows were windowed by hand.
- `wgpu-atoms`: not found. crates.io search and the direct crate lookup return nothing, `github.com/pop-os/wgpu-atoms` is 404, and neither `libcosmic`'s nor `cosmic-files`' `Cargo.toml` mentions it.

## Options for the per-frame cost, as they stand

- Cache the built list: a revision counter in `Browser` bumped by anything that changes the rows, with the list inside `Lazy`. Needs the rows to stop borrowing `&Browser`.
- Window the rows: `scrollable::on_scroll` for the offset, spacer quads above and below, `keyed::Column` so state survives the moving window. Needs one row height and changes the "one `Element` per entry" model.
- Keep building every row and keep listing cheap in the app: cap, paginate, or collapse by default.
- Build against the COSMIC fork as a variant, where a windowed list already exists.
