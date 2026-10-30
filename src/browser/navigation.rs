//! Navigation: entering folders (browse and tree), breadcrumbs, mode, and the queries about where the view stands.

use super::{Browser, Checkboxes, Mode, Node, NodeId, Outcome, Selection, node::EntryKind};

impl Browser {
    /// Browse or tree, as it currently stands.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Switches mode; entering tree mode keeps the current node expanded.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if mode == Mode::Tree {
            self.expanded.insert(self.current);
        }
    }

    /// The browse/tree switch: the other [`Mode`].
    pub fn toggle_mode(&mut self) {
        self.set_mode(match self.mode {
            Mode::Browse => Mode::Tree,
            Mode::Tree => Mode::Browse,
        });
    }

    /// The node whose children the browse mode shows.
    pub fn current(&self) -> NodeId {
        self.current
    }

    /// Breadcrumb from the root to the current node.
    pub fn path(&self) -> &[NodeId] {
        &self.path
    }

    /// The node behind an id, if the browser knows it (a deep id may not be listed yet).
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Node names from the root down to `id` — enough to rebuild the backend's path.
    /// An id the browser does not know gives an empty chain.
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

    /// The names from the root's children down to `id` — the path a backend expects, without the root's display name (the root itself gives an empty path, and so does an id the browser does not know). Same order as [`name_chain`](Self::name_chain) minus its first element, so the two joined with `/` are the backend key.
    pub fn path_below_root(&self, id: NodeId) -> Vec<&str> {
        self.name_chain(id).into_iter().skip(1).collect()
    }

    /// Whether this folder's branch is open in tree mode.
    pub fn is_expanded(&self, id: NodeId) -> bool {
        self.expanded.contains(&id)
    }

    /// A folder click (caret in tree mode): navigate in browse mode, expand or collapse in tree mode, and request the listing when the children are unknown.
    pub fn activate(&mut self, id: NodeId) -> Outcome {
        let Some(node) = self.nodes.get(&id).cloned() else {
            return Outcome::None;
        };
        if node.kind == EntryKind::File {
            return Outcome::FileOpened(id);
        }
        if self.mode == Mode::Tree {
            // Known to be empty: nothing to open, so the row stays as it is.
            if self.holds_nothing_listed(id) {
                self.expanded.remove(&id);
                return Outcome::None;
            }
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

    /// Click on a row's name in browse mode: folders navigate, files report activation. A file ticks silently whenever a [`Selection`] other than [`None`](Selection::None) is possible and the boxes are not merely waiting for a revealing long press — so [`Hidden`](Checkboxes::Hidden) still ticks on tap (the boxes are presentation, not permission), while [`OnLongPress`](Checkboxes::OnLongPress) leaves a press unarmed the row keeps its normal meaning ([`FileOpened`](Outcome::FileOpened)).
    pub fn tap(&mut self, id: NodeId) -> Outcome {
        let file = self
            .nodes
            .get(&id)
            .is_some_and(|node| node.kind == EntryKind::File);
        let ticks = self.selection != Selection::None
            && (self.checkboxes != Checkboxes::OnLongPress || self.armed);
        if file && self.mode == Mode::Browse && ticks {
            // Material-style: without a separate open action a file tap selects it.
            self.toggle(id);
            return Outcome::None;
        }
        self.activate(id)
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
}
