use crate::browser::Browser;
use crate::browser::{EntryKind, NodeId, long_press::LONG_PRESS};

#[test]
fn long_press_reveals_boxes_and_short_presses_do_not() {
    let mut browser = Browser::sample();
    browser.set_selection_ui(crate::SelectionUi::LongPress);
    let notes = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    assert!(!browser.boxes_visible(), "unarmed: no boxes");
    browser.hold_started(notes);
    assert!(
        !browser.hold_ended_after(Some((notes, std::time::Duration::from_millis(100)))),
        "a short press stays unarmed"
    );
    assert!(!browser.boxes_visible());
    browser.hold_started(notes);
    assert!(
        browser.hold_ended_after(Some((notes, LONG_PRESS))),
        "a hold of exactly LONG_PRESS arms"
    );
    assert!(browser.boxes_visible());
    assert!(
        browser.selected().any(|id| id == notes),
        "the held row comes ticked with the boxes"
    );
    // With the boxes revealed, ticking works as usual until the app disarms again.
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    browser.toggle(folder);
    assert!(browser.selected().any(|id| id == folder));
    browser.disarm();
    assert!(!browser.boxes_visible());
    // Releasing a different row than the one pressed arms nothing.
    browser.hold_started(notes);
    assert!(!browser.hold_ended(folder));
}
