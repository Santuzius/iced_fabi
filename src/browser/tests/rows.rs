use super::column;
use crate::browser::{Browser, Entry, EntryKind, NodeId, rows::Row};
use crate::{Cell, Column, Marker};
use std::cmp::Ordering;

#[test]
fn browse_rows_have_folders_first() {
    let browser = Browser::sample();
    let rows = browser.browse_rows();
    assert_eq!(rows.len(), 2);
    let first = browser.node(rows[0].id).unwrap();
    assert_eq!(first.kind, EntryKind::Folder);
}

#[test]
fn rows_carry_their_selection_markers() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let _ = browser.activate(folder);
    browser.set_children(folder, vec![Entry::file("beleg.pdf")]);
    let file = NodeId::derive(folder, EntryKind::File, "beleg.pdf");
    browser.toggle(file);
    assert_eq!(
        markers(&browser, &browser.browse_rows()),
        vec![("beleg.pdf", Marker::Selected)],
        "inside the folder the ticked file is the only row"
    );
    let _ = browser.goto(NodeId::ROOT);
    assert_eq!(
        markers(&browser, &browser.browse_rows()),
        vec![
            ("Finanzen", Marker::Partial),
            ("notes.txt", Marker::Unselected)
        ],
        "the folder carries the dot for what sits below it"
    );
    browser.set_mode(crate::Mode::Tree);
    assert_eq!(
        markers(&browser, &browser.tree_rows()),
        vec![
            ("My files", Marker::Partial),
            ("Finanzen", Marker::Partial),
            ("beleg.pdf", Marker::Selected),
            ("notes.txt", Marker::Unselected),
        ],
        "the same rules reach the tree rows, which ask once per frame"
    );
    assert_eq!(
        browser.marker(NodeId::ROOT),
        Marker::Partial,
        "and the single-query answer still agrees"
    );
}

/// Each row's name with the marker it carries.
fn markers<'a>(browser: &'a Browser, rows: &[Row]) -> Vec<(&'a str, Marker)> {
    rows.iter()
        .map(|row| {
            (
                browser.node(row.id).expect("listed node").name.as_str(),
                row.marker,
            )
        })
        .collect()
}

#[test]
fn folders_only_hides_files_in_both_modes() {
    let mut browser = Browser::sample();
    browser.set_mode(crate::Mode::Tree);
    browser.set_folders_only(true);
    let names = |rows: &[Row]| -> Vec<String> {
        rows.iter()
            .map(|r| browser.node(r.id).unwrap().name.clone())
            .collect()
    };
    assert_eq!(names(&browser.browse_rows()), vec!["Finanzen"]);
    assert_eq!(names(&browser.tree_rows()), vec!["My files", "Finanzen"]);
    browser.set_folders_only(false);
    assert_eq!(browser.browse_rows().len(), 2, "the file comes back");
}

