use crate::browser::{Browser, Entry, EntryKind, NodeId, rows::Row};
use crate::{Cell, Column};
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
