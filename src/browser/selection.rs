//! Material-style unit selection: ticking, the select-all box, markers, and the rules about covered entries.

use std::collections::HashSet;

use super::{Browser, Checkboxes, NodeId, Selection};
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
            Selection::None => self.selected.clear(),
            Selection::Single => {
                // Keep at most one unit: the most recently selected one, the rest go.
                let last = self.selected.last();
                self.selected.clear();
                if let Some(id) = last {
                    self.selected.insert(id);
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

    /// Whether anything at all is selected.
    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    /// Untick everything (e.g. from the app's cancel button).
    pub fn clear_selection(&mut self) {
        self.selected.clear();
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

    /// The header's select-all box clicked: select every child of the scope when nothing in it is selected, otherwise clear the selection within the scope. When the scope is only ticked because an ancestor is selected, that unticks the ancestor — the same rule as for a covered row's box, see [`toggle`](Self::toggle). In [`Selection::Single`] the box instead toggles the scope itself as the one unit.
    pub fn toggle_all(&mut self) {
        let scope = self.scope();
        if self.selection == Selection::Single {
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
                .filter(|id| *id == scope || super::node::is_below(self, *id, scope))
                .collect();
            for id in inside {
                self.selected.remove(&id);
            }
        }
    }

    /// Select every child of the scope node — the current directory in browse mode, the root in tree mode — provided it is listed. Does nothing while the scope itself or one of its ancestors is selected (the children are already covered). In [`Selection::Single`](Selection::Single) this selects the scope itself as the one unit.
    pub fn select_all(&mut self) {
        let scope = self.scope();
        if self.selected.contains(&scope) || self.selected_ancestor(scope).is_some() {
            return;
        }
        if self.selection == Selection::Single {
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

    /// Whether anything at or below `id` (only loaded parts can be known) is explicitly selected.
    fn has_selected_below(&self, id: NodeId) -> bool {
        let Some(node) = self.nodes.get(&id) else {
            return false;
        };
        node.children
            .iter()
            .any(|child| self.selected.contains(child) || self.has_selected_below(*child))
    }

    /// Toggle the unit selection of a node. With [`Selection::None`](Selection::None) this does nothing.
    ///
    /// Rules (Material-inspired): a node under a selected ancestor shows a tick because the ancestor's unit covers it, and clicking its box removes just that entry from the selection — the sibling rows stay ticked and the ancestors become dots (the unit is dissolved along the chain from the ancestor down); selecting a folder that had explicit selections below clears them, because the folder now stands for its whole subtree. In [`Selection::Single`](Selection::Single) any tap first clears everything: tapping the (possibly covered) entry that reads ticked leaves nothing selected, tapping another makes it the one selection.
    pub fn toggle(&mut self, id: NodeId) {
        if self.selection == Selection::None || !self.nodes.contains_key(&id) {
            return;
        }
        if self.selection == Selection::Single {
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
            .filter(|other| *other != id && super::node::is_below(self, *other, id))
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

    pub(super) fn selected_ancestor(&self, id: NodeId) -> Option<NodeId> {
        let mut parent = self.nodes.get(&id).and_then(|node| node.parent);
        while let Some(here) = parent {
            if self.selected.contains(&here) {
                return Some(here);
            }
            parent = self.nodes.get(&here).and_then(|node| node.parent);
        }
        None
    }
}
