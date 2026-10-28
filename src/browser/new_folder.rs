//! The new folder dialog the built-in [`CreateFolder`](crate::BuiltInTool::CreateFolder) button opens. The browser owns the drafted name; confirming hands the app a name made free of the open folder's siblings through [`CreateFolderRequested`](crate::Outcome::CreateFolderRequested), so the app only has to create.

use super::{Browser, NodeId, Outcome};

/// The name drafted when the dialog opens; a sibling of that name gets ` (1)`, ` (2)`, … appended.
pub(super) const NEW_FOLDER: &str = "New folder";

impl Browser {
    /// Open the dialog with `New folder` drafted, closing a pending tool confirmation: one modal at a time. The app may call this for its own create-folder affordance; the built-in button goes through [`Event::CreateFolderPressed`](crate::Event::CreateFolderPressed).
    pub fn open_new_folder_dialog(&mut self) {
        self.pending_confirmation = None;
        self.new_folder = Some(NEW_FOLDER.into());
    }

    /// Whether the dialog is open; while it is, the app should leave the keyboard to the dialog (a global Enter must not open a row behind it).
    pub fn new_folder_dialog_open(&self) -> bool {
        self.new_folder.is_some()
    }

    /// Store a name typed into the dialog's field; while the dialog is closed this does nothing.
    pub fn edit_new_folder_name(&mut self, name: impl Into<String>) {
        if let Some(draft) = self.new_folder.as_mut() {
            *draft = name.into();
        }
    }

    /// Dismiss the dialog, discarding the draft.
    pub fn close_new_folder_dialog(&mut self) {
        self.new_folder = None;
    }

    /// The dialog's confirm (its button, or Enter in the field): the trimmed draft, made free of the current folder's siblings, goes to the app as [`CreateFolderRequested`](Outcome::CreateFolderRequested) and the dialog closes. A blank draft answers [`Outcome::None`] and leaves the dialog open, so the user can still type a name.
    pub fn confirm_new_folder(&mut self) -> Outcome {
        let Some(draft) = self.new_folder.take() else {
            return Outcome::None;
        };
        let wanted = draft.trim();
        if wanted.is_empty() {
            self.new_folder = Some(draft);
            return Outcome::None;
        }
        let name = self.free_name(self.current, wanted);
        Outcome::CreateFolderRequested {
            parent: self.current,
            name,
        }
    }

    /// The draft the cabinet paints into the dialog's field; `None` while the dialog is closed.
    pub(crate) fn new_folder_draft(&self) -> Option<&str> {
        self.new_folder.as_deref()
    }

    /// `wanted` when no sibling of `parent` already has it, otherwise `wanted (1)`, `wanted (2)`, … up to the first free name. Siblings means every listed child, files included — one namespace, as on any filesystem; names are compared exactly, as listings usually arrive them. A parent whose children are not listed yet has no known clash; the app's create call reports the real error then.
    fn free_name(&self, parent: NodeId, wanted: &str) -> String {
        let taken = |name: &str| {
            self.nodes.get(&parent).is_some_and(|node| {
                node.children
                    .iter()
                    .any(|id| self.nodes.get(id).is_some_and(|child| child.name == name))
            })
        };
        if !taken(wanted) {
            return wanted.to_owned();
        }
        // `parent` has finitely many children, so among the first `count + 1` suffixed names one is free.
        let siblings = self
            .nodes
            .get(&parent)
            .map_or(0, |node| node.children.len());
        (1..=siblings + 1)
            .map(|n| format!("{wanted} ({n})"))
            .find(|candidate| !taken(candidate))
            .expect("a free name among count + 1 suffixes")
    }
}
