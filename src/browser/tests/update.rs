//! [`Browser::update`] routing: one test per [`Event`] variant, asserting the widget event lands on the right state operation and reports the right [`Outcome`].

use crate::browser::{Checkboxes, EntryKind, Event, NodeId, Outcome, Selection};
use crate::{Browser, Marker, Mode};

fn into_folder(browser: &mut Browser) -> NodeId {
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let _ = browser.update(Event::RowPressed(folder)); // navigate in, triggers NeedsLoad
    browser.set_children(folder, Vec::new());
    folder
}

#[test]
fn open_asks_for_the_root_listing_once() {
    let mut browser = Browser::new("My files");
    assert_eq!(browser.open(), Outcome::NeedsLoad(NodeId::ROOT));
    assert_eq!(
        browser.open(),
        Outcome::None,
        "already loading: no second request"
    );
}

#[test]
fn row_pressed_on_a_folder_navigates_and_asks_for_the_listing() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    assert_eq!(
        browser.update(Event::RowPressed(folder)),
        Outcome::NeedsLoad(folder)
    );
    assert_eq!(browser.current(), folder, "browse mode entered the folder");
}

#[test]
fn row_pressed_on_a_file_ticks_while_the_boxes_are_shown() {
    let mut browser = Browser::sample(); // default Select: boxes shown
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    assert_eq!(
        browser.update(Event::RowPressed(file)),
        Outcome::None,
        "tap selected it"
    );
    assert_eq!(browser.marker(file), Marker::Selected);
}

#[test]
fn row_pressed_on_a_file_ticks_silently_when_the_boxes_are_hidden() {
    let mut browser = Browser::sample();
    browser.set_checkboxes(Checkboxes::Hidden);
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    assert_eq!(
        browser.update(Event::RowPressed(file)),
        Outcome::None,
        "Hidden is presentation: the selection machinery still runs, just without boxes"
    );
    assert_eq!(browser.marker(file), Marker::Selected);
    assert!(!browser.boxes_visible(), "and stays invisible");
}

#[test]
fn row_pressed_on_a_file_reports_with_no_selection_possible() {
    let mut browser = Browser::sample();
    browser.set_selection(Selection::None);
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    assert!(
        !browser.boxes_visible(),
        "Selection::None hides the boxes whatever Checkboxes says"
    );
    assert_eq!(
        browser.update(Event::RowPressed(file)),
        Outcome::FileOpened(file)
    );
}

#[test]
fn caret_pressed_expands_a_tree_branch() {
    let mut browser = Browser::sample();
    browser.set_mode(Mode::Tree);
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    assert_eq!(
        browser.update(Event::CaretPressed(folder)),
        Outcome::NeedsLoad(folder),
        "expanded and loading"
    );
    assert!(browser.is_expanded(folder));
}

#[test]
fn marker_pressed_toggles_the_unit_selection() {
    let mut browser = Browser::sample();
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    assert_eq!(
        browser.update(Event::MarkerPressed(file)),
        Outcome::SelectionChanged
    );
    assert_eq!(browser.marker(file), Marker::Selected);
    assert_eq!(
        browser.update(Event::MarkerPressed(file)),
        Outcome::SelectionChanged
    );
    assert!(!browser.has_selection(), "the second press unticked it");
}

#[test]
fn header_marker_pressed_selects_the_whole_scope() {
    let mut browser = Browser::sample();
    assert_eq!(
        browser.update(Event::HeaderMarkerPressed),
        Outcome::SelectionChanged
    );
    assert_eq!(
        browser.scope_marker(),
        Marker::Selected,
        "every child of the browse scope is checked"
    );
}

#[test]
fn crumb_pressed_jumps_back_up_the_path() {
    let mut browser = Browser::sample();
    let folder = into_folder(&mut browser);
    assert_eq!(browser.current(), folder);
    assert_eq!(
        browser.update(Event::CrumbPressed(NodeId::ROOT)),
        Outcome::None,
        "root is already loaded"
    );
    assert_eq!(
        browser.current(),
        NodeId::ROOT,
        "the breadcrumb stepped out"
    );
}

#[test]
fn retry_pressed_asks_for_a_failed_listing_again() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    assert_eq!(
        browser.update(Event::RowPressed(folder)),
        Outcome::NeedsLoad(folder)
    );
    browser.load_failed(folder);
    assert_eq!(
        browser.update(Event::RetryPressed(folder)),
        Outcome::NeedsLoad(folder),
        "retry re-arms the load"
    );
}

#[test]
fn sort_pressed_flips_the_direction() {
    let mut browser = Browser::sample();
    assert_eq!(browser.sort(), (0, true));
    assert_eq!(browser.update(Event::SortPressed(0)), Outcome::None);
    assert_eq!(browser.sort(), (0, false), "same column reverses");
}

#[test]
fn mode_switch_pressed_flips_browse_and_tree() {
    let mut browser = Browser::sample();
    assert_eq!(browser.mode(), Mode::Browse);
    let _ = browser.update(Event::ModeSwitchPressed);
    assert_eq!(browser.mode(), Mode::Tree);
    let _ = browser.update(Event::ModeSwitchPressed);
    assert_eq!(browser.mode(), Mode::Browse);
}

#[test]
fn short_press_under_long_press_reaches_the_state_only_through_hold_ended() {
    let mut browser = Browser::sample();
    browser.set_checkboxes(Checkboxes::OnLongPress);
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    let _ = browser.update(Event::HoldStarted(file));
    let outcome = browser.update(Event::HoldEnded(file));
    assert_eq!(
        outcome,
        Outcome::FileOpened(file),
        "the widget's press layer hid the row's own events: update() applies the short press as its tap"
    );
    assert!(!browser.is_armed(), "a short press arms nothing");
    assert!(
        !browser.has_selection(),
        "and while unarmed a file tap never ticks"
    );
    assert!(
        !browser.has_selection(),
        "the armed selection only comes from a real hold"
    );
}
