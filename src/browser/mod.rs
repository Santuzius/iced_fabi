//! The state behind [`Cabinet`](crate::Cabinet): a keyed tree of nodes that the application fills from wherever its listings come from (local disk, rclone, an HTTP API), plus breadcrumb navigation, columns, sorting and Material-style selection.
//!
//! The application owns a [`Browser`], sends it [`Event`](crate::Event)s from the widget's buttons and feeds new listings back with [`set_children`](Browser::set_children). The [`Browser`] never touches the filesystem or the network itself; [`activate`](Browser::activate) tells it when data is missing ([`Outcome::NeedsLoad`]) so the app can fetch it asynchronously and stay in control of staleness (drop the response if the node was pruned meanwhile — [`set_children`](Browser::set_children) then does nothing).
//!
//! This module owns the state and its settings; one file per topic adds its own `impl Browser` block, so the fields stay private to `browser` while its parts can all work on them.

pub mod node;

mod loading;
mod long_press;
mod navigation;
pub(crate) mod rows;
mod selection;
#[cfg(test)]
mod tests;

pub use long_press::LONG_PRESS;
pub use node::{Checkboxes, Entry, EntryKind, Marker, Mode, Node, NodeId, Selection};

use std::collections::{HashMap, HashSet};

use crate::{
    column::{Column, ColumnKey, ModifiedColumn, NameColumn, SizeColumn},
    event::Event,
    tool::ToolKey,
};

/// The consequence of a [`Browser`] action, telling the application what it must do next.
#[must_use = "the outcome says what the application must do next"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing happened (or a change the calling code already accounts for, e.g. a re-render).
    None,
    /// The app should list this node's children now; [`loading`](Browser::loading) is set until [`set_children`](Browser::set_children) or [`load_failed`](Browser::load_failed) answers it.
    NeedsLoad(NodeId),
    /// A file was opened (a press that did not navigate or tick) and the app should decide what that means (open it in a viewer, download, …).
    FileOpened(NodeId),
    /// The selection grew or shrank: re-evaluate anything keyed on it (an action button, a count line).
    SelectionChanged,
}

/// The state of one file-browser screen: tree, navigation, columns, selection, sorting, mode.
pub struct Browser {
    // Internal to the `browser` module: every topic file works on these directly; what the app may see goes through the methods below and in the topic files.
    pub(super) nodes: HashMap<NodeId, Node>,
    /// Breadcrumb from the root down to [`current`](Self::current).
    pub(super) path: Vec<NodeId>,
    pub(super) current: NodeId,
    /// Nodes whose branch is open in tree mode. The path to the current node is kept expanded so browse→tree shows the same place.
    pub(super) expanded: HashSet<NodeId>,
    /// Explicit unit selections; a node selected here is not selected again below it, and [`toggle`](Self::toggle) refuses to select under a selected ancestor.
    pub(super) selected: HashSet<NodeId>,
    /// How many entries can be selected at once (or none at all).
    pub(super) selection: Selection,
    /// How the selection boxes appear.
    pub(super) checkboxes: Checkboxes,
    /// Long-press bookkeeping: the row currently held and when the hold started.
    pub(super) held: Option<(NodeId, std::time::Instant)>,
    /// Whether a long press has revealed the boxes ([`OnLongPress`](crate::Checkboxes::OnLongPress)).
    pub(super) armed: bool,
    /// Index into [`columns`](Self::columns); index 0 is the built-in [`NameColumn`].
    pub(super) sort_column: usize,
    pub(super) sort_ascending: bool,
    /// Folders sort before files regardless of the key, as file managers usually do.
    pub(super) folders_first: bool,
    /// Hide file rows and list folders only.
    pub(super) folders_only: bool,
    /// When set, a file row appears only if its extension (after the last dot, case-insensitive) is in this list; folders are unaffected. `None` lists every file.
    pub(super) extensions: Option<Vec<String>>,
    pub(super) mode: Mode,
    pub(super) columns: Vec<Box<dyn Column>>,
    /// Parallel to [`columns`](Self::columns): a column whose flag is set is not drawn — header cell, body cells and its upright rule all stop; index 0 is never hidden.
    pub(super) hidden: Vec<bool>,
    /// Whether the app's clipboard holds something: only the app knows (the library carries no file data), and this flag is its answer, driving the built-in [`Paste`](crate::BuiltInTool::Paste).
    pub(super) has_clipboard: bool,
    /// Ids of tools taken out of the toolbar ([`hide_tool`](Self::hide_tool)); an empty tool id is never in here, so anonymous tools always draw.
    pub(super) hidden_tools: HashSet<String>,
}

