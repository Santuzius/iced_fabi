use crate::browser::Browser;
use crate::browser::{Entry, EntryKind, NodeId, Outcome};

#[test]
fn activate_walks_path_and_loads_once() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    assert_eq!(browser.activate(folder), Outcome::NeedsLoad(folder));
    assert_eq!(browser.current(), folder, "browse mode navigates");
    assert_eq!(
        browser.activate(folder),
        Outcome::None,
        "still loading, no second request"
    );
    assert_eq!(browser.path(), &[NodeId::ROOT, folder]);
    browser.set_children(
        folder,
        vec![Entry {
            name: "Krypto".into(),
            kind: EntryKind::Folder,
            size: None,
            modified: None,
        }],
    );
    let krypto = NodeId::derive(folder, EntryKind::Folder, "Krypto");
    assert_eq!(browser.activate(krypto), Outcome::NeedsLoad(krypto));
}

#[test]
fn a_scope_under_a_selected_ancestor_shares_its_selection() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    browser.set_children(
        folder,
        vec![Entry {
            name: "beleg.pdf".into(),
            kind: EntryKind::File,
            size: None,
            modified: None,
        }],
    );
    let _ = browser.activate(folder); // browse into it, then check an ancestor
    browser.toggle(NodeId::ROOT);
    assert_eq!(
        browser.scope_marker(),
        crate::Marker::Selected,
        "inside a checked folder the select-all box is ticked"
    );
    browser.select_all();
    assert_eq!(
        browser.selected().collect::<Vec<_>>(),
        vec![NodeId::ROOT],
        "select-all adds nothing under a selected ancestor"
    );
    browser.toggle_all();
    assert!(
        !browser.has_selection(),
        "the box unticks the covering ancestor"
    );
}

#[test]
fn tree_rows_follow_expansion() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    browser.set_mode(crate::Mode::Tree);
    let _ = browser.activate(folder);
    let folder_node = browser.node(folder).unwrap();
    assert!(
        folder_node.children.is_empty(),
        "not loaded yet: caret but no children"
    );
    let ids: Vec<_> = browser.tree_rows().iter().map(|row| row.id).collect();
    // Root expanded: its folder (caret, still unloaded) and its file.
    let notes = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    assert_eq!(ids, vec![NodeId::ROOT, folder, notes]);
}
