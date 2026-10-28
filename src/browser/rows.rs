//! The row lists the widget draws: [`Row`] building for browse and tree mode, sorting, and the visibility settings that decide which entries appear at all.

use std::cmp::Ordering;
use std::collections::HashSet;

use super::{Browser, Node, NodeId, node::EntryKind};
use crate::browser::Mode;
use crate::natural::{NameChunk, name_key};

/// One line of the list: which node, how deep it sits (tree mode indents by this), and how its controls look. The view builds rows for browse mode and tree mode alike, so it never branches on the mode itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Row {
    pub id: NodeId,
    pub depth: usize,
    /// Whether a caret is shown (tree mode only): folders and unlisted branches.
    pub expandable: bool,
    pub expanded: bool,
    pub marker: super::Marker,
}

impl Browser {
    /// Whether folders sort before files regardless of the sort key. Default: true.
    pub fn folders_first(&self) -> bool {
        self.folders_first
    }

    /// Turn folder-first ordering on or off.
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
    pub(super) fn is_listed(&self, id: NodeId) -> bool {
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

    /// The node whose children the list shows at the top level: the current directory in browse mode, the root in tree mode.
    pub(super) fn scope(&self) -> NodeId {
        if self.mode == Mode::Tree {
            NodeId::ROOT
        } else {
            self.current
        }
    }

    /// The row for the browse view: the sorted children of the current node.
    pub(crate) fn browse_rows(&self) -> Vec<Row> {
        let Some(current) = self.nodes.get(&self.current) else {
            return Vec::new();
        };
        let partial = self.partial_marks();
        // One question for the whole listing: is the scope itself, or one of its ancestors, a selected unit? That covers every row here.
        let covered = self.units_cover()
            && (self.selected.contains(&self.current)
                || self.selected_ancestor(self.current).is_some());
        self.sorted(&current.children)
            .into_iter()
            .filter(|id| self.is_listed(*id))
            .map(|id| Row {
                id,
                depth: 0,
                expandable: false,
                expanded: false,
                marker: self.marker_with(id, &partial, covered),
            })
            .collect()
    }

    /// The rows for the tree view: every expanded branch from the root down, the root included.
    pub(crate) fn tree_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        let partial = self.partial_marks();
        self.tree_rows_at(NodeId::ROOT, 0, false, &partial, &mut rows);
        rows
    }

    /// One row and, when the branch is open, its children. `covered` says an ancestor of this node is a selected unit; the walk passes it down once it steps below a selected node.
    fn tree_rows_at(
        &self,
        id: NodeId,
        depth: usize,
        covered: bool,
        partial: &HashSet<NodeId>,
        rows: &mut Vec<Row>,
    ) {
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
            marker: self.marker_with(id, partial, covered),
        });
        // A file has no children; an unlisted folder shows no child rows until loaded, but keeps its caret (see `expandable`).
        if expanded {
            let below = covered || (self.units_cover() && self.selected.contains(&id));
            for child in self.sorted(&node.children) {
                self.tree_rows_at(child, depth + 1, below, partial, rows);
            }
        }
    }

    /// The children of `id` in display order, by the sorted column ([`Column::compare`](crate::Column::compare)), ties broken by name; `folders_first` wins over both.
    fn sorted(&self, children: &[NodeId]) -> Vec<NodeId> {
        // The name key is built once per child, not once per comparison: 10 000 children are compared about 130 000 times, and `name_key` allocates.
        let mut nodes: Vec<(&Node, Vec<NameChunk>)> = children
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .map(|node| (node, name_key(&node.name)))
            .collect();
        let column = &self.columns[self.sort_column.min(self.columns.len() - 1)].column;
        let ascending = self.sort_ascending;
        let folders_first = self.folders_first;
        nodes.sort_by(|(a, name_a), (b, name_b)| {
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
            ordered.then_with(|| name_a.cmp(name_b))
        });
        nodes.iter().map(|(node, _)| node.id).collect()
    }
}
