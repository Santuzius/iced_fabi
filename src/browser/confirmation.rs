//! The confirmation modal any [`Tool`](crate::Tool) can put in front of its click. The browser only remembers *which* tool is pending — the card's words come fresh from [`Tool::confirmation`](crate::Tool::confirmation) every frame — and confirming hands that tool back to the app as [`ToolConfirmed`](crate::Outcome::ToolConfirmed) so the app runs the action it never received.

use super::{Browser, Outcome};
use crate::ToolKey;

impl Browser {
    /// Open the modal for the tool `key` names, closing the new folder dialog: one modal at a time. The app may call this for its own destructive affordance; a tool that [asks itself](crate::Tool::confirmation) needs no call — its button press arrives as [`Event::ToolConfirmationRequested`](crate::Event::ToolConfirmationRequested).
    pub fn open_tool_confirmation(&mut self, key: ToolKey) {
        self.new_folder = None;
        self.pending_confirmation = Some(key);
    }

    /// The tool the open modal belongs to, by its switch id; `None` while no confirmation is pending.
    pub fn pending_tool_confirmation(&self) -> Option<ToolKey> {
        self.pending_confirmation.clone()
    }

    /// Dismiss the modal: nothing runs.
    pub fn close_tool_confirmation(&mut self) {
        self.pending_confirmation = None;
    }

    /// The modal's confirm button: close and hand the tool to the app as [`ToolConfirmed`](Outcome::ToolConfirmed); while no modal is open this answers [`Outcome::None`].
    pub(crate) fn confirm_tool_confirmation(&mut self) -> Outcome {
        match self.pending_confirmation.take() {
            Some(tool) => Outcome::ToolConfirmed { tool },
            None => Outcome::None,
        }
    }
}
