//! Material-style unit selection: ticking, the select-all box, markers, and the rules about covered entries.

use super::{Browser, NodeId, SelectionMode, SelectionUi};
use crate::Marker;

impl Browser {
    pub fn selection_mode(&self) -> SelectionMode {
        self.selection_mode
    }

    pub fn selection_ui(&self) -> SelectionUi {
        self.selection_ui
    }

    /// Set the widget mode; switching to [`ViewOnly`](SelectionUi::ViewOnly) drops any existing selection, and switching modes always re-hides a revealed long-press box column. Default: [`Select`](SelectionUi::Select).
    pub fn set_selection_ui(&mut self, ui: SelectionUi) {
        self.selection_ui = ui;
        self.held = None;
        self.armed = false;
        if ui == SelectionUi::ViewOnly {
            self.selected.clear();
        }
    }

    /// Whether the selection boxes should be drawn right now: always in [`Select`](SelectionUi::Select), only after a revealing long press in [`LongPress`](SelectionUi::LongPress), never in [`ViewOnly`](SelectionUi::ViewOnly). (The cabinet's own checkbox column setting can still hide the column; the widget combines both.)
    pub fn boxes_visible(&self) -> bool {
        match self.selection_ui {
            SelectionUi::ViewOnly => false,
            SelectionUi::LongPress => self.armed,
            SelectionUi::Select => true,
        }
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
                .filter(|id| *id == scope || super::node::is_below(self, *id, scope))
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

    /// Toggle the unit selection of a node. In [`ViewOnly`](SelectionUi::ViewOnly) this does nothing.
    ///
    /// Rules (Material-inspired): a node under a selected ancestor shows a tick because the ancestor's unit covers it, and clicking its box removes just that entry from the selection — the sibling rows stay ticked and the ancestors become dots (the unit is dissolved along the chain from the ancestor down); selecting a folder that had explicit selections below clears them, because the folder now stands for its whole subtree. In [`SelectionMode::Single`] any tap first clears everything: tapping the (possibly covered) entry that reads ticked leaves nothing selected, tapping another makes it the one selection.
    pub fn toggle(&mut self, id: NodeId) {
        if self.selection_ui == SelectionUi::ViewOnly || !self.nodes.contains_key(&id) {
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
