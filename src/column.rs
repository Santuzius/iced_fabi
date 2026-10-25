//! Columns of a listing row: the [`Column`] trait an app implements to add its own, the built-in Name/Modified/Size columns, and what a cell may contain.

use std::{cmp::Ordering, fmt};

use crate::{browser::Browser, browser::node::Node, natural::name_key};

/// How a column asks for its share of the row width.
#[derive(Debug, Clone, Copy)]
pub enum ColumnWidth {
    /// A single flexible share — the same as `Portion(1)`.
    Fill,
    /// Share of the leftover width, relative to the other portioned columns.
    Portion(u16),
    Fixed(f32),
}

/// What one column shows for one node.
#[derive(Debug, Clone)]
pub enum Cell {
    Empty,
    Text {
        text: String,
        muted: bool,
    },
    Glyph(icondata::Icon),
    Both {
        glyph: icondata::Icon,
        text: String,
        muted: bool,
    },
}

/// One column of the row: a header, a cell per node and — optionally — a sort order. Implement this to add columns (file type, owner, cloud provider, sync state …).
///
/// The Name column (index 0) is built in and rendered specially (flexible width, clickable, folder/file icon); custom columns render their [`Cell`] as-is.
pub trait Column: fmt::Debug {
    /// Header label, clicked to sort by this column.
    fn header(&self) -> String;

    fn width(&self) -> ColumnWidth {
        ColumnWidth::Portion(2)
    }

    /// The cell for this node.
    fn cell(&self, browser: &Browser, node: &Node) -> Cell;

    /// Total order for sorting; [`Ordering::Equal`] means the column is not sortable (headers then still sort stably by name).
    fn compare(&self, _a: &Node, _b: &Node) -> Ordering {
        Ordering::Equal
    }
}

/// The built-in Name column (index 0): the flexible, clickable one.
#[derive(Debug)]
pub struct NameColumn;

impl Column for NameColumn {
    fn header(&self) -> String {
        "Name".into()
    }

    fn width(&self) -> ColumnWidth {
        // The largest share by default, as in Proton Drive and KDE Dolphin: the name is what a file list is for, and in tree mode the indent eats into this column.
        ColumnWidth::Portion(4)
    }

    fn cell(&self, _browser: &Browser, node: &Node) -> Cell {
        Cell::Text {
            text: node.name.clone(),
            muted: false,
        }
    }

    fn compare(&self, a: &Node, b: &Node) -> Ordering {
        name_key(&a.name).cmp(&name_key(&b.name))
    }
}

/// The built-in modification-time column, rendered with [`format_modified`](Browser::format_modified).
#[derive(Debug)]
pub struct ModifiedColumn;

impl Column for ModifiedColumn {
    fn header(&self) -> String {
        "Modified".into()
    }

    fn cell(&self, browser: &Browser, node: &Node) -> Cell {
        Cell::Text {
            text: node
                .modified
                .map(|e| browser.format_modified(e))
                .unwrap_or_default(),
            muted: true,
        }
    }

    fn compare(&self, a: &Node, b: &Node) -> Ordering {
        a.modified
            .unwrap_or(i64::MIN)
            .cmp(&b.modified.unwrap_or(i64::MIN))
    }
}

/// The built-in size column, rendered with [`format_size`](Browser::format_size).
#[derive(Debug)]
pub struct SizeColumn;

impl Column for SizeColumn {
    fn header(&self) -> String {
        "Size".into()
    }

    fn cell(&self, browser: &Browser, node: &Node) -> Cell {
        Cell::Text {
            text: node
                .size
                .map(|s| browser.format_size(s))
                .unwrap_or_default(),
            muted: true,
        }
    }

    fn compare(&self, a: &Node, b: &Node) -> Ordering {
        a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0))
    }
}
