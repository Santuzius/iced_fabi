//! The state behind [`crate::Cabinet`]: a keyed tree of nodes that the application fills from wherever its listings come from (local disk, rclone, an HTTP API), plus breadcrumb navigation, columns, sorting and Material-style selection.
//!
//! The application owns a [`Browser`], sends it [`crate::Event`]s from the widget's buttons and feeds new listings back with [`Browser::set_children`]. The [`Browser`] never touches the filesystem or the network itself; [`Browser::activate`] tells it when data is missing ([`Outcome::NeedsLoad`]) so the app can fetch it asynchronously and stay in control of staleness (drop the response if the node was pruned meanwhile — [`Browser::set_children`] then does nothing).

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt,
};

/// Identity of a node in the tree; derived, not allocated, so the same directory addressed through the same path always has the same id (see [`NodeId::derive`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Identity of the root node, which has no parent.
    pub const ROOT: NodeId = NodeId(0);

    /// Deterministic id from the parent's id, the entry's kind and name (FNV-1a-64). Ids are therefore stable across refreshes — an update that re-lists a directory keeps its nodes, and expansion and selection survive.
    ///
    /// A collision (different child, same 64-bit hash) is not expected to occur; should one happen, the entry that was listed later wins the slot.
    pub fn derive(parent: NodeId, kind: EntryKind, name: &str) -> Self {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |hash: &mut u64, byte: u8| {
            *hash ^= byte as u64;
            *hash = hash.wrapping_mul(0x1000_0000_01b3);
        };
        for byte in parent.0.to_le_bytes() {
            mix(&mut hash, byte);
        }
        mix(
            &mut hash,
            match kind {
                EntryKind::Folder => 0x2f,
                EntryKind::File => 0x2e,
            },
        );
        for byte in name.as_bytes() {
            mix(&mut hash, *byte);
        }
        NodeId(hash)
    }
}

/// Whether a node is a directory (or a not-yet-listed branch that will turn out to have children) or a leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    Folder,
    File,
}

/// One listing entry handed to [`Browser::set_children`]. The display strings for `size` and `modified` are produced by the browser's formatters, so the app only passes raw values.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
    /// Size in bytes; `None` renders as an empty cell (and sorts as smallest).
    pub size: Option<u64>,
    /// Seconds since the Unix epoch; `None` renders empty and sorts first ascending.
    pub modified: Option<i64>,
}

/// A node the browser knows about, filled by [`Browser::set_children`].
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub kind: EntryKind,
    pub parent: Option<NodeId>,
    /// Children in listing order; the view sorts them per [`Browser::sort_by_column`].
    pub children: Vec<NodeId>,
    /// Whether the children of this node are known (it was listed at least once and the listing stuck).
    pub children_known: bool,
    /// A load requested via [`Outcome::NeedsLoad`] that has not been answered yet.
    pub loading: bool,
    /// The last load of this node failed; a later [`Browser::activate`] retries.
    pub failed: bool,
    pub size: Option<u64>,
    pub modified: Option<i64>,
}

/// How a node's selection marker renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// Empty box; the node and its loaded content are unselected.
    Unselected,
    /// Tick; the node is selected as a unit — explicitly or because one of its ancestors is, which covers it the same way — in the app's semantics typically its whole subtree, whether or not the subtree is loaded.
    Selected,
    /// Dot in the box; some node below this one is selected, but this node is not selected itself.
    Partial,
}

/// The two ways of looking at the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// One level at a time: the children of [`Browser::current`], with the [`Browser::path`] above it as breadcrumbs (Android Documents-App style).
    Browse,
    /// The whole tree in place, every level expandable; selection can span levels.
    Tree,
}

/// How many entries the selection can hold at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// As many units as the user likes, across levels.
    #[default]
    Multiple,
    /// At most one entry: a new tick replaces whatever was selected.
    Single,
}

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

/// The built-in modification-time column, rendered with [`Browser::format_modified`].
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

/// The built-in size column, rendered with [`Browser::format_size`].
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

/// The consequence of a [`Browser`] action, telling the application what it must do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing happened (or a toggle/selection-only event that the calling code already accounted for).
    None,
    /// The app should list this node's children now; [`Browser::loading`] is set until [`Browser::set_children`] or [`Browser::load_failed`] answers it.
    NeedsLoad(NodeId),
    /// A file was activated and the app should decide what that means (open it, select it, …).
    FileActivated(NodeId),
}

/// One line of the list: which node, how deep it sits (tree mode indents by this), and how its controls look. The view builds rows for browse mode and tree mode alike, so it never branches on the mode itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub id: NodeId,
    pub depth: usize,
    /// Whether a caret is shown (tree mode only): folders and unlisted branches.
    pub expandable: bool,
    pub expanded: bool,
    pub marker: Marker,
}

/// Formats a byte count for the size column.
pub type FormatSize = fn(u64) -> String;

/// Formats epoch seconds for the modified column.
pub type FormatModified = fn(i64) -> String;

/// The state of one file-browser screen: tree, navigation, columns, selection, sorting, mode.
#[derive(Debug)]
pub struct Browser {
    nodes: HashMap<NodeId, Node>,
    /// Breadcrumb from the root down to [`current`](Self::current).
    path: Vec<NodeId>,
    current: NodeId,
    /// Nodes whose branch is open in tree mode. The path to the current node is kept expanded so browse→tree shows the same place.
    expanded: HashSet<NodeId>,
    /// Explicit unit selections; a node selected here is not selected again below it, and [`toggle`](Self::toggle) refuses to select under a selected ancestor.
    selected: HashSet<NodeId>,
    /// Whether the boxes accumulate or replace.
    selection_mode: SelectionMode,
    /// Index into [`columns`](Self::columns); index 0 is the built-in [`NameColumn`].
    sort_column: usize,
    sort_ascending: bool,
    /// Folders sort before files regardless of the key, as file managers usually do.
    folders_first: bool,
    /// Hide file rows and list folders only.
    folders_only: bool,
    /// When set, a file row appears only if its extension (after the last dot, case-insensitive) is in this list; folders are unaffected. `None` lists every file.
    extensions: Option<Vec<String>>,
    mode: Mode,
    columns: Vec<Box<dyn Column>>,
    format_size: FormatSize,
    format_modified: FormatModified,
}

