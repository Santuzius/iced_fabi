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
    /// A fixed pixel width.
    Fixed(f32),
}

/// What one column shows for one node.
#[derive(Debug, Clone)]
pub enum Cell {
    /// Nothing in this cell.
    Empty,
    /// A text cell; `muted` renders it in the palette's muted colour (the default for the value columns).
    Text {
        /// The text to show.
        text: String,
        /// Render in the palette's muted colour.
        muted: bool,
    },
    /// Just a glyph.
    Glyph(icondata::Icon),
    /// A glyph followed by text.
    Both {
        /// The leading glyph.
        glyph: icondata::Icon,
        /// The text after the glyph.
        text: String,
        /// Render the text in the palette's muted colour.
        muted: bool,
    },
}

impl Cell {
    /// The cell's text, if it has any (tests and apps poking at custom columns' output).
    pub fn to_text(&self) -> Option<&str> {
        match self {
            Cell::Empty | Cell::Glyph(_) => None,
            Cell::Text { text, .. } | Cell::Both { text, .. } => Some(text),
        }
    }
}

/// One column of the row: a header, a cell per node and — optionally — a sort order. Implement this to add columns (file type, owner, cloud provider, sync state …).
///
/// The Name column (index 0) is built in and rendered specially (flexible width, clickable, folder/file icon); custom columns render their [`Cell`] as-is.
pub trait Column {
    /// Header label, clicked to sort by this column.
    fn header(&self) -> String;

    /// How the column claims its share of the row width. Default: [`Portion(2)`](ColumnWidth::Portion).
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

/// The built-in Name column (index 0): the flexible, clickable one. Its header is "Name" unless set otherwise.
#[derive(Debug)]
pub struct NameColumn {
    header: Option<String>,
}

impl Default for NameColumn {
    fn default() -> Self {
        Self::new()
    }
}

impl NameColumn {
    /// The built-in flexible, clickable Name column.
    #[must_use]
    pub fn new() -> Self {
        Self { header: None }
    }

    /// Replace the header label (the first column header in [`headers`](crate::Browser::columns) order).
    pub fn with_header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }
}

impl Column for NameColumn {
    fn header(&self) -> String {
        self.header.clone().unwrap_or_else(|| "Name".into())
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

/// The built-in modification-time column, rendering each node through its [`format`](Self::format) closure (default [`format_epoch`](crate::format_epoch)).
pub struct ModifiedColumn {
    header: Option<String>,
    format: Box<dyn Fn(i64) -> String>,
}

impl Default for ModifiedColumn {
    fn default() -> Self {
        Self::new()
    }
}

impl ModifiedColumn {
    /// The modification-time column, default header and renderer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            header: None,
            format: Box::new(crate::format::format_epoch),
        }
    }

    /// Replace the header label.
    pub fn with_header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Replace the renderer, e.g. to localise: the closure gets the raw epoch seconds and may capture a locale.
    pub fn format(mut self, f: impl Fn(i64) -> String + 'static) -> Self {
        self.format = Box::new(f);
        self
    }
}

impl Column for ModifiedColumn {
    fn header(&self) -> String {
        self.header.clone().unwrap_or_else(|| "Modified".into())
    }

    fn cell(&self, _browser: &Browser, node: &Node) -> Cell {
        Cell::Text {
            text: node.modified.map(|e| (self.format)(e)).unwrap_or_default(),
            muted: true,
        }
    }

    fn compare(&self, a: &Node, b: &Node) -> Ordering {
        a.modified
            .unwrap_or(i64::MIN)
            .cmp(&b.modified.unwrap_or(i64::MIN))
    }
}

/// The built-in size column, rendering each node through its [`format`](Self::format) closure (default [`format_bytes`](crate::format_bytes)).
pub struct SizeColumn {
    header: Option<String>,
    format: Box<dyn Fn(u64) -> String>,
}

impl Default for SizeColumn {
    fn default() -> Self {
        Self::new()
    }
}

impl SizeColumn {
    /// The size column, default header and renderer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            header: None,
            format: Box::new(crate::format::format_bytes),
        }
    }

    /// Replace the header label.
    pub fn with_header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Replace the renderer, e.g. to localise or to use 1024-based units: the closure gets the raw byte count and may capture a locale.
    pub fn format(mut self, f: impl Fn(u64) -> String + 'static) -> Self {
        self.format = Box::new(f);
        self
    }
}

impl Column for SizeColumn {
    fn header(&self) -> String {
        self.header.clone().unwrap_or_else(|| "Size".into())
    }

    fn cell(&self, _browser: &Browser, node: &Node) -> Cell {
        Cell::Text {
            text: node.size.map(|s| (self.format)(s)).unwrap_or_default(),
            muted: true,
        }
    }

    fn compare(&self, a: &Node, b: &Node) -> Ordering {
        a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0))
    }
}

// The formatter closures are not Debug, so neither column can derive it.
impl fmt::Debug for ModifiedColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifiedColumn")
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for SizeColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SizeColumn")
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}
