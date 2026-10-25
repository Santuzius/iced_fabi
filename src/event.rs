//! What the cabinet widget reports back to the application; map these to your own message type.

use crate::browser::node::NodeId;

/// An interaction the widget produced; map it to your message and apply it to the [`Browser`](crate::Browser).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Row clicked (browse) or caret clicked (tree): folders navigate or expand, files report [`FileActivated`](crate::Outcome::FileActivated).
    Activate(NodeId),
    /// Selection marker clicked; see [`toggle`](crate::Browser::toggle).
    Toggle(NodeId),
    /// A row's press began — only emitted in [`LongPress`](crate::SelectionUi::LongPress) while the boxes are hidden; feed it to [`hold_started`](crate::Browser::hold_started).
    HoldStarted(NodeId),
    /// A row's press ended — feed it to [`hold_ended`](crate::Browser::hold_ended), which reveals the boxes and ticks the held row when the row was held long enough.
    HoldEnded(NodeId),
    /// The header's select-all box clicked; see [`toggle_all`](crate::Browser::toggle_all).
    ToggleAll,
    /// Breadcrumb clicked.
    GoTo(NodeId),
    /// Sort header clicked; index into [`columns`](crate::Browser::columns). See [`sort_by_column`](crate::Browser::sort_by_column).
    Sort(usize),
    /// List/tree switch clicked.
    ToggleMode,
}