#[test]
fn tree_rows_carry_depth_and_display_order() {
    let mut browser = Browser::sample();
    browser.set_mode(crate::Mode::Tree);
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let _ = browser.activate(folder); // in tree mode this opens the branch and asks for its listing
    browser.set_children(
        folder,
        vec![Entry::file("beleg.pdf"), Entry::folder("Krypto")],
    );
    let shape: Vec<(String, usize, bool, bool)> = browser
        .tree_rows()
        .iter()
        .map(|row| {
            (
                browser.node(row.id).expect("listed node").name.clone(),
                row.depth,
                row.expandable,
                row.expanded,
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            ("My files".to_owned(), 0, true, true),
            ("Finanzen".to_owned(), 1, true, true),
            ("Krypto".to_owned(), 2, true, false),
            ("beleg.pdf".to_owned(), 2, false, false),
            ("notes.txt".to_owned(), 1, false, false),
        ],
        "folders first, name asc, every level one deeper, an unopened folder keeps its caret"
    );
}

#[test]
fn collapsing_a_branch_hides_its_child_rows() {
    let mut browser = Browser::sample();
    browser.set_mode(crate::Mode::Tree);
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let _ = browser.activate(folder);
    browser.set_children(folder, vec![Entry::file("beleg.pdf")]);
    let names = |browser: &Browser| -> Vec<String> {
        browser
            .tree_rows()
            .iter()
            .map(|row| browser.node(row.id).expect("listed node").name.clone())
            .collect()
    };
    assert_eq!(
        names(&browser),
        vec!["My files", "Finanzen", "beleg.pdf", "notes.txt"],
        "open branch: the child sits under it"
    );
    let _ = browser.activate(folder); // the caret again: closed
    assert!(!browser.is_expanded(folder));
    assert_eq!(
        names(&browser),
        vec!["My files", "Finanzen", "notes.txt"],
        "the child row is gone, the branch row stays"
    );
}

#[test]
fn file_extension_filter_keeps_folders_and_matching_files() {
    let mut browser = Browser::sample();
    browser.set_file_extensions(["txt"]);
    let names: Vec<String> = browser
        .browse_rows()
        .iter()
        .map(|r| browser.node(r.id).unwrap().name.clone())
        .collect();
    assert_eq!(names, vec!["Finanzen", "notes.txt"]);
    browser.set_file_extensions(["pdf"]);
    assert_eq!(
        browser.browse_rows().len(),
        1,
        "no pdf here, only the folder"
    );
    // Select-all inside the scope only reaches listed children.
    browser.select_all();
    let hidden = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    assert!(
        !browser.selected().any(|id| id == hidden),
        "an unlisted file is never selected by select-all"
    );
    browser.show_all_files();
    assert_eq!(
        browser.browse_rows().len(),
        2,
        "no filter: every file again"
    );
}

#[test]
fn custom_column_sorts() {
    let mut browser = Browser::sample();
    let extension = browser.add_column(ExtensionColumn);
    browser.set_children(
        NodeId::ROOT,
        vec![
            Entry::file("a.pdf"),
            Entry::file("b.txt"),
            Entry::file("c.pdf"),
        ],
    );
    browser.sort_by_column(extension);
    let names: Vec<String> = browser
        .browse_rows()
        .iter()
        .map(|row| browser.node(row.id).unwrap().name.clone())
        .collect();
    assert_eq!(names, vec!["a.pdf", "c.pdf", "b.txt"]);
}

/// Test column: sorts files by their extension.
#[derive(Debug)]
struct ExtensionColumn;

impl Column for ExtensionColumn {
    fn header(&self) -> String {
        "Ext".into()
    }

    fn cell(&self, _browser: &Browser, node: &crate::Node) -> Cell {
        Cell::Text {
            text: node.name.rsplit('.').next().unwrap_or("").to_uppercase(),
            muted: true,
        }
    }

    fn compare(&self, a: &crate::Node, b: &crate::Node) -> Ordering {
        a.name
            .rsplit('.')
            .next()
            .unwrap_or("")
            .cmp(b.name.rsplit('.').next().unwrap_or(""))
    }
}

#[test]
fn set_columns_replaces_everything_but_the_name_anchor() {
    let mut browser = Browser::sample();
    browser.set_columns([
        Box::new(
            crate::SizeColumn::new()
                .with_header("Bytes")
                .format(|b| format!("{b} B")),
        ) as Box<dyn Column>,
        Box::new(ExtensionColumn),
    ]);
    let notes = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    let node = browser.node(notes).expect("notes");
    assert_eq!(browser.column_count(), 3, "Name stayed at index 0");
    assert_eq!(column(&browser, 1).header(), "Bytes");
    assert_eq!(
        column(&browser, 1).cell(&browser, node).to_text(),
        Some("900 B"),
        "the closure formatter renders"
    );
    assert_eq!(column(&browser, 2).header(), "Ext");
    browser.sort_by_column(1);
    assert_eq!(browser.sort(), (1, true));
    browser.set_columns([]);
    assert_eq!(browser.column_count(), 1, "the defaults can be reset away");
    assert_eq!(
        browser.sort(),
        (0, true),
        "a sort column past the end fell back to Name"
    );
}

/// The caret of a tree row by name.
fn caret(browser: &Browser, name: &str) -> bool {
    browser.tree_rows().iter().find(|row| browser.node(row.id).is_some_and(|n| n.name == name)).expect("row is shown").expandable
}

#[test]
fn a_folder_marked_empty_has_no_caret_and_is_never_loaded() {
    let mut browser = Browser::new("root");
    browser.set_mode(crate::Mode::Tree);
    browser.set_children(NodeId::ROOT, vec![Entry::folder("Empty").empty(), Entry::folder("Unknown")]);
    assert!(!caret(&browser, "Empty"));
    assert!(caret(&browser, "Unknown"), "a folder not loaded yet keeps its caret");
    let empty = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Empty");
    assert_eq!(browser.activate(empty), crate::Outcome::None);
}

#[test]
fn a_folder_loses_its_caret_once_its_listing_comes_back_empty() {
    let mut browser = Browser::new("root");
    browser.set_mode(crate::Mode::Tree);
    browser.set_folders_only(true);
    browser.set_children(NodeId::ROOT, vec![Entry::folder("Photos")]);
    let photos = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Photos");
    // Only files inside: a folders-only browser has nothing to show below it.
    browser.set_children(photos, vec![Entry::file("a.jpg")]);
    assert!(!caret(&browser, "Photos"));
}

#[test]
fn a_parent_listed_again_without_the_mark_lets_the_folder_load() {
    let mut browser = Browser::new("root");
    browser.set_children(NodeId::ROOT, vec![Entry::folder("Grows").empty()]);
    browser.set_children(NodeId::ROOT, vec![Entry::folder("Grows")]);
    let grows = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Grows");
    assert_eq!(browser.activate(grows), crate::Outcome::NeedsLoad(grows));
}

#[test]
fn tapping_an_empty_folder_in_the_tree_changes_nothing() {
    let mut browser = Browser::new("root");
    browser.set_mode(crate::Mode::Tree);
    browser.set_children(NodeId::ROOT, vec![Entry::folder("Empty").empty(), Entry::folder("Later")]);
    let empty = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Empty");
    let _ = browser.activate(empty);
    assert!(!browser.is_expanded(empty));

    // A folder found empty only after it was opened shows closed again.
    let later = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Later");
    assert_eq!(browser.activate(later), crate::Outcome::NeedsLoad(later));
    browser.set_children(later, Vec::new());
    let row = browser.tree_rows().into_iter().find(|row| row.id == later).unwrap();
    assert!(!row.expanded && !row.expandable);
}
