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
    assert_eq!(browser.columns().len(), 3, "Name stayed at index 0");
    assert_eq!(browser.columns()[1].header(), "Bytes");
    assert_eq!(
        browser.columns()[1].cell(&browser, node).to_text(),
        Some("900 B"),
        "the closure formatter renders"
    );
    assert_eq!(browser.columns()[2].header(), "Ext");
    browser.sort_by_column(1);
    assert_eq!(browser.sort(), (1, true));
    browser.set_columns([]);
    assert_eq!(browser.columns().len(), 1, "the defaults can be reset away");
    assert_eq!(
        browser.sort(),
        (0, true),
        "a sort column past the end fell back to Name"
    );
}