impl Browser {
    /// A browser whose root is a folder named `root`, not listed yet: the first [`activate`](Self::activate) on it (which the view triggers on open) asks the app for its children. The columns start as Name, Modified, Size.
    pub fn new(root: impl Into<String>) -> Self {
        let root_name = root.into();
        let node = Node {
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
            selection_mode: SelectionMode::Multiple,
            sort_column: 0,
            sort_ascending: true,
            folders_first: true,
            folders_only: false,
            extensions: None,
            mode: Mode::Browse,
            columns: vec![
                Box::new(NameColumn),
                Box::new(ModifiedColumn),
                Box::new(SizeColumn),
            ],
            format_size: format_bytes,
            format_modified: format_epoch,
        }
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

    /// Override the size and modified renderers (e.g. to localise).
    pub fn set_formatters(&mut self, size: FormatSize, modified: FormatModified) {
        self.format_size = size;
        self.format_modified = modified;
    }

    pub fn format_size(&self, bytes: u64) -> String {
        (self.format_size)(bytes)
    }

    pub fn format_modified(&self, epoch: i64) -> String {
        (self.format_modified)(epoch)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn selection_mode(&self) -> SelectionMode {
        self.selection_mode
    }

    /// `Single` makes every box replace the selection: at most one entry is ever ticked; the header's select-all then ticks the scope itself as that one unit, and unticking a covered entry clears the whole (only) selection.
    pub fn set_selection_mode(&mut self, mode: SelectionMode) {
        self.selection_mode = mode;
        if mode == SelectionMode::Single {
            // Keep at most one unit: the first explicit one wins, the rest go.
            let first = self.selected.iter().next().copied();
            self.selected.clear();
            if let Some(id) = first {
                self.selected.insert(id);
            }
        }
    }

    /// Switches mode; entering tree mode keeps the current node expanded.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if mode == Mode::Tree {
            self.expanded.insert(self.current);
        }
    }

    pub fn folders_first(&self) -> bool {
        self.folders_first
    }

    pub fn set_folders_first(&mut self, yes: bool) {
        self.folders_first = yes;
    }

    /// List folders only: every file row disappears from both modes, so files cannot be reached or selected while hidden. Default: off.
    pub fn set_folders_only(&mut self, yes: bool) {
        self.folders_only = yes;
    }

    /// Show only files whose extension — the part after the last dot, compared case-insensitively — is one of these; folders are unaffected. This is the app-side knob, usually set once while building the [`Browser`]; a file without a dot never passes while a filter is set. [`show_all_files`](Self::show_all_files) (the default) lists every file again.
    pub fn set_file_extensions(&mut self, extensions: impl IntoIterator<Item = impl Into<String>>) {
        self.extensions = Some(
            extensions
                .into_iter()
                .map(|ext| {
                    let mut ext: String = ext.into();
                    ext.make_ascii_lowercase();
                    ext
                })
                .collect(),
        );
    }

    /// Undo [`set_file_extensions`](Self::set_file_extensions): every file is listed again.
    pub fn show_all_files(&mut self) {
        self.extensions = None;
    }

    /// Whether the node's row appears in the lists under the current visibility settings.
    fn is_listed(&self, id: NodeId) -> bool {
        let Some(node) = self.nodes.get(&id) else {
            return false;
        };
        if node.kind == EntryKind::Folder {
            return true;
        }
        if self.folders_only {
            return false;
        }
        match &self.extensions {
            None => true,
            Some(exts) => node
                .name
                .rsplit_once('.')
                .is_some_and(|(_, ext)| exts.iter().any(|e| e.eq_ignore_ascii_case(ext))),
        }
    }

    /// The node whose children the browse mode shows.
    pub fn current(&self) -> NodeId {
        self.current
    }

    /// Breadcrumb from the root to the current node.
    pub fn path(&self) -> &[NodeId] {
        &self.path
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Node names from the root down to `id` — enough to rebuild the backend's path.
    pub fn name_chain(&self, id: NodeId) -> Vec<&str> {
        let mut chain = Vec::new();
        let mut here = Some(id);
        while let Some(node) = here.and_then(|id| self.nodes.get(&id)) {
            chain.push(node.name.as_str());
            here = node.parent;
        }
        chain.reverse();
        chain
    }

    pub fn is_expanded(&self, id: NodeId) -> bool {
        self.expanded.contains(&id)
    }

    pub fn loading(&self, id: NodeId) -> bool {
        self.nodes.get(&id).is_some_and(|n| n.loading)
    }

    /// The explicit unit selections; effective selection is this set (a selected folder stands for its whole subtree).
    pub fn selected(&self) -> impl Iterator<Item = NodeId> + use<'_> {
        self.selected.iter().copied()
    }

    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// Remove the selected entries from the tree, the way a delete the app performed in the backend must show up; see [`remove`](Self::remove). The selection ends up empty.
    pub fn remove_selected(&mut self) {
        let selected: Vec<NodeId> = self.selected.iter().copied().collect();
        self.remove(&selected);
        self.selected.clear();
    }

