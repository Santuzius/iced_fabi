//! What the cabinet widget reports back to the application. Apps hand each [`Event`] straight to [`Browser::update`]; they never construct or match on them, so the enum is `#[non_exhaustive]` and the variants are named after what the user *did*, not after the state method that happens to apply it.

use crate::browser::node::NodeId;

/// One interaction inside the widget. Route it with [`update`](crate::Browser::update).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
    /// A row was pressed — its name, or anywhere on it while the boxes are shown. Folders navigate (browse) and files are the app's business; which one and whether the press instead ticks is [`update`](crate::Browser::update)'s call.
    RowPressed(NodeId),
    /// The caret of a tree row was pressed: expand or collapse that branch.
    CaretPressed(NodeId),
    /// The "could not load — tap to retry" note was pressed for this node.
    RetryPressed(NodeId),
    /// A row's selection box was pressed.
    MarkerPressed(NodeId),
    /// The header's select-all box was pressed.
    HeaderMarkerPressed,
    /// A breadcrumb was pressed.
    CrumbPressed(NodeId),
    /// A column header was pressed; the index is into [`column_count`](crate::Browser::column_count).
    SortPressed(usize),
    /// The browse/tree switch was pressed.
    ModeSwitchPressed,
    /// A row's press began — only emitted while the boxes are hidden in the long-press mode; see [`hold_started`](crate::Browser::hold_started).
    HoldStarted(NodeId),
    /// A row's press ended — see [`hold_ended`](crate::Browser::hold_ended): held long enough it reveals the boxes and ticks the row, otherwise it is that row's normal press.
    HoldEnded(NodeId),
}

impl Event {
    /// The node this event is about, if any.
    pub fn node(&self) -> Option<NodeId> {
        match self {
            Event::RowPressed(id)
            | Event::CaretPressed(id)
            | Event::RetryPressed(id)
            | Event::MarkerPressed(id)
            | Event::CrumbPressed(id)
            | Event::HoldStarted(id)
            | Event::HoldEnded(id) => Some(*id),
            Event::HeaderMarkerPressed | Event::SortPressed(_) | Event::ModeSwitchPressed => None,
        }
    }
}
