//! Column configuration: the built-in enum, the visible list and hiding.

use crate::browser::Browser;
use crate::{BuiltInColumn, Cell, Column, ModifiedColumn, Node, SizeColumn};

/// A stand-in custom column, like the apps' own.
#[derive(Clone, Copy)]
struct TypeColumn;

impl Column for TypeColumn {
    fn header(&self) -> String {
        "Type".into()
    }

    fn cell(&self, _browser: &Browser, _node: &Node) -> Cell {
        Cell::Empty
    }
}

#[test]
fn every_column_starts_visible() {
    let browser = Browser::new("My files");
    assert!((0..browser.columns().len()).all(|index| !browser.is_column_hidden(index)));
}

#[test]
fn hiding_and_showing_a_column_by_index() {
    let mut browser = Browser::new("My files");
    browser.hide_column(2);
    assert!(browser.is_column_hidden(2));
    browser.show_column(2);
    assert!(!browser.is_column_hidden(2));
}

#[test]
fn the_name_column_and_bad_indices_stay_put() {
    let mut browser = Browser::new("My files");
    browser.hide_column(0);
    browser.hide_column(99);
    assert!(!browser.is_column_hidden(0));
    assert!(!browser.is_column_hidden(99));
    assert!(!browser.is_column_hidden(2)); // hide_column(99) did not shift anything
}

#[test]
fn a_hidden_column_still_sorts() {
    let mut browser = Browser::sample();
    browser.hide_column(2); // Size
    browser.sort_by_column(2);
    assert_eq!(browser.sort(), (2, true));
}

#[test]
fn the_visible_list_names_columns_not_indices() {
    let mut browser = Browser::new("My files");
    browser.add_column(TypeColumn); // index 3
    browser.set_visible_columns([BuiltInColumn::Size.into(), TypeColumn.into()]);
    assert!(browser.is_column_hidden(1)); // Modified is off
    assert!(!browser.is_column_hidden(2)); // Size stays
    assert!(!browser.is_column_hidden(3)); // the custom column stays
    assert!(!browser.is_column_hidden(0)); // the anchor stays
}

#[test]
fn a_renamed_custom_header_still_matches_its_type() {
    let mut browser = Browser::new("My files");
    // The custom column's id is its header, so a configured header is what the list has to name.
    struct LabelledColumn;

    impl Column for LabelledColumn {
        fn header(&self) -> String {
            "Typ".into()
        }

        fn cell(&self, _browser: &Browser, _node: &Node) -> Cell {
            Cell::Empty
        }
    }

    browser.add_column(LabelledColumn);
    browser.set_visible_columns([BuiltInColumn::Size.into(), TypeColumn.into()]); // "Type" matches nothing
    assert!(browser.is_column_hidden(3));
    browser.set_visible_columns([BuiltInColumn::Size.into(), LabelledColumn.into()]);
    assert!(!browser.is_column_hidden(3));
}

#[test]
fn built_ins_keep_their_ids_across_header_renames() {
    let mut browser = Browser::new("My files");
    browser.set_columns([
        Box::new(ModifiedColumn::new().with_header("Geändert")) as Box<dyn Column>,
        Box::new(SizeColumn::new()),
    ]);
    assert_eq!(browser.columns()[1].header(), "Geändert");
    browser.set_visible_columns([BuiltInColumn::Modified.into()]);
    assert!(!browser.is_column_hidden(1)); // named by id, not by the renamed header
    assert!(browser.is_column_hidden(2));
}

#[test]
fn setting_and_adding_columns_resets_or_extends_visibility() {
    let mut browser = Browser::new("My files");
    browser.hide_column(2);
    browser.set_columns([
        Box::new(BuiltInColumn::Modified) as Box<dyn Column>,
        Box::new(BuiltInColumn::Size),
    ]);
    assert_eq!(browser.columns().len(), 3);
    assert!(!browser.is_column_hidden(2)); // the new list starts fully visible
    browser.hide_column(2);
    browser.add_column(TypeColumn);
    assert!(browser.is_column_hidden(2)); // hiding survives an append…
    assert!(!browser.is_column_hidden(3)); // …and the appended column starts visible
}

#[test]
fn set_columns_follows_the_sorted_column_to_its_new_place() {
    let mut browser = Browser::new("My files");
    browser.sort_by_column(2); // Size, ascending
    browser.sort_by_column(2); // descending
    assert_eq!(browser.sort(), (2, false));
    // Same columns in another order: Size sits at index 1 now.
    browser.set_columns([
        Box::new(BuiltInColumn::Size) as Box<dyn Column>,
        Box::new(BuiltInColumn::Modified),
    ]);
    assert_eq!(
        browser.sort(),
        (1, false),
        "still sorted by Size, at its new index"
    );
    // A list without Size at all: back to the Name anchor, ascending.
    browser.set_columns([Box::new(BuiltInColumn::Modified) as Box<dyn Column>]);
    assert_eq!(
        browser.sort(),
        (0, true),
        "a gone sort column falls back to Name"
    );
}

#[test]
fn the_built_in_enum_dispatches_like_the_structs() {
    assert_eq!(
        BuiltInColumn::Name.header(),
        crate::NameColumn::new().header()
    );
    assert_eq!(BuiltInColumn::Modified.id(), ModifiedColumn::new().id());
    assert_eq!(BuiltInColumn::Size.width(), SizeColumn::new().width());
}
