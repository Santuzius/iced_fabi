//! Material-style unit selection: ticking, the select-all box, markers, and the rules about covered entries.

use std::collections::HashSet;

use super::{Browser, Checkboxes, EntryKind, NodeId, Selection};
use crate::Marker;

/// The explicit unit selections in selection order: `order` gives `iter` and `last` their tick order, `set` answers `contains` in constant time. The two are kept in step by the methods below and never touched directly; the field on [`Browser`] stays inside the `browser` module.
#[derive(Debug, Default)]
pub(crate) struct Selected {
    order: Vec<NodeId>,
    set: HashSet<NodeId>,
}

impl Selected {
    /// Adds an id at the end of the selection order; an id already selected keeps its position. Whether it was new.
    pub fn insert(&mut self, id: NodeId) -> bool {
        if self.set.insert(id) {
            self.order.push(id);
            true
        } else {
            false
        }
    }

    /// Drops an id from the selection order. Whether it was selected.
    pub fn remove(&mut self, id: &NodeId) -> bool {
        if self.set.remove(id) {
            let position = self
                .order
                .iter()
                .position(|entry| entry == id)
                .expect("set and order kept in step");
            self.order.remove(position);
            true
        } else {
            false
        }
    }

    pub fn contains(&self, id: &NodeId) -> bool {
        self.set.contains(id)
    }

    /// The selected ids in selection order, oldest first.
    pub fn iter(&self) -> std::slice::Iter<'_, NodeId> {
        self.order.iter()
    }

    /// The most recently selected id.
    pub fn last(&self) -> Option<NodeId> {
        self.order.last().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.set.clear();
    }

    /// Adds the ids not yet selected, at the end of the order and in the given sequence.
    pub fn extend(&mut self, ids: impl IntoIterator<Item = NodeId>) {
        for id in ids {
            self.insert(id);
        }
    }
}

impl Browser {
    /// How many entries can be selected at once.
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// How the selection boxes appear.
    pub fn checkboxes(&self) -> Checkboxes {
        self.checkboxes
    }

