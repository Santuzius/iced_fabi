use crate::Marker;
use crate::browser::{Browser, Entry, EntryKind, NodeId, Outcome};

#[test]
fn partial_marker_propagates_up() {
    let mut browser = Browser::sample();
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    browser.toggle(file);
    assert_eq!(browser.marker(file), Marker::Selected);
    assert_eq!(browser.marker(NodeId::ROOT), Marker::Partial);
}

#[test]
fn selected_folder_ticks_its_loaded_children() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    browser.set_children(folder, vec![Entry::file("beleg.pdf")]);
    let child = NodeId::derive(folder, EntryKind::File, "beleg.pdf");
    browser.toggle(folder);
    assert_eq!(
        browser.marker(child),
        Marker::Selected,
        "the folder's tick shows on the child row too"
    );
    assert_eq!(
        browser.selected().collect::<Vec<_>>(),
        vec![folder],
        "the selection set stays normalized: only the folder is explicit"
    );
    browser.toggle(child);
    assert!(
        !browser.has_selection(),
        "tapping the only covered row leaves nothing ticked"
    );
    assert_eq!(browser.marker(child), Marker::Unselected);
    assert_eq!(
        browser.marker(folder),
        Marker::Unselected,
        "no dot left without selected children"
    );
}

#[test]
fn unchecking_a_covered_row_keeps_the_rest_of_the_unit() {
    let mut browser = Browser::sample();
    browser.toggle(NodeId::ROOT);
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    browser.toggle(file);
    assert_eq!(
        browser.marker(file),
        Marker::Unselected,
        "the tapped box unticks itself"
    );
    assert_eq!(
        browser.marker(folder),
        Marker::Selected,
        "the sibling stays ticked"
    );
    assert_eq!(
        browser.marker(NodeId::ROOT),
        Marker::Partial,
        "the ancestor becomes a dot"
    );
    assert_eq!(browser.selected().collect::<Vec<_>>(), vec![folder]);
    browser.toggle(NodeId::ROOT);
    assert_eq!(
        browser.selected().collect::<Vec<_>>(),
        vec![NodeId::ROOT],
        "rechecking the ancestor collapses the dissolved unit back into one"
    );
    assert_eq!(
        browser.marker(file),
        Marker::Selected,
        "covered by the root again"
    );
}

#[test]
fn unchecking_deep_below_a_selected_folder_dissolves_along_the_chain() {
    let mut browser = Browser::sample();
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    browser.set_children(
        folder,
        vec![Entry::folder("Depot"), Entry::file("beleg.pdf")],
    );
    let depot = NodeId::derive(folder, EntryKind::Folder, "Depot");
    browser.set_children(depot, vec![Entry::file("keys.txt"), Entry::file("pub.txt")]);
    let keys = NodeId::derive(depot, EntryKind::File, "keys.txt");
    let public = NodeId::derive(depot, EntryKind::File, "pub.txt");
    browser.toggle(folder);
    assert_eq!(
        browser.marker(keys),
        Marker::Selected,
        "covered from two levels up"
    );
    browser.toggle(keys);
    assert_eq!(browser.marker(keys), Marker::Unselected);
    assert_eq!(
        browser.marker(public),
        Marker::Selected,
        "the sibling inside the same folder stays ticked"
    );
    assert_eq!(
        browser.marker(depot),
        Marker::Partial,
        "the chain's folder becomes a dot"
    );
    assert_eq!(browser.marker(folder), Marker::Partial);
    let mut units: Vec<NodeId> = browser.selected().collect();
    units.sort();
    assert_eq!(units, {
        let mut expected = vec![NodeId::derive(folder, EntryKind::File, "beleg.pdf"), public];
        expected.sort();
        expected
    });
}

#[test]
fn selecting_folder_clears_explicit_descendants() {
    let mut browser = Browser::sample();
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    browser.toggle(file);
    browser.toggle(NodeId::ROOT);
    assert_eq!(browser.selected().collect::<Vec<_>>(), vec![NodeId::ROOT]);
}

#[test]
fn select_all_marks_current_children() {
    let mut browser = Browser::sample();
    browser.select_all();
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    assert!(browser.selected().collect::<Vec<_>>().contains(&file));
    assert!(browser.selected().collect::<Vec<_>>().contains(&folder));
    // The scope itself stays unselected, marked Partial by its selected children.
    assert_eq!(browser.marker(NodeId::ROOT), Marker::Partial);
}