    /// Remove entries from the tree, the way a delete the app performed in the backend must show up: each entry is spliced out of its parent's listing and its whole subtree's state (nodes, expansion, selections) is pruned. Entries below another listed entry go with it. Refuses to delete the root scope.
    pub fn remove(&mut self, ids: &[NodeId]) {
        if ids.contains(&NodeId::ROOT) {
            return;
        }
        // The parent whose listing loses the entry, then the entry itself.
        let victims: Vec<(NodeId, NodeId)> = ids
            .iter()
            .copied()
            .filter(|id| {
                !ids.iter()
                    .any(|other| *other != *id && is_below(self, *id, *other))
            })
            .filter_map(|id| Some((self.nodes.get(&id)?.parent?, id)))
            .collect();
        for (parent, id) in victims {
            if let Some(node) = self.nodes.get_mut(&parent) {
                node.children.retain(|child| *child != id);
            }
            self.prune(id);
        }
    }

    /// The node whose children the list shows at the top level: the current directory in browse mode, the root in tree mode.
    pub fn scope(&self) -> NodeId {
        if self.mode == Mode::Tree {
            NodeId::ROOT
        } else {
            self.current
        }
    }

    /// The marker of the header's select-all box: ticked when the scope, one of its ancestors, or every one of its children is selected, a dot when something below the scope is.
    pub fn scope_marker(&self) -> Marker {
        let scope = self.scope();
        let children: Vec<NodeId> = self
            .nodes
            .get(&scope)
            .map(|node| {
                node.children
                    .iter()
                    .copied()
                    .filter(|id| self.is_listed(*id))
                    .collect()
            })
            .unwrap_or_default();
        if self.selected.contains(&scope)
            || self.selected_ancestor(scope).is_some()
            || (!children.is_empty() && children.iter().all(|child| self.selected.contains(child)))
        {
            Marker::Selected
        } else if self.has_selected_below(scope) {
            Marker::Partial
        } else {
            Marker::Unselected
        }
    }

    /// The header's select-all box clicked: select every child of the scope when nothing in it is selected, otherwise clear the selection within the scope. When the scope is only ticked because an ancestor is selected, that unticks the ancestor — the same rule as for a covered row's box, see [`toggle`](Self::toggle). In [`SelectionMode::Single`] the box instead toggles the scope itself as the one unit.
    pub fn toggle_all(&mut self) {
        let scope = self.scope();
        if self.selection_mode == SelectionMode::Single {
            let on = self.selected.contains(&scope) || self.selected_ancestor(scope).is_some();
            self.selected.clear();
            if !on {
                self.selected.insert(scope);
            }
            return;
        }
        if let Some(ancestor) = self.selected_ancestor(scope) {
            self.selected.remove(&ancestor);
            return;
        }
        if self.scope_marker() == Marker::Unselected {
            self.select_all();
        } else {
            let inside: Vec<NodeId> = self
                .selected
                .iter()
                .copied()
                .filter(|id| *id == scope || is_below(self, *id, scope))
                .collect();
            for id in inside {
                self.selected.remove(&id);
            }
        }
    }

    /// Select every child of the scope node — the current directory in browse mode, the root in tree mode — provided it is listed. Does nothing while the scope itself or one of its ancestors is selected (the children are already covered). In [`SelectionMode::Single`] this selects the scope itself as the one unit.
    pub fn select_all(&mut self) {
        let scope = self.scope();
        if self.selected.contains(&scope) || self.selected_ancestor(scope).is_some() {
            return;
        }
        if self.selection_mode == SelectionMode::Single {
            self.selected.clear();
            self.selected.insert(scope);
            return;
        }
        let Some(node) = self.nodes.get(&scope) else {
            return;
        };
        if !node.children_known {
            return;
        }
        let children: Vec<NodeId> = node
            .children
            .iter()
            .copied()
            .filter(|id| self.is_listed(*id))
            .collect();
        self.selected.extend(children);
    }

    /// The marker this node shows: a tick when it is selected, explicitly or covered by a selected ancestor, a dot when something below it is selected, otherwise an empty box. A covered node therefore looks selected, mirroring what [`toggle`](Self::toggle) does when one of its boxes is tapped.
    pub fn marker(&self, id: NodeId) -> Marker {
        if self.selected.contains(&id) || self.selected_ancestor(id).is_some() {
            Marker::Selected
        } else if self.has_selected_below(id) {
            Marker::Partial
        } else {
            Marker::Unselected
        }
    }

    /// Whether anything at or below `id` (only loaded parts can be known) is explicitly selected.
    fn has_selected_below(&self, id: NodeId) -> bool {
        let Some(node) = self.nodes.get(&id) else {
            return false;
        };
        node.children
            .iter()
            .any(|child| self.selected.contains(child) || self.has_selected_below(*child))
    }

    /// The row for the browse view: the sorted children of the current node.
    pub fn browse_rows(&self) -> Vec<Row> {
        let Some(current) = self.nodes.get(&self.current) else {
            return Vec::new();
        };
        self.sorted(&current.children)
            .into_iter()
            .filter(|id| self.is_listed(*id))
            .map(|id| Row {
                id,
                depth: 0,
                expandable: false,
                expanded: false,
                marker: self.marker(id),
            })
            .collect()
    }

