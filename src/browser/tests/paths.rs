use crate::Marker;
use crate::browser::{Browser, Entry, EntryKind, Mode, NodeId, Pick, Selection};

fn folder(parent: NodeId, name: &str) -> NodeId {
    NodeId::derive(parent, EntryKind::Folder, name)
}

/// A root with `Videos/` (listed: `Raw/`, `Cut/`, `clip.mp4`) and `notes.txt`.
fn videos() -> Browser {
    let mut browser = Browser::sample();
    browser.set_children(
        NodeId::ROOT,
        vec![Entry::folder("Videos"), Entry::file("notes.txt")],
    );
    browser
}

fn list_videos(browser: &mut Browser) {
    browser.set_children(
        folder(NodeId::ROOT, "Videos"),
        vec![
            Entry::folder("Raw"),
            Entry::folder("Cut"),
            Entry::file("clip.mp4"),
        ],
    );
}

#[test]
fn a_listed_path_selects_its_node() {
    let mut browser = videos();
    list_videos(&mut browser);
    browser.select_path("Videos/Raw");
    let raw = folder(folder(NodeId::ROOT, "Videos"), "Raw");
    assert_eq!(browser.selected().collect::<Vec<_>>(), vec![raw]);
    assert_eq!(browser.selected_paths(), vec!["Videos/Raw"]);
}

#[test]
fn an_unlisted_path_waits_and_resolves_with_its_listing() {
    let mut browser = videos();
    browser.select_path("/Videos/Raw/");
    let videos = folder(NodeId::ROOT, "Videos");
    assert!(browser.has_selection(), "a pending path counts as selected");
    assert_eq!(browser.selection_count(), 1);
    assert_eq!(
        browser.selected().count(),
        0,
        "nothing listed is ticked yet"
    );
    assert_eq!(browser.selected_paths(), vec!["Videos/Raw"]);
    assert_eq!(
        browser.marker(videos),
        Marker::Partial,
        "the listed ancestor shows the dot"
    );
    assert_eq!(browser.scope_marker(), Marker::Partial);
    list_videos(&mut browser);
    let raw = folder(videos, "Raw");
    assert_eq!(browser.selected().collect::<Vec<_>>(), vec![raw]);
    assert_eq!(browser.marker(raw), Marker::Selected);
    assert_eq!(browser.selected_paths(), vec!["Videos/Raw"]);
}

#[test]
fn a_path_the_listing_lacks_stays_pending() {
    let mut browser = videos();
    browser.select_path("Videos/Gone");
    list_videos(&mut browser);
    assert_eq!(
        browser.selected_paths(),
        vec!["Videos/Gone"],
        "the app's saved state is only dropped by the user"
    );
}

#[test]
fn a_covered_path_changes_nothing_and_selecting_a_folder_covers_pending_paths() {
    let mut browser = videos();
    let videos_id = folder(NodeId::ROOT, "Videos");
    browser.select_path("Videos/Raw");
    browser.toggle(videos_id);
    assert_eq!(
        browser.selected_paths(),
        vec!["Videos"],
        "the folder stands for the pending path below it"
    );
    browser.select_path("Videos/Cut");
    assert_eq!(browser.selected_paths(), vec!["Videos"]);
}

#[test]
fn the_header_box_clears_pending_paths_in_its_scope() {
    let mut browser = videos();
    browser.select_path("Videos/Raw");
    assert_eq!(browser.scope_marker(), Marker::Partial);
    browser.toggle_all();
    assert!(!browser.has_selection());
}

#[test]
fn select_all_covers_pending_paths_below_the_children() {
    let mut browser = videos();
    browser.select_path("Videos/Raw");
    browser.select_all();
    assert_eq!(browser.selection_count(), 2, "Videos and notes.txt");
    assert!(!browser.selected_paths().contains(&"Videos/Raw".to_owned()));
}

#[test]
fn single_selection_keeps_one_path() {
    let mut browser = videos();
    browser.set_selection(Selection::Single);
    browser.select_paths(["notes.txt", "Videos/Raw"]);
    assert_eq!(browser.selected_paths(), vec!["Videos/Raw"]);
    browser.toggle(NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt"));
    assert_eq!(
        browser.selected_paths(),
        vec!["notes.txt"],
        "a tick replaces the pending path"
    );
}

#[test]
fn the_empty_path_is_the_root() {
    let mut browser = videos();
    browser.select_path("");
    assert_eq!(browser.selected().collect::<Vec<_>>(), vec![NodeId::ROOT]);
    assert_eq!(browser.selected_paths(), vec![""]);
}

#[test]
fn a_folder_wins_over_a_file_of_the_same_name() {
    let mut browser = Browser::new("r");
    browser.set_children(NodeId::ROOT, vec![Entry::file("x"), Entry::folder("x")]);
    browser.select_path("x");
    assert_eq!(
        browser.selected().collect::<Vec<_>>(),
        vec![folder(NodeId::ROOT, "x")]
    );
}

#[test]
fn removing_a_folder_drops_pending_paths_below_it() {
    let mut browser = videos();
    browser.select_path("Videos/Raw");
    browser.remove(&[folder(NodeId::ROOT, "Videos")]);
    assert!(!browser.has_selection());
}

#[test]
fn the_folder_picker_picks_the_open_folder_until_one_is_ticked() {
    let mut browser = Browser::picker("Drive", Pick::Folder);
    let _ = browser.open();
    browser.set_children(
        NodeId::ROOT,
        vec![Entry::folder("Videos"), Entry::file("a.txt")],
    );
    assert!(browser.has_pick());
    assert_eq!(browser.picked(), vec![""], "the root, i.e. the whole drive");
    let videos_id = folder(NodeId::ROOT, "Videos");
    let _ = browser.activate(videos_id);
    assert_eq!(browser.picked(), vec!["Videos"]);
    let _ = browser.goto(NodeId::ROOT);
    browser.toggle(videos_id);
    assert_eq!(
        browser.picked(),
        vec!["Videos"],
        "the tick wins over the open folder"
    );
    browser.toggle(videos_id);
    browser.set_mode(Mode::Tree);
    assert!(
        !browser.has_pick(),
        "tree mode has no open folder to fall back to"
    );
}

#[test]
fn a_multi_picker_picks_nothing_without_a_selection() {
    let browser = Browser::picker("Drive", Pick::Folders);
    assert!(!browser.has_pick());
    assert!(browser.picked().is_empty());
}
