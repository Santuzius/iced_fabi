//! The node vocabulary of the tree: identity, entries and node state. Pure data — the logic over it lives in [`Browser`](crate::Browser).

use super::Browser;

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

/// One listing entry handed to [`set_children`](crate::Browser::set_children). The display strings for `size` and `modified` are produced by the browser's formatters, so the app only passes raw values. Build one with [`folder`](Self::folder) / [`file`](Self::file) and their `.size()` / `.modified()`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
    /// Size in bytes; `None` renders as an empty cell (and sorts as smallest).
    pub size: Option<u64>,
    /// Seconds since the Unix epoch; `None` renders empty and sorts first ascending.
    pub modified: Option<i64>,
}

impl Entry {
    /// A folder entry; only `name` is required.
    pub fn folder(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: EntryKind::Folder,
            size: None,
            modified: None,
        }
    }

    /// A file entry; only `name` is required.
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: EntryKind::File,
            size: None,
            modified: None,
        }
    }

    pub fn size(mut self, bytes: u64) -> Self {
        self.size = Some(bytes);
        self
    }

    pub fn modified(mut self, seconds_since_epoch: i64) -> Self {
        self.modified = Some(seconds_since_epoch);
        self
    }
}

/// A node the browser knows about, filled by [`set_children`](crate::Browser::set_children). Read-only for apps: the browser keeps it current.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Node {
    /// This node's identity, derived once at creation from the parent's id, kind and name — unlike a re-derivation, the root's id is [`NodeId::ROOT`] this way.
    pub id: NodeId,
    pub name: String,
    pub kind: EntryKind,
    pub parent: Option<NodeId>,
    /// Children in listing order; the view sorts them per [`sort_by_column`](crate::Browser::sort_by_column).
    pub children: Vec<NodeId>,
    /// Whether the children of this node are known (it was listed at least once and the listing stuck).
    pub children_known: bool,
    /// A load requested via [`Outcome::NeedsLoad`](crate::Outcome::NeedsLoad) that has not been answered yet.
    pub loading: bool,
    /// The last load of this node failed; a later [`Browser::activate`](crate::Browser::activate) retries.
    pub failed: bool,
    pub size: Option<u64>,
    pub modified: Option<i64>,
}

/// Whether `id` is a proper descendant of `below` (walking parents, so it stays correct as the tree is rebuilt). Shared by the selection and removal logic.
pub(super) fn is_below(browser: &Browser, id: NodeId, below: NodeId) -> bool {
    let mut parent = browser.nodes.get(&id).and_then(|node| node.parent);
    while let Some(here) = parent {
        if here == below {
            return true;
        }
        parent = browser.nodes.get(&here).and_then(|node| node.parent);
    }
    false
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
    /// One level at a time: the children of [`current`](crate::Browser::current), with the [`path`](crate::Browser::path) above it as breadcrumbs (Android Documents-App style).
    Browse,
    /// The whole tree in place, every level expandable; selection can span levels.
    Tree,
}

/// How many entries the selection can hold — and whether it can hold any at all. Orthogonal to [`Checkboxes`] (where the boxes are); the default is [`Multiple`](Self::Multiple).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Selection {
    /// As many units as the user likes, across levels.
    #[default]
    Multiple,
    /// At most one entry: a new tick replaces whatever was selected.
    Single,
    /// No selection at all: a pure file view. The boxes never appear, ticking does nothing, and tapping a file reports [`Outcome::FileOpened`](crate::Outcome::FileOpened).
    None,
}

/// How the selection boxes appear — orthogonal to [`Selection`] (how many can be picked); the default is [`Shown`](Self::Shown).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Checkboxes {
    /// A box sits on every row and on the header.
    #[default]
    Shown,
    /// The rows start without boxes: pressing and holding a row for at least [`LONG_PRESS`](super::long_press::LONG_PRESS) reveals them on every row and ticks the held row itself — until the app calls [`disarm`](crate::Browser::disarm) or sets this setting again. While hidden, a row press keeps its normal meaning (folders navigate, files tick only under a [`Selection`] other than [`None`](Selection::None), which then happens invisibly).
    OnLongPress,
    /// The box column stays hidden — the selection lives on, visible only through row backgrounds and the header's count line.
    Hidden,
}

#[cfg(test)]
mod tests {
    use crate::browser::{EntryKind, NodeId};

    #[test]
    fn derived_ids_are_stable() {
        let a = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        let b = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
        assert_eq!(a, b);
        assert_ne!(a, NodeId::derive(NodeId::ROOT, EntryKind::File, "Finanzen"));
    }
}
