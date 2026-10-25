//! Long press in the [`OnLongPress`](crate::Checkboxes::OnLongPress) mode: the hold that arms the selection and reveals the boxes.

use super::{Browser, NodeId};

/// How long a row must be held in [`OnLongPress`](crate::Checkboxes::OnLongPress) to reveal the boxes.
pub const LONG_PRESS: std::time::Duration = std::time::Duration::from_millis(500);

impl Browser {
    /// Whether a revealing long press has armed the boxes.
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Hide the boxes revealed by a long press again (e.g. from a back or cancel action).
    pub fn disarm(&mut self) {
        self.armed = false;
        self.held = None;
    }

    /// A row's press began ([`OnLongPress`](crate::Checkboxes::OnLongPress) wiring): remember which row and when.
    pub fn hold_started(&mut self, id: NodeId) {
        self.held = Some((id, std::time::Instant::now()));
    }

    /// A row's press ended: true if the same row was held at least [`LONG_PRESS`] — the boxes appear and the held row itself becomes the selection. On false the press was short: the app should now apply the row's normal [`tap`](Browser::tap) — while the boxes are hidden the widget's press layer replaces the row button's own events, so the tap only arrives through this path.
    pub fn hold_ended(&mut self, id: NodeId) -> bool {
        let held = self
            .held
            .take()
            .filter(|(held_id, _)| *held_id == id)
            .map(|(held_id, at)| (held_id, at.elapsed()));
        self.hold_ended_after(held)
    }

    /// The clock-free core of [`hold_ended`](Self::hold_ended): arm when the hold lasted long enough and tick the held row with it.
    pub(super) fn hold_ended_after(&mut self, held: Option<(NodeId, std::time::Duration)>) -> bool {
        self.held = None;
        let armed = held.is_some_and(|(_, held)| held >= LONG_PRESS);
        if armed {
            self.armed = true;
            if let Some((id, _)) = held
                && self.marker(id) != super::Marker::Selected
            {
                // Material-style: the row you held long enough to reveal the boxes is already chosen.
                self.toggle(id);
            }
        }
        armed
    }
}