    /// The rows for the tree view: every expanded branch from the root down, the root included.
    pub fn tree_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        self.tree_rows_at(NodeId::ROOT, 0, &mut rows);
        rows
    }

    fn tree_rows_at(&self, id: NodeId, depth: usize, rows: &mut Vec<Row>) {
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        let expanded = self.expanded.contains(&id);
        if !self.is_listed(id) {
            return;
        }
        rows.push(Row {
            id,
            depth,
            expandable: node.kind == EntryKind::Folder,
            expanded,
            marker: self.marker(id),
        });
        // A file has no children; an unlisted folder shows no child rows until loaded, but keeps its caret (see `expandable`).
        if expanded {
            for child in self.sorted(&node.children) {
                self.tree_rows_at(child, depth + 1, rows);
            }
        }
    }

    /// The children of `id` in display order, by the sorted column ([`Column::compare`]), ties broken by name; `folders_first` wins over both.
    fn sorted(&self, children: &[NodeId]) -> Vec<NodeId> {
        let mut nodes: Vec<&Node> = children
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .collect();
        let column = &self.columns[self.sort_column.min(self.columns.len() - 1)];
        let ascending = self.sort_ascending;
        let folders_first = self.folders_first;
        nodes.sort_by(|a, b| {
            if folders_first && a.kind != b.kind {
                return match (a.kind, b.kind) {
                    (EntryKind::Folder, _) => Ordering::Less,
                    (_, EntryKind::Folder) => Ordering::Greater,
                    _ => Ordering::Equal,
                };
            }
            let by_key = column.compare(a, b);
            let ordered = if ascending { by_key } else { by_key.reverse() };
            // Equal keys (and unsortable columns) fall back to name so the order is stable.
            ordered.then_with(|| name_key(&a.name).cmp(&name_key(&b.name)))
        });
        nodes.iter().map(|node| node.id()).collect()
    }

    /// A folder click (caret in tree mode): navigate in browse mode, expand or collapse in tree mode, and request the listing when the children are unknown.
    pub fn activate(&mut self, id: NodeId) -> Outcome {
        let Some(node) = self.nodes.get(&id) else {
            return Outcome::None;
        };
        if node.kind == EntryKind::File {
            return Outcome::FileActivated(id);
        }
        if self.mode == Mode::Tree {
            let now_expanded = !self.expanded.contains(&id);
            if now_expanded {
                self.expanded.insert(id);
            } else {
                self.expanded.remove(&id);
                return Outcome::None;
            }
            return self.request_load(id);
        }
        // Browse mode: jump to this directory.
        self.current = id;
        self.rebuild_path(id);
        self.expanded.insert(id);
        self.request_load(id)
    }

    /// Jump to an earlier breadcrumb node; only path ancestors can be jumped to.
    pub fn goto(&mut self, id: NodeId) -> Outcome {
        if let Some(position) = self.path.iter().position(|node| *node == id) {
            self.path.truncate(position + 1);
            self.current = id;
            self.expanded.insert(id);
            self.request_load(id)
        } else {
            Outcome::None
        }
    }

    /// One breadcrumb step up.
    pub fn up(&mut self) -> Outcome {
        if self.path.len() > 1 {
            self.path.pop();
            self.current = *self.path.last().expect("path kept non-empty");
            self.request_load(self.current)
        } else {
            Outcome::None
        }
    }

    /// Click on a row's name in browse mode: folders navigate, files report activation.
    pub fn tap(&mut self, id: NodeId) -> Outcome {
        let file = self
            .nodes
            .get(&id)
            .is_some_and(|node| node.kind == EntryKind::File);
        if file && self.mode == Mode::Browse {
            // Material-style: without a separate open action a file tap selects it.
            self.toggle(id);
            return Outcome::None;
        }
        self.activate(id)
    }

    /// Toggle the unit selection of a node.
    ///
    /// Rules (Material-inspired): a node under a selected ancestor shows a tick because the ancestor's unit covers it, and clicking its box removes just that entry from the selection (see [`uncheck_below`](Self::uncheck_below)) — the sibling rows stay ticked and the ancestors become dots; selecting a folder that had explicit selections below clears them, because the folder now stands for its whole subtree. In [`SelectionMode::Single`] any tap first clears everything: tapping the (possibly covered) entry that reads ticked leaves nothing selected, tapping another makes it the one selection.
    pub fn toggle(&mut self, id: NodeId) {
        if !self.nodes.contains_key(&id) {
            return;
        }
        if self.selection_mode == SelectionMode::Single {
            let on = self.selected.contains(&id) || self.selected_ancestor(id).is_some();
            self.selected.clear();
            if !on {
                self.selected.insert(id);
            }
            return;
        }
        if self.selected.contains(&id) {
            self.selected.remove(&id);
            return;
        }
        if let Some(ancestor) = self.selected_ancestor(id) {
            self.uncheck_below(ancestor, id);
            return;
        }
        self.selected.insert(id);
        let clear: Vec<NodeId> = self
            .selected
            .iter()
            .copied()
            .filter(|other| *other != id && is_below(self, *other, id))
            .collect();
        for other in clear {
            self.selected.remove(&other);
        }
    }

    /// Unticks one covered entry while the rest of the selected `ancestor`'s subtree stays selected: the unit is dissolved along the chain from the ancestor down to `id`, re-entering every listed child as an explicit unit except the one the chain leads through. That loses nothing: `id` shows on screen, so every folder on the chain is expanded and listed, and the re-entered siblings cover their unloaded subtrees themselves. The chain's folders end up Partial, `id` itself Unselected.
    fn uncheck_below(&mut self, ancestor: NodeId, id: NodeId) {
        self.selected.remove(&ancestor);
        let mut chain: Vec<NodeId> = Vec::new();
        let mut here = id;
        while here != ancestor {
            chain.push(here);
            match self.nodes.get(&here).and_then(|node| node.parent) {
                Some(parent) => here = parent,
                None => break,
            }
        }
        let mut level = ancestor;
        for step in chain.iter().rev() {
            let siblings = match self.nodes.get(&level) {
                Some(node) => node.children.clone(),
                None => break,
            };
            for sibling in siblings {
                if sibling != *step {
                    self.selected.insert(sibling);
                }
            }
            level = *step;
        }
    }

    fn selected_ancestor(&self, id: NodeId) -> Option<NodeId> {
        let mut parent = self.nodes.get(&id).and_then(|node| node.parent);
        while let Some(here) = parent {
            if self.selected.contains(&here) {
                return Some(here);
            }
            parent = self.nodes.get(&here).and_then(|node| node.parent);
        }
        None
    }

    /// Answer a [`Outcome::NeedsLoad`]: install `entries` as the children of `id`.
    ///
    /// Does nothing if `id` is gone (pruned by an earlier refresh — the app's own staleness guard can also skip the call). Existing child nodes keep their id, so their expansion and selections survive; children that disappeared are pruned with their whole subtree.
    pub fn set_children(&mut self, id: NodeId, entries: Vec<Entry>) {
        if !self.nodes.contains_key(&id) {
            return;
        }
        let mut next = Vec::with_capacity(entries.len());
        for entry in entries {
            let child_id = NodeId::derive(id, entry.kind, &entry.name);
            let node = self.nodes.entry(child_id).or_insert(Node {
                name: entry.name.clone(),
                kind: entry.kind,
                parent: Some(id),
                children: Vec::new(),
                children_known: false,
                loading: false,
                failed: false,
                size: None,
                modified: None,
            });
            node.name = entry.name;
            node.kind = entry.kind;
            node.size = entry.size;
            node.modified = entry.modified;
            next.push(child_id);
        }
        let Some(parent) = self.nodes.get_mut(&id) else {
            return;
        };
        parent.children_known = true;
        parent.loading = false;
        parent.failed = false;
        let stale: Vec<NodeId> = parent
            .children
            .iter()
            .copied()
            .filter(|child| !next.contains(child))
            .collect();
        parent.children = next;
        for child in stale {
            self.prune(child);
        }
    }

    /// Report that the listing for `id` failed; the node stays a (still expandable) branch and a later `activate` retries.
    pub fn load_failed(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.loading = false;
            node.failed = true;
        }
    }

    /// Mark `id` as loading now and ask the app for it next time `activate` reaches it (used for the implicit load of the root on open).
    pub fn request_load(&mut self, id: NodeId) -> Outcome {
        let Some(node) = self.nodes.get_mut(&id) else {
            return Outcome::None;
        };
        if node.children_known || node.loading {
            return Outcome::None;
        }
        node.loading = true;
        Outcome::NeedsLoad(id)
    }

    fn rebuild_path(&mut self, id: NodeId) {
        let mut chain = vec![id];
        let mut here = id;
        while let Some(parent) = self.nodes.get(&here).and_then(|node| node.parent) {
            chain.push(parent);
            here = parent;
        }
        chain.reverse();
        self.path = chain;
    }

    /// Drop a node, its subtree, its selections and its expansion state (a disappeared entry cannot be selected anymore).
    fn prune(&mut self, id: NodeId) {
        let children = self
            .nodes
            .remove(&id)
            .map(|node| node.children)
            .unwrap_or_default();
        self.expanded.remove(&id);
        self.selected.remove(&id);
        for child in children {
            self.prune(child);
        }
        // If the view pointed at a pruned node, fall back to its parent's parent.
        if self.current == id {
            self.rebuild_path_fallback();
        }
    }

    fn rebuild_path_fallback(&mut self) {
        self.path.retain(|id| self.nodes.contains_key(id));
        if self.path.is_empty() {
            self.path = vec![NodeId::ROOT];
        }
        self.current = *self.path.last().expect("path kept non-empty");
    }
}