// Boxed columns are not Debug, so Browser cannot derive it.
impl std::fmt::Debug for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Browser")
            .field("nodes", &self.nodes)
            .field("path", &self.path)
            .field("current", &self.current)
            .field("expanded", &self.expanded)
            .field("selected", &self.selected)
            .field("selection", &self.selection)
            .field("checkboxes", &self.checkboxes)
            .field("armed", &self.armed)
            .field("sort_column", &self.sort_column)
            .field("sort_ascending", &self.sort_ascending)
            .field("folders_first", &self.folders_first)
            .field("folders_only", &self.folders_only)
            .field("extensions", &self.extensions)
            .field("mode", &self.mode)
            .field("columns", &self.columns.len())
            .field("hidden", &self.hidden)
            .field("has_clipboard", &self.has_clipboard)
            .field("hidden_tools", &self.hidden_tools)
            .finish()
    }
}

impl Browser {
    /// A browser whose root is a folder named `root`, not listed yet: [`open`](Browser::open) asks the app for the root's children. The columns start as Name, Modified, Size.
    pub fn new(root: impl Into<String>) -> Self {
        let root_name = root.into();
        let node = Node {
            id: NodeId::ROOT,
            name: root_name,
            kind: EntryKind::Folder,
            parent: None,
            children: Vec::new(),
            children_known: false,
            loading: false,
            failed: false,
            size: None,
            modified: None,
        };
        Browser {
            nodes: HashMap::from([(NodeId::ROOT, node)]),
            path: vec![NodeId::ROOT],
            current: NodeId::ROOT,
            expanded: HashSet::from([NodeId::ROOT]),
            selected: HashSet::new(),
            selection: Selection::Multiple,
            checkboxes: Checkboxes::Shown,
            held: None,
            armed: false,
            sort_column: 0,
            sort_ascending: true,
            folders_first: true,
            folders_only: false,
            extensions: None,
            mode: Mode::Browse,
            columns: vec![
                Box::new(NameColumn::new()),
                Box::new(ModifiedColumn::new()),
                Box::new(SizeColumn::new()),
            ],
            hidden: vec![false; 3],
            has_clipboard: false,
            hidden_tools: HashSet::new(),
        }
    }

    /// Start the browser on its root: everything is configured, now ask for the root listing. On a fresh browser this always returns [`NeedsLoad`](Outcome::NeedsLoad)([`NodeId::ROOT`]) — act on it before the first draw, e.g. `Task::done` a fetch message in `main`. It is a plain `&mut self` method, so it fits both a configuration chain (`browser` built first, then `let outcome = browser.open();`) and re-opening an existing browser.
    pub fn open(&mut self) -> Outcome {
        self.request_load(NodeId::ROOT)
    }

    /// Apply one [`Event`] from the widget and report what the application must do next. This is the normal entry point: a single call in `update`, no routing knowledge on the app side.
    ///
    /// The mapping it hides: a row press goes to [`tap`](Self::tap) (folders navigate, files tick while the boxes are shown or report [`FileOpened`](Outcome::FileOpened) while they are not), a tree caret to [`activate`](Self::activate), the retry note to a fresh load request, a short press under the long-press mode to the row's normal press (the widget's press layer hides the row's own events until the boxes appear), and every box to the matching selection operation.
    pub fn update(&mut self, event: Event) -> Outcome {
        match event {
            Event::RowPressed(id) => self.tap(id),
            Event::CaretPressed(id) => self.activate(id),
            Event::RetryPressed(id) => self.retry(id),
            Event::MarkerPressed(id) => {
                self.toggle(id);
                Outcome::SelectionChanged
            }
            Event::HeaderMarkerPressed => {
                self.toggle_all();
                Outcome::SelectionChanged
            }
            Event::CrumbPressed(id) => self.goto(id),
            Event::SortPressed(column) => {
                self.sort_by_column(column);
                Outcome::None
            }
            Event::ModeSwitchPressed => {
                self.toggle_mode();
                Outcome::None
            }
            Event::HoldStarted(id) => {
                self.hold_started(id);
                Outcome::None
            }
            Event::HoldEnded(id) => {
                if self.hold_ended(id) {
                    // A revealing hold armed the boxes and ticked the held row along the way.
                    Outcome::SelectionChanged
                } else {
                    // Short press: the widget's press layer replaced the row button's events, so this is the row's only press.
                    self.tap(id)
                }
            }
        }
    }

    /// Root listing of one folder and one file, for the topic tests.
    #[cfg(test)]
    pub(crate) fn sample() -> Browser {
        let mut browser = Browser::new("My files");
        browser.set_children(
            NodeId::ROOT,
            vec![
                Entry::folder("Finanzen").modified(1_745_000_000),
                Entry::file("notes.txt").size(900).modified(1_745_000_100),
            ],
        );
        browser
    }

    /// Append a custom column; returns its index (for [`sort_by_column`](Self::sort_by_column)). Added columns start visible.
    pub fn add_column(&mut self, column: impl Column + 'static) -> usize {
        self.columns.push(Box::new(column));
        self.hidden.push(false);
        self.columns.len() - 1
    }

    /// The columns in display order: index 0 is the built-in Name column, then the [set](Self::set_columns) / [added](Self::add_column) ones.
    pub fn columns(&self) -> &[Box<dyn Column>] {
        &self.columns
    }

    /// The column the rows are ordered by, and whether that order is ascending.
    pub fn sort(&self) -> (usize, bool) {
        (self.sort_column, self.sort_ascending)
    }

