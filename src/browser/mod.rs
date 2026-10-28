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
    tool::{BuiltInTool, ToolKey},
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
    /// Explicit unit selections, in selection order; a node selected here is not selected again below it, and [`toggle`](Self::toggle) refuses to select under a selected ancestor.
    pub(super) selected: selection::Selected,
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
    /// The [`id`](Column::id) behind [`sort_column`](Self::sort_column); [`set_columns`](Self::set_columns) re-finds the index by this id, because a replacement list can move or drop the sorted column.
    pub(super) sort_id: String,
    pub(super) sort_ascending: bool,
    /// Folders sort before files regardless of the key, as file managers usually do.
    pub(super) folders_first: bool,
    /// Hide file rows and list folders only.
    pub(super) folders_only: bool,
    /// When set, a file row appears only if its extension (after the last dot, case-insensitive) is in this list; folders are unaffected. `None` lists every file.
    pub(super) extensions: Option<Vec<String>>,
    pub(super) mode: Mode,
    /// The columns in display order, each with whether the cabinet draws it: index 0 is the built-in Name column and its `hidden` stays false. Hiding stops the header cell, the body cells and the upright rule; the column keeps its index, so sorting by it keeps working.
    pub(super) columns: Vec<ColumnSlot>,
    /// Whether the app's clipboard holds something: only the app knows (the library carries no file data), and this flag is its answer, driving the built-in [`Paste`](crate::BuiltInTool::Paste).
    pub(super) has_clipboard: bool,
    /// Ids of tools taken out of the toolbar ([`hide_tool`](Self::hide_tool)); a tool without an [`id`](crate::Tool::id) can never match one of these, so it always draws.
    pub(super) hidden_tools: HashSet<String>,
}

/// One column of the listing and whether the cabinet draws it. The column and its visibility travel together so an index always means the same column; index 0 is the built-in Name column, whose `hidden` stays false.
pub(super) struct ColumnSlot {
    pub column: Box<dyn Column>,
    pub hidden: bool,
}