#[test]
fn scope_marker_tracks_the_scope_selection() {
    let mut browser = Browser::sample();
    assert_eq!(
        browser.scope_marker(),
        Marker::Unselected,
        "nothing is checked"
    );
    let file = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    browser.toggle(file);
    assert_eq!(
        browser.scope_marker(),
        Marker::Partial,
        "one of the two children is checked"
    );
    browser.select_all();
    assert_eq!(
        browser.scope_marker(),
        Marker::Selected,
        "now every child is"
    );
    browser.clear_selection();
    browser.toggle(NodeId::ROOT);
    assert_eq!(
        browser.scope_marker(),
        Marker::Selected,
        "the scope itself counts too"
    );
}

#[test]
fn toggle_all_selects_then_clears_only_inside_the_scope() {
    let mut browser = Browser::sample();
    let outside = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    browser.toggle(outside);
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let _ = browser.activate(folder);
    browser.set_children(folder, vec![Entry::folder("Krypto")]);
    let inside = NodeId::derive(folder, EntryKind::Folder, "Krypto");
    assert_eq!(
        browser.scope_marker(),
        Marker::Unselected,
        "a selection one level up is outside this scope"
    );
    browser.toggle_all();
    let selected: Vec<NodeId> = browser.selected().collect();
    assert!(
        selected.contains(&inside),
        "this directory's children got checked"
    );
    assert!(
        selected.contains(&outside),
        "the selection from one level up stays"
    );
    browser.toggle_all();
    assert_eq!(
        browser.selected().collect::<Vec<_>>(),
        vec![outside],
        "the second click clears only this directory"
    );
}

#[test]
fn single_selection_replaces_the_selection() {
    let mut browser = Browser::sample();
    browser.set_selection(crate::Selection::Single);
    let folder = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "Finanzen");
    let notes = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    browser.toggle(folder);
    browser.toggle(notes);
    assert_eq!(
        browser.selected().collect::<Vec<_>>(),
        vec![notes],
        "the second tap replaced the first"
    );
    browser.toggle_all();
    assert_eq!(
        browser.selected().collect::<Vec<_>>(),
        vec![NodeId::ROOT],
        "the header box ticks the scope itself as the one unit"
    );
    // Tapping a box covered by that unit clears it — there is no dissolving in single mode.
    browser.toggle(notes);
    assert!(
        !browser.has_selection(),
        "the covered tap unticks the only unit"
    );
    // Select-all under the mode selects the scope, never every child.
    browser.select_all();
    assert_eq!(browser.selected().collect::<Vec<_>>(), vec![NodeId::ROOT]);
}

#[test]
fn selection_none_never_selects() {
    let mut browser = Browser::sample();
    let notes = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    browser.set_selection(crate::Selection::None);
    browser.toggle(notes);
    assert!(
        !browser.has_selection(),
        "toggles are ignored in the pure view"
    );
    assert!(
        !browser.boxes_visible(),
        "a pure view has no boxes whatever Checkboxes says"
    );
    assert_eq!(
        browser.tap(notes),
        Outcome::FileOpened(notes),
        "files just report"
    );
    assert_eq!(browser.activate(notes), Outcome::FileOpened(notes));
    // Existing selections are dropped on the way in, and selection comes back on the way out.
    browser.set_selection(crate::Selection::Multiple);
    browser.toggle(notes);
    assert!(browser.has_selection());
    browser.set_selection(crate::Selection::None);
    assert!(
        !browser.has_selection(),
        "and switching into the view drops it"
    );
}

#[test]
fn selected_yields_in_tick_order_and_single_keeps_the_last() {
    // 20 runs: a HashSet order that happens to match the tick order in one process would pass by luck.
    for _ in 0..20 {
        let mut browser = Browser::new("r");
        browser.set_children(
            NodeId::ROOT,
            vec![
                Entry::file("a.txt"),
                Entry::file("b.txt"),
                Entry::file("c.txt"),
            ],
        );
        let a = NodeId::derive(NodeId::ROOT, EntryKind::File, "a.txt");
        let b = NodeId::derive(NodeId::ROOT, EntryKind::File, "b.txt");
        let c = NodeId::derive(NodeId::ROOT, EntryKind::File, "c.txt");
        browser.toggle(a);
        browser.toggle(b);
        browser.toggle(c);
        assert_eq!(
            browser.selected().collect::<Vec<_>>(),
            vec![a, b, c],
            "selected() yields in selection order"
        );
        browser.set_selection(crate::Selection::Single);
        assert_eq!(
            browser.selected().collect::<Vec<_>>(),
            vec![c],
            "entering Single keeps the most recently selected unit"
        );
    }
}