    /// Click on a sort header: same column flips the direction, a new column starts ascending.
    pub fn sort_by_column(&mut self, column: usize) {
        if column >= self.columns.len() {
            return;
        }
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
    }

    /// Replace every column but the built-in Name column at index 0 (which stays, so sorting, the breadcrumb and the rows keep their anchor); pass boxed [`BuiltInColumn`](crate::BuiltInColumn) variants for the defaults, boxed [`SizeColumn::new`] / [`ModifiedColumn::new`] instances to configure headers and renderers, or boxed custom [`Column`]s. The new list starts fully visible.
    pub fn set_columns(&mut self, columns: impl IntoIterator<Item = Box<dyn Column>>) {
        self.columns.truncate(1);
        self.columns.extend(columns);
        self.hidden.truncate(1);
        self.hidden.resize(self.columns.len(), false);
        if self.sort_column >= self.columns.len() {
            self.sort_column = 0;
        }
    }

    /// Take a column out of the drawing: its header cell, every body cell and its upright rules stop being painted and the remaining columns claim its width. The column itself stays in [`columns`](Self::columns) — its index and sorting by it keep working — only the painting stops. Index 0 (the Name column) is always visible; an out-of-range index does nothing.
    pub fn hide_column(&mut self, column: usize) {
        if column == 0 || column >= self.hidden.len() {
            return;
        }
        self.hidden[column] = true;
    }

    /// Put a [hidden](Self::hide_column) column back into the drawing.
    pub fn show_column(&mut self, column: usize) {
        if column < self.hidden.len() {
            self.hidden[column] = false;
        }
    }

    /// Whether this column is hidden from the drawing (out of range: false).
    pub fn is_column_hidden(&self, column: usize) -> bool {
        self.hidden.get(column).copied().unwrap_or(false)
    }

    /// Show these columns and only these; every other one is hidden (the Name column at index 0 is never). The entries are [`ColumnKey`]s, and [`ColumnKey::from`] turns any [`Column`] value into one: built-ins as `ColumnKey::from(BuiltInColumn::Size)`, a custom column as `ColumnKey::from(TypeColumn)`, matched on the [`id`](Column::id) the columns report. An id that matches no column does nothing.
    ///
    /// ```
    /// # use iced_fabi::{Browser, BuiltInColumn, Cell, Column, ColumnKey};
    /// # struct TypeColumn;
    /// # impl Column for TypeColumn {
    /// #     fn header(&self) -> String {
    /// #         "Type".into()
    /// #     }
    /// #     fn cell(&self, _browser: &Browser, _node: &iced_fabi::Node) -> Cell {
    /// #         Cell::Empty
    /// #     }
    /// # }
    /// let mut browser = Browser::new("My files");
    /// browser.add_column(TypeColumn);
    /// // an app that offers only size and type:
    /// browser.set_visible_columns([ColumnKey::from(BuiltInColumn::Size), ColumnKey::from(TypeColumn)]);
    /// assert!(browser.is_column_hidden(1)); // Modified
    /// assert!(!browser.is_column_hidden(2)); // Size
    /// assert!(!browser.is_column_hidden(3)); // Type
    /// ```
    pub fn set_visible_columns(&mut self, visible: impl IntoIterator<Item = ColumnKey>) {
        let wanted: Vec<String> = visible.into_iter().map(|key| key.id().to_owned()).collect();
        for (index, column) in self.columns.iter().enumerate().skip(1) {
            self.hidden[index] = !wanted.contains(&column.id());
        }
    }

    /// Tell the browser whether the app's clipboard currently holds something: the only thing the library can know about it (the file data itself stays with the app), and what enables the built-in [`Paste`](crate::BuiltInTool::Paste) button.
    pub fn set_has_clipboard(&mut self, has: bool) {
        self.has_clipboard = has;
    }

    /// Whether a [clipboard](Self::set_has_clipboard) was announced.
    pub fn has_clipboard(&self) -> bool {
        self.has_clipboard
    }

    /// Take a named tool out of the toolbar: it stops being drawn wherever the app adds it, and a group left without tools takes its divider with it. Tools are named, not counted: [`ToolKey`] from a [`BuiltInTool`](crate::BuiltInTool) variant or from the [`id`](crate::Tool::id) a custom tool returns. A tool without an id (the [`Tool`](crate::Tool) default) cannot be hidden.
    pub fn hide_tool(&mut self, tool: impl Into<ToolKey>) {
        let key = tool.into();
        if !key.id().is_empty() {
            self.hidden_tools.insert(key.id().to_owned());
        }
    }

    /// Put a [hidden](Self::hide_tool) tool back into the toolbar.
    pub fn show_tool(&mut self, tool: impl Into<ToolKey>) {
        self.hidden_tools.remove(tool.into().id());
    }

    /// Whether the toolbar leaves this tool id out.
    pub fn is_tool_hidden(&self, id: &str) -> bool {
        self.hidden_tools.contains(id)
    }
}
