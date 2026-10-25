//! iced File cABInet — a widget for browsing directories and selecting items.
//!
//! Two views over one state ([`Browser`]):
//!
//! - [`Mode::Browse`]: one directory per screen with a breadcrumb bar, Android Documents-App style.
//! - [`Mode::Tree`]: the whole tree in place, every branch expandable — selection can span several levels at once.
//!
//! Material-style selection on every row: a box, a tick when the item is selected as a unit, and a dot when something inside it is selected ([`Marker`]); the header's box selects or clears the whole list. The listings themselves come from the application: the widget only says which nodes need loading ([`Outcome::NeedsLoad`]), the app answers with [`Browser::set_children`] — from disk, rclone, or an HTTP API.
//!
//! The widget is configured through the [`Cabinet`] builder: toolbar [`Tool`]s in groups ([`Cabinet::tool_separator`] between them, besides the built-in mode switch), a text [`action_button`](Cabinet::action_button) for the selection, custom [`Column`]s (registered on the [`Browser`]), a [`Style`] hook so an app's own theme (its palette, or a fork's) drives every colour, and a [`RowIcons`] hook for the file/folder glyphs (its own icon pack, tinted or multicolour).
//!
//! # Where what is configured
//!
//! [`Browser`] holds the state that persists across frames — what the app wants to show and how selection behaves:
//!
//! - Content: [`set_mode`](Browser::set_mode) (browse or tree), [`set_folders_only`](Browser::set_folders_only), [`set_file_extensions`](Browser::set_file_extensions) / [`show_all_files`](Browser::show_all_files), custom [`add_column`](Browser::add_column)s, and the listings themselves via [`set_children`](Browser::set_children).
//! - Selection: [`set_selection_mode`](Browser::set_selection_mode) (one entry at a time or many), [`set_selection_ui`](Browser::set_selection_ui) (view-only, long-press-to-reveal, or the normal boxes), and the operations [`tap`](Browser::tap), [`toggle`](Browser::toggle), [`toggle_all`](Browser::toggle_all), [`clear_selection`](Browser::clear_selection), [`remove_selected`](Browser::remove_selected).
//!
//! [`Cabinet`] is rebuilt on every draw from [`cabinet`] and carries the look of that one draw: the [`checkboxes`](Cabinet::checkboxes) column, the toolbar [`tool`](Cabinet::tool)s, the [`action_button`](Cabinet::action_button) and where it sits ([`action_placement`](Cabinet::action_placement)), [`separators`](Cabinet::separators), [`style`](Cabinet::style), [`icons`](Cabinet::icons), [`text_size`](Cabinet::text_size), [`font`](Cabinet::font) and [`scrollbar`](Cabinet::scrollbar).
//!
//! The widget talks to the app in two directions only: it emits [`Event`]s (in the app's `update`, each maps to the [`Browser`] method that applies it), and it receives listings plus the [`Outcome`]s of [`tap`](Browser::tap)/[`activate`](Browser::activate) telling it when to fetch or open.
//!
//! ```no_run
//! use iced_fabi::{Browser, Event, file_cabinet};
//!
//! #[derive(Clone)]
//! enum Message { Cabinet(Event) }
//!
//! let browser = Browser::new("My files");
//! let _element = file_cabinet::<Message, iced::Renderer>(&browser, &Message::Cabinet);
//! ```

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
    Browser, Entry, EntryKind, FormatModified, FormatSize, LONG_PRESS, Marker, Mode, Node, NodeId,
    Outcome, SelectionMode, SelectionUi,
};
pub use cabinet::{ActionPlacement, Cabinet, Separators, cabinet, file_cabinet};
pub use column::{Cell, Column, ColumnWidth, ModifiedColumn, NameColumn, SizeColumn};
pub use event::Event;
pub use format::{format_bytes, format_epoch};
pub use icon::{RowIcons, TablerIcons};
pub use style::{Style, default_style};
pub use tool::{SimpleTool, Tool, tool};