impl ColumnSlot {
    /// A listed column that is drawn.
    fn shown(column: impl Column + 'static) -> Self {
        Self {
            column: Box::new(column),
            hidden: false,
        }
    }
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
            .field("sort_id", &self.sort_id)
            .field("sort_ascending", &self.sort_ascending)
            .field("folders_first", &self.folders_first)
            .field("folders_only", &self.folders_only)
            .field("extensions", &self.extensions)
            .field("mode", &self.mode)
            .field(
                "columns",
                &self
                    .columns
                    .iter()
                    .map(|slot| (slot.column.header(), slot.hidden))
                    .collect::<Vec<_>>(),
            )
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
            selected: selection::Selected::default(),
            selection: Selection::Multiple,
            checkboxes: Checkboxes::Shown,
            held: None,
            armed: false,
            sort_column: 0,
            sort_id: NameColumn::new().id(),
            sort_ascending: true,
            folders_first: true,
            folders_only: false,
            extensions: None,
            mode: Mode::Browse,
            columns: vec![
                ColumnSlot::shown(NameColumn::new()),
                ColumnSlot::shown(ModifiedColumn::new()),
                ColumnSlot::shown(SizeColumn::new()),
            ],
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
        self.columns.push(ColumnSlot::shown(column));
        self.columns.len() - 1
    }

    /// How many columns the listing has: index 0 is the built-in Name column, then the [set](Self::set_columns) / [added](Self::add_column) ones.
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// The column at `index` in display order ([`column_count`](Self::column_count) counts them), or `None` past the end. Whether it is drawn is the separate question [`is_column_hidden`](Self::is_column_hidden) answers.
    pub fn column(&self, index: usize) -> Option<&dyn Column> {
        self.columns.get(index).map(|slot| slot.column.as_ref())
    }

    /// The column the rows are ordered by, and whether that order is ascending.
    pub fn sort(&self) -> (usize, bool) {
        (self.sort_column, self.sort_ascending)
    }

    /// Click on a sort header: same column flips the direction, a new column starts ascending.
    pub fn sort_by_column(&mut self, column: usize) {
        let Some(slot) = self.columns.get(column) else {
            return;
        };
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
        self.sort_id = slot.column.id();
    }

    /// Replace every column but the built-in Name column at index 0 (which stays, so sorting, the breadcrumb and the rows keep their anchor); pass boxed [`BuiltInColumn`](crate::BuiltInColumn) variants for the defaults, boxed [`SizeColumn::new`] / [`ModifiedColumn::new`] instances to configure headers and renderers, or boxed custom [`Column`]s. The new list starts fully visible. Sorting follows the sorted column by its [`id`](Column::id): it keeps its direction at a new index, and a list without it sorts by Name, ascending.
    pub fn set_columns(&mut self, columns: impl IntoIterator<Item = Box<dyn Column>>) {
        self.columns.truncate(1);
        self.columns
            .extend(columns.into_iter().map(|column| ColumnSlot {
                column,
                hidden: false,
            }));
        match self
            .columns
            .iter()
            .position(|slot| slot.column.id() == self.sort_id)
        {
            Some(index) => self.sort_column = index,
            None => {
                self.sort_column = 0;
                self.sort_ascending = true;
                self.sort_id = self.columns[0].column.id();
            }
        }
    }

    /// The display index of the column this id names ([`column`](Self::column) reports the ids back), or `None` when no column has it.
    fn index_of(&self, id: &str) -> Option<usize> {
        self.columns.iter().position(|slot| slot.column.id() == id)
    }

    /// Take a column out of the drawing: its header cell, every body cell and its upright rules stop being painted and the remaining columns claim its width. The column itself stays listed — its index and sorting by it keep working — only the painting stops. Columns are named, not counted: [`ColumnKey`] from a [`BuiltInColumn`](crate::BuiltInColumn) variant or from the [`id`](Column::id) a custom column reports. The Name column at index 0 is always visible; an id that matches no column does nothing.
    pub fn hide_column(&mut self, column: impl Into<ColumnKey>) {
        if let Some(index) = self.index_of(column.into().id()) {
            // The Name column at index 0 is the anchor of rows, breadcrumb and sorting and stays visible.
            if index > 0 {
                self.columns[index].hidden = true;
            }
        }
    }

    /// Put a [hidden](Self::hide_column) column back into the drawing.
    pub fn show_column(&mut self, column: impl Into<ColumnKey>) {
        if let Some(index) = self.index_of(column.into().id()) {
            self.columns[index].hidden = false;
        }
    }

    /// Whether this column is hidden from the drawing (an id no column has: false).
    pub fn is_column_hidden(&self, column: impl Into<ColumnKey>) -> bool {
        self.index_of(column.into().id())
            .is_some_and(|index| self.columns[index].hidden)
    }

    /// Whether the column at this display index is hidden — the cabinet's per-frame check; apps name columns by id through [`is_column_hidden`](Self::is_column_hidden).
    pub(crate) fn is_column_hidden_at(&self, index: usize) -> bool {
        self.columns.get(index).is_some_and(|slot| slot.hidden)
    }

    /// Show these columns and only these; every other one is hidden (the Name column at index 0 is never). The entries are [`ColumnKey`]s, matched on the [`id`](Column::id) the columns report: built-ins as `ColumnKey::built_in(BuiltInColumn::Size)`, a custom column as `ColumnKey::custom("Type")`. An id that matches no column does nothing.
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
    /// browser.set_visible_columns([ColumnKey::built_in(BuiltInColumn::Size), ColumnKey::custom("Type")]);
    /// assert!(browser.is_column_hidden(BuiltInColumn::Modified));
    /// assert!(!browser.is_column_hidden(BuiltInColumn::Size));
    /// assert!(!browser.is_column_hidden("Type"));
    /// ```
    pub fn set_visible_columns(&mut self, visible: impl IntoIterator<Item = ColumnKey>) {
        let wanted: Vec<String> = visible.into_iter().map(|key| key.id().to_owned()).collect();
        for slot in self.columns.iter_mut().skip(1) {
            slot.hidden = !wanted.contains(&slot.column.id());
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
        self.hidden_tools.insert(tool.into().id().to_owned());
    }

    /// Put a [hidden](Self::hide_tool) tool back into the toolbar.
    pub fn show_tool(&mut self, tool: impl Into<ToolKey>) {
        self.hidden_tools.remove(tool.into().id());
    }

    /// Whether the toolbar leaves this tool out.
    pub fn is_tool_hidden(&self, tool: impl Into<ToolKey>) -> bool {
        self.hidden_tools.contains(tool.into().id())
    }

    /// Show these tools and only these; every other one the browser can name is hidden. The entries are [`ToolKey`]s, matched on the [`id`](crate::Tool::id) tools report: listed ids turn visible, the unlisted [built-ins](BuiltInTool::ALL) hidden. A custom tool the browser was never told about keeps its current state — it can only be switched through [`hide_tool`](Self::hide_tool) and [`show_tool`](Self::show_tool) — and a tool without an id always draws.
    ///
    /// ```
    /// # use iced_fabi::{Browser, BuiltInTool, ToolKey};
    /// let mut browser = Browser::new("My files");
    /// // an app whose toolbar offers copy and paste but no delete:
    /// browser.set_visible_tools([ToolKey::from(BuiltInTool::Copy), ToolKey::from(BuiltInTool::Paste)]);
    /// assert!(!browser.is_tool_hidden(BuiltInTool::Copy));
    /// assert!(browser.is_tool_hidden(BuiltInTool::Delete));
    /// ```
    pub fn set_visible_tools(&mut self, visible: impl IntoIterator<Item = ToolKey>) {
        let wanted: Vec<ToolKey> = visible.into_iter().collect();
        for key in &wanted {
            self.hidden_tools.remove(key.id());
        }
        for tool in BuiltInTool::ALL {
            if !wanted.iter().any(|key| key.id() == tool.id()) {
                self.hidden_tools.insert(tool.id().to_owned());
            }
        }
    }
}