    /// How many entries can be selected at once — or none at all. Entering [`None`](Selection::None) drops any existing selection; entering [`Single`](Selection::Single) keeps the most recently selected unit and drops the rest; [`Multiple`](Selection::Multiple) keeps everything. Default: [`Multiple`](Selection::Multiple).
    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = selection;
        match selection {
            Selection::None => self.clear_selection(),
            Selection::Single => {
                // Keep at most one unit: the most recently selected listed one, else the last pending path; the rest go.
                let last = self.selected.last();
                let last_pending = self.pending.pop();
                self.clear_selection();
                match (last, last_pending) {
                    (Some(id), _) => {
                        self.selected.insert(id);
                    }
                    (None, Some(path)) => self.pending.push(path),
                    (None, None) => {}
                }
            }
            Selection::Multiple => {}
        }
    }

    /// How the selection boxes appear. Setting it always re-hides a column revealed by a long press. Default: [`Shown`](Checkboxes::Shown).
    pub fn set_checkboxes(&mut self, boxes: Checkboxes) {
        self.checkboxes = boxes;
        self.held = None;
        self.armed = false;
    }

    /// Whether the selection boxes should be drawn right now: no with [`Selection::None`](Selection::None) (a pure view has no boxes whatever [`Checkboxes`] says), otherwise per the [`Checkboxes`](Checkboxes) setting — always in [`Shown`](Checkboxes::Shown), only after a revealing long press in [`OnLongPress`](Checkboxes::OnLongPress), never in [`Hidden`](Checkboxes::Hidden).
    pub fn boxes_visible(&self) -> bool {
        if self.selection == Selection::None {
            return false;
        }
        match self.checkboxes {
            Checkboxes::Shown => true,
            Checkboxes::OnLongPress => self.armed,
            Checkboxes::Hidden => false,
        }
    }

    /// The explicit unit selections in selection order, oldest first (effective selection is this set — a selected folder stands for its whole subtree).
    pub fn selected(&self) -> impl Iterator<Item = NodeId> + use<'_> {
        self.selected.iter().copied()
    }

    /// Whether anything at all is selected, [pending paths](Self::select_path) included.
    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty() || !self.pending.is_empty()
    }

    /// Untick everything, [pending paths](Self::select_path) included (e.g. from the app's cancel button).
    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.pending.clear();
    }

    /// The marker of the header's select-all box: ticked when the scope, one of its ancestors, or every one of its children is selected, a minus when something below the scope is.
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
            || (self.units_cover()
                && !children.is_empty()
                && children.iter().all(|child| self.selected.contains(child)))
        {
            Marker::Selected
        } else if self.has_selected_below(scope) {
            Marker::Partial
        } else {
            Marker::Unselected
        }
    }

    /// The header's select-all box clicked: select every child of the scope when nothing in it is selected, otherwise clear the selection within the scope. When the scope is only ticked because an ancestor is selected, that unticks the ancestor — the same rule as for a covered row's box, see [`toggle`](Self::toggle). In [`Selection::Single`] the box instead toggles the scope itself as the one unit.
    pub fn toggle_all(&mut self) {
        let scope = self.scope();
        if self.selection == Selection::Single {
            let on = self.selected.contains(&scope) || self.selected_ancestor(scope).is_some();
            self.clear_selection();
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
                .filter(|id| *id == scope || super::node::is_below(self, *id, scope))
                .collect();
            for id in inside {
                self.selected.remove(&id);
            }
            self.drop_pending_below(scope);
        }
    }

    /// Select every child of the scope node — the current directory in browse mode, the root in tree mode — provided it is listed. Does nothing while the scope itself or one of its ancestors is selected (the children are already covered). In [`Selection::Single`](Selection::Single) this selects the scope itself as the one unit.
    pub fn select_all(&mut self) {
        let scope = self.scope();
        if self.selected.contains(&scope) || self.selected_ancestor(scope).is_some() {
            return;
        }
        if self.selection == Selection::Single {
            self.clear_selection();
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
        // Pending paths that run through a child just selected are covered by it now.
        self.settle_pending();
    }

    /// The marker this node shows: a tick when it is selected, explicitly or covered by a selected ancestor (in [`Selection::Multiple`] only), a minus when something below it is selected, otherwise an empty box. A covered node therefore looks selected, mirroring what [`toggle`](Self::toggle) does when one of its boxes is tapped.
    pub fn marker(&self, id: NodeId) -> Marker {
        if self.selected.contains(&id) || self.selected_ancestor(id).is_some() {
            Marker::Selected
        } else if self.has_selected_below(id) {
            Marker::Partial
        } else {
            Marker::Unselected
        }
    }

    /// The marker `id` shows, given what the row walk already knows: `covered` says an ancestor of this row is selected (Material covers a whole subtree with one tick), and `partial` is the frame's set of nodes with something selected below them (see [`partial_marks`](Self::partial_marks)). Same answer as [`marker`](Self::marker), but neither the ancestor chain nor the subtree is walked per row.
    pub(super) fn marker_with(
        &self,
        id: NodeId,
        partial: &HashSet<NodeId>,
        covered: bool,
    ) -> Marker {
        if covered || self.selected.contains(&id) {
            Marker::Selected
        } else if partial.contains(&id) {
            Marker::Partial
        } else {
            Marker::Unselected
        }
    }

    /// Every node with a selected node somewhere below it: the parents of the selected entries, collected once for a row list. Walking up stops at a node already in the set, so a shared chain is visited once; without this set, each row would walk its own whole loaded subtree.
    pub(super) fn partial_marks(&self) -> HashSet<NodeId> {
        let mut partial = HashSet::new();
        self.pending_marks(&mut partial);
        for id in self.selected.iter() {
            let mut parent = self.nodes.get(id).and_then(|node| node.parent);
            while let Some(here) = parent {
                if !partial.insert(here) {
                    break;
                }
                parent = self.nodes.get(&here).and_then(|node| node.parent);
            }
        }
        partial
    }

    /// Whether anything below `id` is explicitly selected: a listed node, or a pending path running through it.
    fn has_selected_below(&self, id: NodeId) -> bool {
        self.has_listed_selection_below(id) || {
            let mut marks = HashSet::new();
            self.pending_marks(&mut marks);
            marks.contains(&id)
        }
    }

    /// Whether a listed node below `id` (only loaded parts can be known) is explicitly selected.
    fn has_listed_selection_below(&self, id: NodeId) -> bool {
        let Some(node) = self.nodes.get(&id) else {
            return false;
        };
        node.children
            .iter()
            .any(|child| self.selected.contains(child) || self.has_listed_selection_below(*child))
    }

    /// Toggle the unit selection of a node. With [`Selection::None`](Selection::None) this does nothing.
    ///
    /// Rules (Material-inspired): a node under a selected ancestor shows a tick because the ancestor's unit covers it, and clicking its box removes just that entry from the selection — the sibling rows stay ticked and the ancestors show a minus (the unit is dissolved along the chain from the ancestor down); selecting a folder that had explicit selections below clears them, because the folder now stands for its whole subtree. In [`Selection::Single`](Selection::Single) a selected folder does not cover its content: tapping the selected entry leaves nothing selected, tapping any other — one inside a selected folder too — makes it the one selection.
    pub fn toggle(&mut self, id: NodeId) {
        if self.selection == Selection::None || !self.nodes.contains_key(&id) {
            return;
        }
        if self.selection == Selection::Single {
            let on = self.selected.contains(&id) || self.selected_ancestor(id).is_some();
            self.clear_selection();
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
        self.select_unit(id);
    }

    /// Unticks one covered entry while the rest of the selected `ancestor`'s subtree stays selected: the unit is dissolved along the chain from the ancestor down to `id`, re-entering every listed child (hidden ones, e.g. files under [`set_folders_only`](Self::set_folders_only), are dropped) as an explicit unit except the one the chain leads through. That loses nothing: `id` shows on screen, so every folder on the chain is expanded and listed, and the re-entered siblings cover their unloaded subtrees themselves. The chain's folders end up Partial, `id` itself Unselected.
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
            // Only listed siblings re-enter, as with select-all: a folders-only picker must not end up with file units it never showed.
            for sibling in siblings {
                if sibling != *step && self.is_listed(sibling) {
                    self.selected.insert(sibling);
                }
            }
            level = *step;
        }
    }

    /// Whether a selected folder stands for its whole subtree: in [`Selection::Multiple`] only. A [`Single`](Selection::Single) pick is the one entry itself — choosing a folder does not choose what is inside it, so its content neither shows ticked nor can be unticked out of it.
    pub(super) fn units_cover(&self) -> bool {
        self.selection == Selection::Multiple
    }

    /// The selected ancestor whose unit covers `id`; always `None` unless [units cover](Self::units_cover) their subtree.
    pub(super) fn selected_ancestor(&self, id: NodeId) -> Option<NodeId> {
        if !self.units_cover() {
            return None;
        }
        let mut parent = self.nodes.get(&id).and_then(|node| node.parent);
        while let Some(here) = parent {
            if self.selected.contains(&here) {
                return Some(here);
            }
            parent = self.nodes.get(&here).and_then(|node| node.parent);
        }
        None
    }

    /// What deleting the current selection would remove, as (folders, files): every selected entry itself plus everything listed beneath it. A selected folder the browser never listed counts as the folder alone — its contents are unknown — and a selection nested inside another selected entry is counted once, at its own root, never as part of the outer subtree too.
    #[must_use]
    pub fn selected_entry_counts(&self) -> (usize, usize) {
        let (mut folders, mut files) = (0, 0);
        let mut stack: Vec<NodeId> = self.selected.iter().copied().collect();
        while let Some(id) = stack.pop() {
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            if node.kind == EntryKind::Folder {
                folders += 1;
            } else {
                files += 1;
            }
            stack.extend(
                node.children
                    .iter()
                    .filter(|child| !self.selected.contains(child))
                    .copied(),
            );
        }
        (folders, files)
    }
}
