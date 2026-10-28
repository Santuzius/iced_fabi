use crate::browser::Browser;
use crate::browser::{Entry, EntryKind, NodeId};

#[test]
fn pruning_removes_selections_of_gone_entries() {
    let mut browser = Browser::sample();
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    browser.toggle(file);
    browser.set_children(NodeId::ROOT, vec![Entry::folder("Finanzen")]);
    assert!(!browser.has_selection());
    assert!(!browser.nodes.contains_key(&file));
}

#[test]
fn remove_selected_deletes_and_prunes() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let _ = browser.activate(folder);
    browser.set_children(folder, vec![Entry::folder("Krypto")]);
    let krypto = NodeId::derive(folder, EntryKind::Folder, "Krypto");
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    let _ = browser.goto(krypto);
    browser.toggle(folder);
    let _ = browser.goto(NodeId::ROOT);
    browser.toggle(file);
    browser.remove_selected();
    assert!(
        !browser.nodes.contains_key(&folder),
        "the folder node is gone"
    );
    assert!(
        !browser.nodes.contains_key(&krypto),
        "so is its whole subtree"
    );
    assert!(!browser.nodes.contains_key(&file));
    let root = browser.node(NodeId::ROOT).expect("root");
    assert!(root.children.is_empty(), "the listing lost both");
    assert!(root.children_known, "the parent stays a known listing");
    assert!(!browser.expanded.contains(&folder), "expansion pruned");
    assert!(
        !browser.has_selection(),
        "a done tool leaves nothing checked"
    );
}

#[test]
fn duplicate_entries_in_one_listing_count_once() {
    let mut browser = Browser::sample();
    // A backend that lists the same entry twice must not double the row.
    browser.set_children(
        NodeId::ROOT,
        vec![
            Entry::file("a.txt").size(10),
            Entry::file("a.txt").size(999),
            Entry::folder("b"),
        ],
    );
    let root = browser.node(NodeId::ROOT).expect("root");
    assert_eq!(root.children.len(), 2, "the duplicate is dropped");
    let a = NodeId::derive(NodeId::ROOT, EntryKind::File, "a.txt");
    assert_eq!(
        root.children.iter().filter(|id| **id == a).count(),
        1,
        "and its id is not listed twice"
    );
    let node = browser.node(a).expect("a.txt");
    assert_eq!(node.size, Some(10), "the first entry wins");
    let rows = browser.browse_rows();
    assert_eq!(rows.len(), 2, "so the view shows two rows");
}

#[test]
fn remove_drops_nested_ids_once_and_refuses_the_root() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let _ = browser.activate(folder);
    browser.set_children(folder, vec![Entry::folder("Krypto")]);
    let krypto = NodeId::derive(folder, EntryKind::Folder, "Krypto");
    browser.toggle(krypto);
    // Both ids at once: the nested one must not need splicing out on its own.
    browser.remove(&[folder, krypto]);
    assert!(!browser.nodes.contains_key(&folder), "gone from the tree");
    assert!(
        !browser.nodes.contains_key(&krypto),
        "and so is everything below it"
    );
    let root = browser.node(NodeId::ROOT).expect("root");
    assert!(
        !root.children.contains(&folder),
        "the root's listing lost the folder"
    );
    assert!(!browser.expanded.contains(&folder), "expansion pruned");
    assert_eq!(
        browser.current(),
        NodeId::ROOT,
        "the view fell back from the deleted directory"
    );
    assert_eq!(browser.path(), &[NodeId::ROOT], "and so did the breadcrumb");
    assert!(
        !browser.has_selection(),
        "selections in the deleted subtree go with it"
    );
    browser.remove(&[NodeId::ROOT]);
    assert!(
        browser.nodes.contains_key(&NodeId::ROOT),
        "the root scope cannot be deleted"
    );
}

#[test]
fn removing_the_current_folder_moves_the_view_to_its_parent() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let _ = browser.activate(folder);
    browser.set_children(folder, vec![Entry::folder("Krypto")]);
    let krypto = NodeId::derive(folder, EntryKind::Folder, "Krypto");
    let _ = browser.activate(krypto);
    browser.remove(&[krypto]);
    assert_eq!(
        browser.current(),
        folder,
        "the view fell back to the parent, not all the way to the root"
    );
    assert_eq!(
        browser.path(),
        &[NodeId::ROOT, folder],
        "the breadcrumb lost the deleted leg"
    );
    let node = browser.node(folder).expect("Finanzen");
    assert!(
        !node.children.contains(&krypto),
        "the parent's listing lost the entry"
    );
}
