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
    column::{Column, ModifiedColumn, NameColumn, SizeColumn},
    event::Event,
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

    /// Append a custom column; returns its index (for [`sort_by_column`](Self::sort_by_column)).
    pub fn add_column(&mut self, column: impl Column + 'static) -> usize {
        self.columns.push(Box::new(column));
        self.columns.len() - 1
    }

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

    /// Replace every column but the built-in Name column at index 0 (which stays, so sorting, the breadcrumb and the rows keep their anchor); pass [`SizeColumn::new`] / [`ModifiedColumn::new`] instances to configure headers and renderers, or your own [`Column`]s.
    pub fn set_columns(&mut self, columns: impl IntoIterator<Item = Box<dyn Column>>) {
        self.columns.truncate(1);
        self.columns.extend(columns);
        if self.sort_column >= self.columns.len() {
            self.sort_column = 0;
        }
    }
}