/// The id a node carries: derived from its parent, kind and name (the same function [`NodeId::derive`] uses when listing).
impl Node {
    fn id(&self) -> NodeId {
        NodeId::derive(self.parent.unwrap_or(NodeId::ROOT), self.kind, &self.name)
    }
}

/// Whether `id` is a proper descendant of `below` (walking parents, so it stays correct as the tree is rebuilt).
fn is_below(browser: &Browser, id: NodeId, below: NodeId) -> bool {
    let mut parent = browser.nodes.get(&id).and_then(|node| node.parent);
    while let Some(here) = parent {
        if here == below {
            return true;
        }
        parent = browser.nodes.get(&here).and_then(|node| node.parent);
    }
    false
}

/// Natural-order key for names: runs of digits compare by value, so `File 10` sorts after `File 9`.
fn name_key(name: &str) -> Vec<NameChunk> {
    let mut chunks = Vec::new();
    let mut digits = String::new();
    for ch in name.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            if !digits.is_empty() {
                chunks.push(NameChunk::Number(
                    digits.parse().expect("digits parse as u64"),
                ));
                digits.clear();
            }
            chunks.push(NameChunk::Text(ch.to_lowercase().collect()));
        }
    }
    if !digits.is_empty() {
        chunks.push(NameChunk::Number(
            digits.parse().expect("digits parse as u64"),
        ));
    }
    chunks
}

#[derive(Debug, PartialEq, Eq)]
enum NameChunk {
    Text(String),
    Number(u64),
}

