//! Selection by path, for pickers: preselecting entries the browser has not listed yet ([`select_path`](Browser::select_path)), reading the selection back as paths ([`selected_paths`](Browser::selected_paths)), and what a picker's confirm hands over ([`picked`](Browser::picked)).
//!
//! A preselected path whose folders are not listed yet waits as a *pending* unit: it counts as selected (the header's count, [`has_selection`](Browser::has_selection), [`selected_paths`](Browser::selected_paths)), its listed ancestors show the dot, and the listing that reaches it turns it into an ordinary selected node. It leaves the selection only the way any unit does — unticked through an ancestor's box, covered by a selected ancestor, or cleared.

use std::collections::HashSet;

use super::{Browser, EntryKind, NodeId, Selection};

/// The kinds of picking [`Browser::picker`] sets a browser up for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// One folder: folders only, [`Selection::Single`], and with nothing ticked the folder open in browse mode is the pick ([`set_pick_open_folder`](Browser::set_pick_open_folder)) — navigate into it and confirm, or tick it from its parent.
    Folder,
    /// Any number of folders: folders only, [`Selection::Multiple`].
    Folders,
    /// Any number of files and folders: [`Selection::Multiple`].
    Entries,
}

/// The components of a `/`-separated path below the root; empty components (doubled, leading or trailing slashes) are dropped, so `""` and `"/"` name the root.
fn components(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether `path` lies strictly below `prefix`.
fn strictly_below(path: &[String], prefix: &[String]) -> bool {
    path.len() > prefix.len() && path.starts_with(prefix)
}

/// Where a pending path's walk through the listed tree ends.
enum Walk {
    /// Every component is listed: this node is the path's entry.
    Found(NodeId),
    /// A selected node lies on the way: the path is already covered by it.
    Covered,
    /// The walk stops at a folder whose listing is unknown or lacks the next name; the nodes passed so far (root first) are the path's listed ancestors.
    Waiting(Vec<NodeId>),
}

impl Browser {
    /// A browser set up for one of the [`Pick`] kinds; the settings it applies stay individually adjustable afterwards. Confirm with [`picked`](Self::picked).
    pub fn picker(root: impl Into<String>, pick: Pick) -> Self {
        let mut browser = Self::new(root);
        match pick {
            Pick::Folder => {
                browser.set_selection(Selection::Single);
                browser.set_folders_only(true);
                browser.set_pick_open_folder(true);
            }
            Pick::Folders => browser.set_folders_only(true),
            Pick::Entries => {}
        }
        browser
    }

    /// With nothing selected, let [`picked`](Self::picked) answer the folder open in browse mode — the "use this folder" of a folder picker. Default: off.
    pub fn set_pick_open_folder(&mut self, yes: bool) {
        self.pick_open_folder = yes;
    }

    /// Whether the open folder stands in for an empty selection; see [`set_pick_open_folder`](Self::set_pick_open_folder).
    pub fn picks_open_folder(&self) -> bool {
        self.pick_open_folder
    }

    /// The path of `id` below the root, its names joined with `/`: `""` for the root and for an id the browser does not know.
    pub fn path_of(&self, id: NodeId) -> String {
        self.path_below_root(id).join("/")
    }

    /// Select the entry at `path` (names below the root, `/`-separated; `""` is the root) as a unit, the way ticking its box would — except that its folders need not be listed yet: the path then waits as a pending unit and turns into the listed entry once [`set_children`](Self::set_children) reaches it. A path under a selected ancestor is already covered and changes nothing; selecting a folder drops the selections below it. Where a folder and a file share the last name, the folder is taken. [`Selection::Single`] replaces whatever was selected, [`Selection::None`] ignores the call.
    pub fn select_path(&mut self, path: &str) {
        if self.selection == Selection::None {
            return;
        }
        let path = components(path);
        if self.selection == Selection::Single {
            self.selected.clear();
            self.pending.clear();
        } else if self.pending.contains(&path) {
            return;
        }
        self.settle(path);
    }

    /// [`select_path`](Self::select_path) for each path in turn — the app's saved state (exclusions, hidden folders) ticked before the picker opens.
    pub fn select_paths<S: AsRef<str>>(&mut self, paths: impl IntoIterator<Item = S>) {
        for path in paths {
            self.select_path(path.as_ref());
        }
    }

    /// The selection as paths below the root (see [`path_of`](Self::path_of)): the listed units in selection order, then the pending ones in the order they were selected. A unit inside another selected unit is left out — the outer one stands for it. Unlike [`selected`](Self::selected), this includes the pending paths, so it is the full answer for a picker.
    pub fn selected_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .selected
            .iter()
            .filter(|id| self.selected_ancestor(**id).is_none())
            .map(|id| self.path_of(*id))
            .collect();
        paths.extend(self.pending.iter().map(|path| path.join("/")));
        paths
    }

    /// What a picker's confirm hands over: the [`selected_paths`](Self::selected_paths), or — with nothing selected, in browse mode and with [`set_pick_open_folder`](Self::set_pick_open_folder) on — the open folder's path alone. Empty when there is nothing to pick.
    pub fn picked(&self) -> Vec<String> {
        if self.falls_back_to_open_folder() {
            vec![self.path_of(self.current)]
        } else {
            self.selected_paths()
        }
    }

    /// Whether [`picked`](Self::picked) has anything to hand over — the default enable rule of the cabinet's [action button](crate::ActionButton).
    pub fn has_pick(&self) -> bool {
        self.has_selection() || self.falls_back_to_open_folder()
    }

    fn falls_back_to_open_folder(&self) -> bool {
        self.pick_open_folder && !self.has_selection() && self.mode == super::Mode::Browse
    }

    /// The number of selected units, pending ones included — the header's count line.
    pub(crate) fn selection_count(&self) -> usize {
        self.selected.iter().count() + self.pending.len()
    }

    /// Walk `path` through the listed tree from the root.
    fn walk(&self, path: &[String]) -> Walk {
        let mut here = NodeId::ROOT;
        let mut passed = Vec::with_capacity(path.len());
        for (index, name) in path.iter().enumerate() {
            if self.selected.contains(&here) {
                return Walk::Covered;
            }
            passed.push(here);
            let Some(node) = self.nodes.get(&here) else {
                return Walk::Waiting(passed);
            };
            if !node.children_known {
                return Walk::Waiting(passed);
            }
            let last = index + 1 == path.len();
            let named = |kind: EntryKind| {
                node.children.iter().copied().find(|child| {
                    self.nodes
                        .get(child)
                        .is_some_and(|n| n.kind == kind && n.name == *name)
                })
            };
            let next = match named(EntryKind::Folder) {
                Some(folder) => Some(folder),
                None if last => named(EntryKind::File),
                None => None,
            };
            match next {
                Some(child) => here = child,
                None => return Walk::Waiting(passed),
            }
        }
        Walk::Found(here)
    }

    /// Resolve `path` as far as the tree allows: select its node when listed, drop it when covered, keep it pending otherwise.
    fn settle(&mut self, path: Vec<String>) {
        match self.walk(&path) {
            Walk::Found(id) => self.select_unit(id),
            Walk::Covered => {}
            Walk::Waiting(_) => {
                // Pending units below this path are covered by it now.
                self.pending.retain(|other| !strictly_below(other, &path));
                self.pending.push(path);
            }
        }
    }

    /// Re-try every pending path after a listing arrived; called by [`set_children`](Self::set_children).
    pub(super) fn settle_pending(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        for path in std::mem::take(&mut self.pending) {
            self.settle(path);
        }
    }

    /// Select a listed node as a unit: the explicit selections and the pending paths below it go, because it now stands for its whole subtree. A node already covered by a selected ancestor stays as it is. Single mode keeps only this one.
    pub(super) fn select_unit(&mut self, id: NodeId) {
        if self.selection == Selection::Single {
            self.selected.clear();
            self.pending.clear();
            self.selected.insert(id);
            return;
        }
        if self.selected.contains(&id) || self.selected_ancestor(id).is_some() {
            return;
        }
        self.selected.insert(id);
        let below: Vec<NodeId> = self
            .selected
            .iter()
            .copied()
            .filter(|other| *other != id && super::node::is_below(self, *other, id))
            .collect();
        for other in below {
            self.selected.remove(&other);
        }
        self.drop_pending_below(id);
    }

    /// Drop the pending paths strictly below the node `id`.
    pub(super) fn drop_pending_below(&mut self, id: NodeId) {
        if self.pending.is_empty() {
            return;
        }
        let prefix: Vec<String> = self
            .path_below_root(id)
            .into_iter()
            .map(str::to_owned)
            .collect();
        self.pending.retain(|path| !strictly_below(path, &prefix));
    }

    /// The listed ancestors of every pending path — the nodes that show the dot for a selection they cannot list yet.
    pub(super) fn pending_marks(&self, marks: &mut HashSet<NodeId>) {
        for path in &self.pending {
            if let Walk::Waiting(passed) = self.walk(path) {
                marks.extend(passed);
            }
        }
    }
}
