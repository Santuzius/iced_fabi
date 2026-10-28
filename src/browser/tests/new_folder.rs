//! The new folder dialog: open, edit, confirm — the name comes back made free of the siblings — and cancel.

use crate::browser::{Event, NodeId, Outcome};
use crate::{Browser, Entry, EntryKind};

#[test]
fn create_folder_pressed_opens_the_dialog_with_the_default_draft() {
    let mut browser = Browser::sample();
    assert_eq!(browser.update(Event::CreateFolderPressed), Outcome::None);
    assert!(browser.new_folder_dialog_open());
    assert_eq!(browser.new_folder_draft(), Some("New folder"));
}

#[test]
fn confirming_without_a_sibling_asks_for_the_drafted_name_under_the_open_folder() {
    let mut browser = Browser::sample();
    let _ = browser.update(Event::CreateFolderPressed);
    assert_eq!(
        browser.update(Event::NewFolderConfirmed),
        Outcome::CreateFolderRequested {
            parent: NodeId::ROOT,
            name: "New folder".into(),
        }
    );
    assert!(
        !browser.new_folder_dialog_open(),
        "confirming closes the dialog"
    );
}

#[test]
fn a_clash_appends_the_first_free_number() {
    let mut browser = Browser::sample();
    browser.set_children(
        NodeId::ROOT,
        vec![Entry::folder("New folder"), Entry::file("notes.txt")],
    );
    let _ = browser.update(Event::CreateFolderPressed);
    assert_eq!(
        browser.update(Event::NewFolderConfirmed),
        Outcome::CreateFolderRequested {
            parent: NodeId::ROOT,
            name: "New folder (1)".into(),
        }
    );
    // The app listed what it created; the next confirm moves on to (2).
    browser.set_children(
        NodeId::ROOT,
        vec![
            Entry::folder("New folder"),
            Entry::folder("New folder (1)"),
            Entry::file("notes.txt"),
        ],
    );
    let _ = browser.update(Event::CreateFolderPressed);
    assert_eq!(
        browser.update(Event::NewFolderConfirmed),
        Outcome::CreateFolderRequested {
            parent: NodeId::ROOT,
            name: "New folder (2)".into(),
        }
    );
}

#[test]
fn a_file_sibling_clashes_a_folder_name_too() {
    // One namespace per folder, as on any filesystem: a file named "New folder" occupies the name.
    let mut browser = Browser::sample();
    browser.set_children(
        NodeId::ROOT,
        vec![Entry::file("New folder"), Entry::file("notes.txt")],
    );
    let _ = browser.update(Event::CreateFolderPressed);
    assert_eq!(
        browser.update(Event::NewFolderConfirmed),
        Outcome::CreateFolderRequested {
            parent: NodeId::ROOT,
            name: "New folder (1)".into(),
        }
    );
}

#[test]
fn an_edited_name_is_used_and_deduped_like_the_default() {
    let mut browser = Browser::sample(); // sample lists the folder "Finanzen"
    let _ = browser.update(Event::CreateFolderPressed);
    assert_eq!(
        browser.update(Event::NewFolderNameEdited("Finanzen".into())),
        Outcome::None
    );
    assert_eq!(browser.new_folder_draft(), Some("Finanzen"));
    assert_eq!(
        browser.update(Event::NewFolderConfirmed),
        Outcome::CreateFolderRequested {
            parent: NodeId::ROOT,
            name: "Finanzen (1)".into(),
        }
    );
}

#[test]
fn the_confirmed_name_is_trimmed_and_compared_exactly() {
    // Case-sensitive like a listing: "new folder" beside "New folder" is no clash; padding is cut off.
    let mut browser = Browser::sample();
    browser.set_children(NodeId::ROOT, vec![Entry::folder("new folder")]);
    let _ = browser.update(Event::CreateFolderPressed);
    let _ = browser.update(Event::NewFolderNameEdited("  New folder  ".into()));
    assert_eq!(
        browser.update(Event::NewFolderConfirmed),
        Outcome::CreateFolderRequested {
            parent: NodeId::ROOT,
            name: "New folder".into(),
        }
    );
}

#[test]
fn a_blank_draft_keeps_the_dialog_open() {
    let mut browser = Browser::sample();
    let _ = browser.update(Event::CreateFolderPressed);
    let _ = browser.update(Event::NewFolderNameEdited("   ".into()));
    assert_eq!(browser.update(Event::NewFolderConfirmed), Outcome::None);
    assert!(
        browser.new_folder_dialog_open(),
        "the user can still type a name"
    );
}

#[test]
fn cancel_discards_and_reopening_starts_fresh() {
    let mut browser = Browser::sample();
    let _ = browser.update(Event::CreateFolderPressed);
    let _ = browser.update(Event::NewFolderNameEdited("half typed".into()));
    assert_eq!(browser.update(Event::NewFolderCancelled), Outcome::None);
    assert!(!browser.new_folder_dialog_open());
    let _ = browser.update(Event::CreateFolderPressed);
    assert_eq!(browser.new_folder_draft(), Some("New folder"));
}

#[test]
fn dialog_events_while_closed_do_nothing() {
    let mut browser = Browser::sample();
    assert_eq!(
        browser.update(Event::NewFolderNameEdited("x".into())),
        Outcome::None
    );
    assert_eq!(browser.update(Event::NewFolderConfirmed), Outcome::None);
    assert_eq!(browser.update(Event::NewFolderCancelled), Outcome::None);
    assert!(!browser.new_folder_dialog_open());
}

#[test]
fn the_dialog_creates_under_the_open_folder_not_the_root() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let _ = browser.update(Event::RowPressed(folder)); // enter the folder
    browser.set_children(folder, Vec::new());
    let _ = browser.update(Event::CreateFolderPressed); // opens the dialog
    assert_eq!(
        browser.update(Event::NewFolderConfirmed),
        Outcome::CreateFolderRequested {
            parent: folder,
            name: "New folder".into(),
        }
    );
}
