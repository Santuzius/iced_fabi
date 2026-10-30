//! The listing feed from the application ([`Browser::set_children`]) and the removal side: pruning gone entries, deleting selected ones, and the fallback when the view points at a pruned node.

use super::{Browser, Node, NodeId, Outcome, node::Entry};
use std::collections::HashSet;

impl Browser {
    /// Whether a load of this node is requested and unanswered.
    pub fn loading(&self, id: NodeId) -> bool {
        self.nodes.get(&id).is_some_and(|n| n.loading)
    }

    /// Remove the selected entries from the tree, the way a delete the app performed in the backend must show up; see [`remove`](Self::remove). The selection ends up empty.
    pub fn remove_selected(&mut self) {
        let selected: Vec<NodeId> = self.selected.iter().copied().collect();
        self.remove(&selected);
        self.clear_selection();
    }

    /// Remove entries from the tree, the way a delete the app performed in the backend must show up: each entry is spliced out of its parent's listing and its whole subtree's state (nodes, expansion, selections, pending paths) is pruned. Entries below another listed entry go with it. Refuses to delete the root scope.
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
                    .any(|other| *other != *id && super::node::is_below(self, *id, *other))
            })
            .filter_map(|id| Some((self.nodes.get(&id)?.parent?, id)))
            .collect();
        for (parent, id) in victims {
            self.drop_pending_below(id);
            if let Some(node) = self.nodes.get_mut(&parent) {
                node.children.retain(|child| *child != id);
            }
            self.prune(id);
        }
    }

    /// Answer a [`NeedsLoad`](Outcome::NeedsLoad): install `entries` as the children of `id`.
    ///
    /// Does nothing if `id` is gone (pruned by an earlier refresh — the app's own staleness guard can also skip the call). Existing child nodes keep their id, so their expansion and selections survive; children that disappeared are pruned with their whole subtree. Two entries naming the same child (a backend that repeats itself) count once: the first one fills the node, the later ones are ignored.
    pub fn set_children(&mut self, id: NodeId, entries: Vec<Entry>) {
        if !self.nodes.contains_key(&id) {
            return;
        }
        let mut next = Vec::with_capacity(entries.len());
        let mut emptied: Vec<NodeId> = Vec::new();
        let mut fresh: HashSet<NodeId> = HashSet::with_capacity(entries.len());
        for entry in entries {
            let child_id = NodeId::derive(id, entry.kind, &entry.name);
            if !fresh.insert(child_id) {
                continue;
            }
            let node = self.nodes.entry(child_id).or_insert(Node {
                id: child_id,
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
            if entry.empty {
                // Known empty: nothing to load, and whatever an earlier listing put below it is gone.
                node.children_known = true;
                node.loading = false;
                node.failed = false;
                emptied.append(&mut node.children);
            } else if node.children_known && node.children.is_empty() {
                // Empty before, maybe not any more: the next activation asks again.
                node.children_known = false;
            }
            next.push(child_id);
        }
        for gone in emptied {
            self.prune(gone);
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
            .filter(|child| !fresh.contains(child))
            .collect();
        parent.children = next;
        for child in stale {
            self.prune(child);
        }
        // A pending path may have become reachable through this listing.
        self.settle_pending();
    }

    /// Report that the listing for `id` failed; the node stays a (still expandable) branch and a later [`activate`](Browser::activate) or the widget's retry note asks again.
    pub fn load_failed(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.loading = false;
            node.failed = true;
        }
    }

    /// The retry behind the widget's "could not load" note: clear the failure and ask the app for the listing again.
    pub(super) fn retry(&mut self, id: NodeId) -> Outcome {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.failed = false;
            node.loading = false;
        }
        self.request_load(id)
    }

    /// Mark `id` as loading now and ask the app for it next time `activate` reaches it (used for the implicit load of the root on open).
    pub(super) fn request_load(&mut self, id: NodeId) -> Outcome {
        let Some(node) = self.nodes.get_mut(&id) else {
            return Outcome::None;
        };
        if node.children_known || node.loading {
            return Outcome::None;
        }
        node.loading = true;
        Outcome::NeedsLoad(id)
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
