#![warn(missing_docs)]

//! iced File cABInet — a widget for browsing directories and selecting items.
//!
//! Two views over one state ([`Browser`]):
//!
//! - [`Mode::Browse`]: one directory per screen with a breadcrumb bar, Android Documents-App style.
//! - [`Mode::Tree`]: the whole tree in place, every branch expandable — selection can span several levels at once.
//!
//! Material-style selection on every row: a box, a tick when the item is selected as a unit, and a dot when something inside it is selected ([`Marker`]); the header's box selects or clears the whole list. The listings themselves come from the application: the widget only says which nodes need loading ([`Outcome::NeedsLoad`]), the app answers with [`Browser::set_children`] — from disk, rclone, or an HTTP API.
//!
//! The widget is configured through the [`Cabinet`] builder: toolbar [`Tool`]s in groups ([`Cabinet::built_in_tools`] for the built-in set, [`Cabinet::tool_separator`] between them, besides the built-in mode switch), one [`ActionButton`](Cabinet::action) for the selection, custom [`Column`]s (registered on the [`Browser`]), a [`Style`] hook so an app's own theme (its palette, or a fork's) drives every colour, and a [`RowIcons`] hook for the file/folder glyphs (its own icon pack, tinted or multicolour).
//!
//! # Where what is configured
//!
//! [`Browser`] holds the state that persists across frames — what the app wants to show and how selection behaves:
//!
//! - Content: [`set_mode`](Browser::set_mode) (browse or tree), [`set_folders_only`](Browser::set_folders_only), [`set_file_extensions`](Browser::set_file_extensions) / [`show_all_files`](Browser::show_all_files), the [`columns`](Browser::column_count) ([`column`](Browser::column) reads one, [`add_column`](Browser::add_column) one, [`set_columns`](Browser::set_columns) the set, [`hide_column`](Browser::hide_column) / [`show_column`](Browser::show_column) and [`set_visible_columns`](Browser::set_visible_columns) which of them are drawn, headers and formatters live on the [columns](crate::Column) themselves), and the listings themselves via [`set_children`](Browser::set_children).
//! - Selection: [`set_selection`](Browser::set_selection) (many entries, one, or none at all), [`set_checkboxes`](Browser::set_checkboxes) (always shown, revealed by a long press, or hidden), and the operations [`tap`](Browser::tap), [`toggle`](Browser::toggle), [`toggle_all`](Browser::toggle_all), [`clear_selection`](Browser::clear_selection), [`remove_selected`](Browser::remove_selected).
//!
//! [`Cabinet`] is rebuilt on every draw from [`cabinet`] and carries the look of that one draw: the selection settings ([`crate::Selection`] and [`crate::Checkboxes`]) on the [`Browser`], the toolbar [`tool`](Cabinet::tool)s and the built-in [`BuiltInTool`] set ([`built_in_tools`](Cabinet::built_in_tools), [`Browser::hide_tool`] / [`Browser::show_tool`] switch named ones out and [`Browser::set_visible_tools`] sets the whole list at once), the [`action`](Cabinet::action) button ([`ActionButton`]), [`separators`](Cabinet::separators), [`style`](Cabinet::style), [`icons`](Cabinet::icons), [`text_size`](Cabinet::text_size), [`font`](Cabinet::font) and [`scrollbar`](Cabinet::scrollbar).
//!
//! The widget talks to the app in two directions only: it emits [`Event`]s — hand each one to [`Browser::update`] in your `update`, and the returned [`Outcome`] says what to do next (fetch, open, create, run a confirmed tool, or nothing) — and it receives listings via [`set_children`](Browser::set_children). Start-up: after configuring the browser, call [`open`](Browser::open) and act on its [`Outcome::NeedsLoad`] before the first draw.
//!
//! # The whole integration, three steps
//!
//! ```no_run
//! use iced::{Element, Task};
//! use iced_fabi::{Browser, Entry, Event, NodeId, Outcome};
//!
//! #[derive(Clone)]
//! enum Message {
//!     Cabinet(Event),
//!     /// A listing fetched for the node behind a [`NeedsLoad`](Outcome::NeedsLoad).
//!     Listed(NodeId, Vec<Entry>),
//! }
//!
//! struct App {
//!     browser: Browser,
//! }
//!
//! // 1. set-up: state, behaviour, then `open()` for the first listing request.
//! impl App {
//!     fn boot() -> (Self, Task<Message>) {
//!         let mut browser = Browser::new("My files");
//!         let task = match browser.open() {
//!             // here the app would start its fetch and send the entries back as Message::Listed.
//!             Outcome::NeedsLoad(id) => Task::done(Message::Listed(id, Vec::new())),
//!             _ => Task::none(),
//!         };
//!         (App { browser }, task)
//!     }
//!
//!     // 2. update: one call, and the Outcome says what the app must do.
//!     fn update(&mut self, message: Message) -> Task<Message> {
//!         match message {
//!             Message::Cabinet(event) => match self.browser.update(event) {
//!                 Outcome::NeedsLoad(id) => Task::done(Message::Listed(id, Vec::new())), // fetch
//!                 // A real app would open the file and create the folder it is asked for here.
//!                 Outcome::FileOpened(_)
//!                 | Outcome::CreateFolderRequested { .. }
//!                 | Outcome::ToolConfirmed { .. }
//!                 | Outcome::SelectionChanged
//!                 | Outcome::None => Task::none(),
//!             },
//!             Message::Listed(id, entries) => {
//!                 self.browser.set_children(id, entries);
//!                 Task::none()
//!             }
//!         }
//!     }
//!
//!     // 3. view: one builder chain per frame; `.into()` is the Element.
//!     fn view(&self) -> Element<'_, Message> {
//!         iced_fabi::cabinet(&self.browser, Message::Cabinet).into()
//!     }
//! }
//!
//! fn main() -> iced::Result {
//!     iced::application(App::boot, App::update, App::view).run()
//! }
//! ```
//!
//! A complete runnable app is [`examples/minimal.rs`](https://github.com/Santuzius/iced_fabi/blob/main/examples/minimal.rs) (local disk via `std::fs`); the full feature parade is `examples/cloud_browse.rs`.

mod browser;
mod cabinet;
mod column;
mod event;
mod format;
mod icon;
mod natural;
mod style;
mod tool;

pub use browser::{
    Browser, Checkboxes, Entry, EntryKind, LONG_PRESS, Marker, Mode, Node, NodeId, Outcome,
    Selection,
};
pub use cabinet::{ActionButton, ActionPlacement, Cabinet, Labels, Separators, cabinet};
pub use column::{
    BuiltInColumn, Cell, Column, ColumnKey, ColumnWidth, ModifiedColumn, NameColumn, SizeColumn,
};
pub use event::Event;
pub use format::{format_bytes, format_epoch};
pub use icon::{RowIcons, TablerIcons};
pub use style::Style;
pub use tool::{BuiltInTool, ConfirmTone, SimpleTool, Tool, ToolConfirmation, ToolKey, tool};