impl Ord for NameChunk {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (NameChunk::Text(a), NameChunk::Text(b)) => a.cmp(b),
            (NameChunk::Number(a), NameChunk::Number(b)) => a.cmp(b),
            (NameChunk::Text(_), NameChunk::Number(_)) => Ordering::Less,
            (NameChunk::Number(_), NameChunk::Text(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for NameChunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `372.82 KB`-style size with 1000-based units up to GB (the Material Files convention).
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let value = bytes as f64;
    let mut unit = 0;
    let mut scaled = value;
    while scaled >= 1000.0 && unit < UNITS.len() - 1 {
        scaled /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.2} {}", scaled, UNITS[unit])
    }
}

/// Epoch seconds to `Apr 7, 2021, 17:45` (UTC; a localised browser should pass its own formatter).
pub fn format_epoch(epoch: i64) -> String {
    let (days, seconds_of_day) = (epoch.div_euclid(86_400), epoch.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    let (hour, minute) = (seconds_of_day / 3600 % 24, seconds_of_day % 3600 / 60);
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{month} {day}, {year}, {hour:0>2}:{minute:0>2}",
        month = MONTHS[(month - 1) as usize]
    )
}

/// Howard Hinnant's `civil_from_days` (public domain): days since 1970-01-01 to proleptic Gregorian (y, m, d).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Browser {
        let mut browser = Browser::new("My files");
        browser.set_children(
            NodeId::ROOT,
            vec![
                Entry {
                    name: "Finanzen".into(),
                    kind: EntryKind::Folder,
                    size: None,
                    modified: Some(1_745_000_000),
                },
                Entry {
                    name: "notes.txt".into(),
                    kind: EntryKind::File,
                    size: Some(900),
                    modified: Some(1_745_000_100),
                },
            ],
        );
        browser
    }

    #[test]
    fn derived_ids_are_stable() {
        let a = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        let b = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        assert_eq!(a, b);
        assert_ne!(a, NodeId::derive(NodeId::ROOT, EntryKind::File, "Finanzen"));
    }

    #[test]
    fn activate_walks_path_and_loads_once() {
        let mut browser = sample();
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        assert_eq!(browser.activate(folder), Outcome::NeedsLoad(folder));
        assert_eq!(browser.current(), folder, "browse mode navigates");
        assert_eq!(
            browser.activate(folder),
            Outcome::None,
            "still loading, no second request"
        );
        assert_eq!(browser.path(), &[NodeId::ROOT, folder]);
        browser.set_children(
            folder,
            vec![Entry {
                name: "Krypto".into(),
                kind: EntryKind::Folder,
                size: None,
                modified: None,
            }],
        );
        let krypto = NodeId::derive(folder, EntryKind::Folder, "Krypto");
        assert_eq!(browser.activate(krypto), Outcome::NeedsLoad(krypto));
    }

    #[test]
    fn partial_marker_propagates_up() {
        let mut browser = sample();
        let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        browser.toggle(file);
        assert_eq!(browser.marker(file), Marker::Selected);
        assert_eq!(browser.marker(NodeId::ROOT), Marker::Partial);
    }

    #[test]
    fn selected_folder_ticks_its_loaded_children() {
        let mut browser = sample();
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        browser.set_children(
            folder,
            vec![Entry {
                name: "beleg.pdf".into(),
                kind: EntryKind::File,
                size: None,
                modified: None,
            }],
        );
        let child = NodeId::derive(folder, EntryKind::File, "beleg.pdf");
        browser.toggle(folder);
        assert_eq!(
            browser.marker(child),
            Marker::Selected,
            "the folder's tick shows on the child row too"
        );
        assert_eq!(
            browser.selected().collect::<Vec<_>>(),
            vec![folder],
            "the selection set stays normalized: only the folder is explicit"
        );
        browser.toggle(child);
        assert!(
            !browser.has_selection(),
            "tapping the only covered row leaves nothing ticked"
        );
        assert_eq!(browser.marker(child), Marker::Unselected);
        assert_eq!(
            browser.marker(folder),
            Marker::Unselected,
            "no dot left without selected children"
        );
    }

    #[test]
    fn a_scope_under_a_selected_ancestor_shares_its_selection() {
        let mut browser = sample();
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        browser.set_children(
            folder,
            vec![Entry {
                name: "beleg.pdf".into(),
                kind: EntryKind::File,
                size: None,
                modified: None,
            }],
        );
        browser.activate(folder); // browse into it, then check an ancestor
        browser.toggle(NodeId::ROOT);
        assert_eq!(
            browser.scope_marker(),
            Marker::Selected,
            "inside a checked folder the select-all box is ticked"
        );
        browser.select_all();
        assert_eq!(
            browser.selected().collect::<Vec<_>>(),
            vec![NodeId::ROOT],
            "select-all adds nothing under a selected ancestor"
        );
        browser.toggle_all();
        assert!(
            !browser.has_selection(),
            "the box unticks the covering ancestor"
        );
    }

    #[test]
    fn unchecking_a_covered_row_keeps_the_rest_of_the_unit() {
        let mut browser = sample();
        browser.toggle(NodeId::ROOT);
        let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        browser.toggle(file);
        assert_eq!(
            browser.marker(file),
            Marker::Unselected,
            "the tapped box unticks itself"
        );
        assert_eq!(
            browser.marker(folder),
            Marker::Selected,
            "the sibling stays ticked"
        );
        assert_eq!(
            browser.marker(NodeId::ROOT),
            Marker::Partial,
            "the ancestor becomes a dot"
        );
        assert_eq!(browser.selected().collect::<Vec<_>>(), vec![folder]);
        browser.toggle(NodeId::ROOT);
        assert_eq!(
            browser.selected().collect::<Vec<_>>(),
            vec![NodeId::ROOT],
            "rechecking the ancestor collapses the dissolved unit back into one"
        );
        assert_eq!(
            browser.marker(file),
            Marker::Selected,
            "covered by the root again"
        );
    }

    #[test]
    fn unchecking_deep_below_a_selected_folder_dissolves_along_the_chain() {
        let mut browser = sample();
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        browser.set_children(
            folder,
            vec![
                Entry {
                    name: "Depot".into(),
                    kind: EntryKind::Folder,
                    size: None,
                    modified: None,
                },
                Entry {
                    name: "beleg.pdf".into(),
                    kind: EntryKind::File,
                    size: None,
                    modified: None,
                },
            ],
        );
        let depot = NodeId::derive(folder, EntryKind::Folder, "Depot");
        browser.set_children(
            depot,
            vec![
                Entry {
                    name: "keys.txt".into(),
                    kind: EntryKind::File,
                    size: None,
                    modified: None,
                },
                Entry {
                    name: "pub.txt".into(),
                    kind: EntryKind::File,
                    size: None,
                    modified: None,
                },
            ],
        );
        let keys = NodeId::derive(depot, EntryKind::File, "keys.txt");
        let public = NodeId::derive(depot, EntryKind::File, "pub.txt");
        browser.toggle(folder);
        assert_eq!(
            browser.marker(keys),
            Marker::Selected,
            "covered from two levels up"
        );
        browser.toggle(keys);
        assert_eq!(browser.marker(keys), Marker::Unselected);
        assert_eq!(
            browser.marker(public),
            Marker::Selected,
            "the sibling inside the same folder stays ticked"
        );
        assert_eq!(
            browser.marker(depot),
            Marker::Partial,
            "the chain's folder becomes a dot"
        );
        assert_eq!(browser.marker(folder), Marker::Partial);
        let mut units: Vec<NodeId> = browser.selected().collect();
        units.sort();
        assert_eq!(units, {
            let mut expected = vec![NodeId::derive(folder, EntryKind::File, "beleg.pdf"), public];
            expected.sort();
            expected
        });
    }

    #[test]
    fn selecting_folder_clears_explicit_descendants() {
        let mut browser = sample();
        let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        browser.toggle(file);
        browser.toggle(NodeId::ROOT);
        assert_eq!(browser.selected().collect::<Vec<_>>(), vec![NodeId::ROOT]);
    }

    #[test]
    fn select_all_marks_current_children() {
        let mut browser = sample();
        browser.select_all();
        let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        assert!(browser.selected().collect::<Vec<_>>().contains(&file));
        assert!(browser.selected().collect::<Vec<_>>().contains(&folder));
        // The scope itself stays unselected, marked Partial by its selected children.
        assert_eq!(browser.marker(NodeId::ROOT), Marker::Partial);
    }

    #[test]
    fn pruning_removes_selections_of_gone_entries() {
        let mut browser = sample();
        let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        browser.toggle(file);
        browser.set_children(
            NodeId::ROOT,
            vec![Entry {
                name: "Finanzen".into(),
                kind: EntryKind::Folder,
                size: None,
                modified: None,
            }],
        );
        assert!(!browser.has_selection());
        assert!(!browser.nodes.contains_key(&file));
    }

    #[test]
    fn remove_selected_deletes_and_prunes() {
        let mut browser = sample();
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        browser.activate(folder);
        browser.set_children(
            folder,
            vec![Entry {
                name: "Krypto".into(),
                kind: EntryKind::Folder,
                size: None,
                modified: None,
            }],
        );
        let krypto = NodeId::derive(folder, EntryKind::Folder, "Krypto");
        let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        browser.goto(krypto);
        browser.toggle(folder);
        browser.goto(NodeId::ROOT);
        browser.toggle(file);
        browser.remove_selected();
        assert!(
            !browser.nodes.contains_key(&folder),
            "the folder node is gone"
        );
        assert!(
            !browser.nodes.contains_key(&krypto),
            "so is its whole subtree"
        );
        assert!(!browser.nodes.contains_key(&file));
        let root = browser.node(NodeId::ROOT).expect("root");
        assert!(root.children.is_empty(), "the listing lost both");
        assert!(root.children_known, "the parent stays a known listing");
        assert!(!browser.expanded.contains(&folder), "expansion pruned");
        assert!(
            !browser.has_selection(),
            "a done tool leaves nothing checked"
        );
    }

    #[test]
    fn scope_marker_tracks_the_scope_selection() {
        let mut browser = sample();
        assert_eq!(
            browser.scope_marker(),
            Marker::Unselected,
            "nothing is checked"
        );
        let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        browser.toggle(file);
        assert_eq!(
            browser.scope_marker(),
            Marker::Partial,
            "one of the two children is checked"
        );
        browser.select_all();
        assert_eq!(
            browser.scope_marker(),
            Marker::Selected,
            "now every child is"
        );
        browser.clear_selection();
        browser.toggle(NodeId::ROOT);
        assert_eq!(
            browser.scope_marker(),
            Marker::Selected,
            "the scope itself counts too"
        );
    }

    #[test]
    fn toggle_all_selects_then_clears_only_inside_the_scope() {
        let mut browser = sample();
        let outside = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        browser.toggle(outside);
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        browser.activate(folder);
        browser.set_children(
            folder,
            vec![Entry {
                name: "Krypto".into(),
                kind: EntryKind::Folder,
                size: None,
                modified: None,
            }],
        );
        let inside = NodeId::derive(folder, EntryKind::Folder, "Krypto");
        assert_eq!(
            browser.scope(),
            folder,
            "the header box works on the current directory"
        );
        assert_eq!(
            browser.scope_marker(),
            Marker::Unselected,
            "a selection one level up is outside this scope"
        );
        browser.toggle_all();
        let selected: Vec<NodeId> = browser.selected().collect();
        assert!(
            selected.contains(&inside),
            "this directory's children got checked"
        );
        assert!(
            selected.contains(&outside),
            "the selection from one level up stays"
        );
        browser.toggle_all();
        assert_eq!(
            browser.selected().collect::<Vec<_>>(),
            vec![outside],
            "the second click clears only this directory"
        );
    }

    #[test]
    fn remove_drops_nested_ids_once_and_refuses_the_root() {
        let mut browser = sample();
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        browser.activate(folder);
        browser.set_children(
            folder,
            vec![Entry {
                name: "Krypto".into(),
                kind: EntryKind::Folder,
                size: None,
                modified: None,
            }],
        );
        let krypto = NodeId::derive(folder, EntryKind::Folder, "Krypto");
        browser.toggle(krypto);
        // Both ids at once: the nested one must not need splicing out on its own.
        browser.remove(&[folder, krypto]);
        assert!(!browser.nodes.contains_key(&folder), "gone from the tree");
        assert!(
            !browser.nodes.contains_key(&krypto),
            "and so is everything below it"
        );
        let root = browser.node(NodeId::ROOT).expect("root");
        assert!(
            !root.children.contains(&folder),
            "the root's listing lost the folder"
        );
        assert!(!browser.expanded.contains(&folder), "expansion pruned");
        assert_eq!(
            browser.current(),
            NodeId::ROOT,
            "the view fell back from the deleted directory"
        );
        assert_eq!(browser.path(), &[NodeId::ROOT], "and so did the breadcrumb");
        assert!(
            !browser.has_selection(),
            "selections in the deleted subtree go with it"
        );
        browser.remove(&[NodeId::ROOT]);
        assert!(
            browser.nodes.contains_key(&NodeId::ROOT),
            "the root scope cannot be deleted"
        );
    }

    #[test]
    fn natural_name_sort() {
        assert!(name_key("File 9") < name_key("File 10"));
        assert!(name_key("apple") < name_key("Banana"));
    }

    #[test]
    fn browse_rows_have_folders_first() {
        let browser = sample();
        let rows = browser.browse_rows();
        assert_eq!(rows.len(), 2);
        let first = browser.node(rows[0].id).unwrap();
        assert_eq!(first.kind, EntryKind::Folder);
    }

    #[test]
    fn folders_only_hides_files_in_both_modes() {
        let mut browser = sample();
        browser.set_mode(Mode::Tree);
        browser.set_folders_only(true);
        let names = |rows: &[Row]| -> Vec<String> {
            rows.iter()
                .map(|r| browser.node(r.id).unwrap().name.clone())
                .collect()
        };
        assert_eq!(names(&browser.browse_rows()), vec!["Finanzen"]);
        assert_eq!(names(&browser.tree_rows()), vec!["My files", "Finanzen"]);
        browser.set_folders_only(false);
        assert_eq!(browser.browse_rows().len(), 2, "the file comes back");
    }

    #[test]
    fn single_mode_replaces_the_selection() {
        let mut browser = sample();
        browser.set_selection_mode(SelectionMode::Single);
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        let notes = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        browser.toggle(folder);
        browser.toggle(notes);
        assert_eq!(
            browser.selected().collect::<Vec<_>>(),
            vec![notes],
            "the second tap replaced the first"
        );
        browser.toggle_all();
        assert_eq!(
            browser.selected().collect::<Vec<_>>(),
            vec![NodeId::ROOT],
            "the header box ticks the scope itself as the one unit"
        );
        // Tapping a box covered by that unit clears it — there is no dissolving in single mode.
        browser.toggle(notes);
        assert!(
            !browser.has_selection(),
            "the covered tap unticks the only unit"
        );
        // Select-all under the mode selects the scope, never every child.
        browser.select_all();
        assert_eq!(browser.selected().collect::<Vec<_>>(), vec![NodeId::ROOT]);
    }

    #[test]
    fn file_extension_filter_keeps_folders_and_matching_files() {
        let mut browser = sample();
        browser.set_file_extensions(["txt"]);
        let names: Vec<String> = browser
            .browse_rows()
            .iter()
            .map(|r| browser.node(r.id).unwrap().name.clone())
            .collect();
        assert_eq!(names, vec!["Finanzen", "notes.txt"]);
        browser.set_file_extensions(["pdf"]);
        assert_eq!(
            browser.browse_rows().len(),
            1,
            "no pdf here, only the folder"
        );
        // Select-all inside the scope only reaches listed children.
        browser.select_all();
        let hidden = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        assert!(
            !browser.selected().any(|id| id == hidden),
            "an unlisted file is never selected by select-all"
        );
        browser.show_all_files();
        assert_eq!(
            browser.browse_rows().len(),
            2,
            "no filter: every file again"
        );
    }

    #[test]
    fn custom_column_sorts() {
        let mut browser = sample();
        let extension = browser.add_column(ExtensionColumn);
        browser.set_children(
            NodeId::ROOT,
            vec![
                Entry {
                    name: "a.pdf".into(),
                    kind: EntryKind::File,
                    size: None,
                    modified: None,
                },
                Entry {
                    name: "b.txt".into(),
                    kind: EntryKind::File,
                    size: None,
                    modified: None,
                },
                Entry {
                    name: "c.pdf".into(),
                    kind: EntryKind::File,
                    size: None,
                    modified: None,
                },
            ],
        );
        browser.sort_by_column(extension);
        let names: Vec<String> = browser
            .browse_rows()
            .iter()
            .map(|row| browser.node(row.id).unwrap().name.clone())
            .collect();
        assert_eq!(names, vec!["a.pdf", "c.pdf", "b.txt"]);
    }

    /// Test column: sorts files by their extension.
    #[derive(Debug)]
    struct ExtensionColumn;

    impl Column for ExtensionColumn {
        fn header(&self) -> String {
            "Ext".into()
        }

        fn cell(&self, _browser: &Browser, node: &Node) -> Cell {
            Cell::Text {
                text: node.name.rsplit('.').next().unwrap_or("").to_uppercase(),
                muted: true,
            }
        }

        fn compare(&self, a: &Node, b: &Node) -> Ordering {
            a.name
                .rsplit('.')
                .next()
                .unwrap_or("")
                .cmp(b.name.rsplit('.').next().unwrap_or(""))
        }
    }

    #[test]
    fn tree_rows_follow_expansion() {
        let mut browser = sample();
        let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        browser.set_mode(Mode::Tree);
        browser.activate(folder);
        let folder_node = browser.node(folder).unwrap();
        assert!(
            folder_node.children.is_empty(),
            "not loaded yet: caret but no children"
        );
        let ids: Vec<_> = browser.tree_rows().iter().map(|row| row.id).collect();
        // Root expanded: its folder (caret, still unloaded) and its file.
        let notes = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
        assert_eq!(ids, vec![NodeId::ROOT, folder, notes]);
    }

    #[test]
    fn epoch_formatting() {
        // 2021-04-07 17:45 UTC.
        assert_eq!(format_epoch(1_617_817_500), "Apr 7, 2021, 17:45");
        assert_eq!(format_bytes(372_820), "372.82 KB");
        assert_eq!(format_bytes(900), "900 B");
    }
}
