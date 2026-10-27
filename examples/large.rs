//! Timing harness for the cabinet's per-frame work — no window, run it in release mode: `cargo run --release --example large`.
//!
//! Two fixtures: one folder with 10 000 files (browse mode) and a tree of three levels — 30 folders, each holding 30 folders, each holding 30 files — fully expanded (tree mode). For each view the harness times `cabinet(..).build()`, the whole per-frame path of the widget: sorting, the visibility filters, the selection markers and one row widget per entry. `set_children` with 10 000 entries is timed on its own.
//!
//! The row builders themselves (`browse_rows` / `tree_rows`) are `pub(crate)` and stay out of an example's reach, so what is measured is the build, including its fixed toolbar, header and scrollable cost — a few hundred µs, as the empty-browser line shows.

use std::time::{Duration, Instant};

use iced_fabi::{Browser, Entry, EntryKind, Mode, NodeId};

/// Files in the single big folder (browse mode).
const FILES: usize = 10_000;

/// Folders per level of the tree fixture, and files in each of its lowest folders.
const FAN_OUT: usize = 30;

/// Runs per measurement; the mean and the fastest run are printed.
const RUNS: usize = 100;

/// The folder holding `FILES` files, and the browse view inside it.
fn big_folder() -> Browser {
    let mut browser = Browser::new("large");
    browser.set_children(NodeId::ROOT, vec![Entry::folder("big")]);
    let big = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "big");
    let _ = browser.activate(big);
    browser.set_children(big, file_entries(FILES, ""));
    browser
}

/// The tree fixture: root → `FAN_OUT` folders → each `FAN_OUT` folders → each `FAN_OUT` files, every branch expanded. Answers the browser and its node count.
fn deep_tree() -> (Browser, usize) {
    let mut browser = Browser::new("large");
    browser.set_mode(Mode::Tree);
    let level1: Vec<String> = (0..FAN_OUT).map(|i| format!("dir-{i:02}")).collect();
    // The root starts expanded; `activate` would collapse it again.
    browser.set_children(
        NodeId::ROOT,
        level1
            .iter()
            .map(|name| Entry::folder(name.clone()))
            .collect(),
    );
    let mut nodes = 1 + level1.len();
    let mut level2: Vec<(NodeId, String)> = Vec::new();
    for name in &level1 {
        let id = NodeId::derive(NodeId::ROOT, EntryKind::Folder, name);
        let kids: Vec<String> = (0..FAN_OUT).map(|i| format!("sub-{i:02}")).collect();
        browser.set_children(
            id,
            kids.iter().map(|kid| Entry::folder(kid.clone())).collect(),
        );
        let _ = browser.activate(id);
        nodes += kids.len();
        for kid in kids {
            level2.push((id, kid));
        }
    }
    for (parent, kid) in &level2 {
        let id = NodeId::derive(*parent, EntryKind::Folder, kid);
        browser.set_children(id, file_entries(FAN_OUT, "leaf-"));
        let _ = browser.activate(id);
        nodes += FAN_OUT;
    }
    (browser, nodes)
}

/// `count` file entries named `<prefix>file-<n>.txt` with a varying size, so the size column has something to sort.
fn file_entries(count: usize, prefix: &str) -> Vec<Entry> {
    (0..count)
        .map(|i| {
            Entry::file(format!("{prefix}file-{i:05}.txt"))
                .size((i as u64 % 97) * 1024)
                .modified(1_700_000_000 + i as i64)
        })
        .collect()
}

/// One measured scenario: what runs, how often, and how many rows the view holds.
fn measure(name: &str, rows: usize, runs: usize, mut work: impl FnMut()) {
    measure_with(name, rows, runs, || (), |()| work());
}

/// The same, with the input built outside the timed span.
fn measure_with<P>(
    name: &str,
    rows: usize,
    runs: usize,
    mut prepare: impl FnMut() -> P,
    mut work: impl FnMut(P),
) {
    work(prepare());
    let mut total = Duration::ZERO;
    let mut fastest = Duration::MAX;
    for _ in 0..runs {
        let input = prepare();
        let start = Instant::now();
        work(input);
        let elapsed = start.elapsed();
        total += elapsed;
        fastest = fastest.min(elapsed);
    }
    let mean = total / runs as u32;
    println!(
        "{name:<26} {rows:>6} rows   mean {:>9.3} ms   fastest {:>8.3} ms   per row {:>7.3} µs",
        mean.as_secs_f64() * 1e3,
        fastest.as_secs_f64() * 1e3,
        mean.as_secs_f64() * 1e6 / rows.max(1) as f64,
    );
}

/// Rows the widget would draw, recomputed through the public API so a fixture that fails to expand or list cannot pass as a large one.
fn listed_rows(browser: &Browser) -> usize {
    match browser.mode() {
        Mode::Browse => browser
            .node(browser.current())
            .map_or(0, |node| node.children.len()),
        Mode::Tree => {
            let mut rows = 0;
            let mut open = vec![NodeId::ROOT];
            while let Some(id) = open.pop() {
                rows += 1;
                if browser.is_expanded(id)
                    && let Some(node) = browser.node(id)
                {
                    open.extend(node.children.iter().copied());
                }
            }
            rows
        }
    }
}

/// The whole widget, as one frame builds it.
fn build(browser: &Browser) {
    let cabinet: iced_fabi::Cabinet<'_, (), iced::Renderer> = iced_fabi::cabinet(browser, |_| ());
    drop(cabinet.build());
}

fn main() {
    println!(
        "iced_fabi {} — cargo run --release --example large ({} runs each)",
        env!("CARGO_PKG_VERSION"),
        RUNS
    );

    let browser = big_folder();
    let browse_rows = listed_rows(&browser);
    assert_eq!(browse_rows, FILES, "the browse fixture lists its files");
    measure("build browse 10k files", browse_rows, RUNS, || {
        build(&browser);
    });

    let mut hidden = big_folder();
    hidden.hide_column(1);
    hidden.hide_column(2);
    measure(
        "build browse 10k, 2 hidden",
        listed_rows(&hidden),
        RUNS,
        || build(&hidden),
    );

    let mut by_size = big_folder();
    by_size.sort_by_column(2);
    measure("build browse 10k by size", browse_rows, RUNS, || {
        build(&by_size);
    });

    let (tree, nodes) = deep_tree();
    let tree_rows = listed_rows(&tree);
    assert_eq!(
        tree_rows, nodes,
        "the tree fixture has every branch expanded"
    );
    measure("build tree 30x30x30", tree_rows, RUNS, || build(&tree));

    let (mut selected_tree, _) = deep_tree();
    for i in (0..FAN_OUT).step_by(3) {
        let top = NodeId::derive(
            NodeId::ROOT,
            EntryKind::Folder,
            format!("dir-{i:02}").as_str(),
        );
        let leaf = NodeId::derive(top, EntryKind::Folder, "sub-07");
        selected_tree.toggle(NodeId::derive(leaf, EntryKind::File, "leaf-file-00007.txt"));
    }
    measure(
        "build tree, 10 files ticked",
        listed_rows(&selected_tree),
        RUNS,
        || build(&selected_tree),
    );

    let mut fresh = big_folder();
    let big = NodeId::derive(NodeId::ROOT, EntryKind::Folder, "big");
    let entries = file_entries(FILES, "");
    measure_with(
        "set_children 10k entries",
        FILES,
        RUNS,
        || entries.clone(),
        |entries| fresh.set_children(big, entries),
    );

    measure("build empty browser", 0, RUNS, || {
        build(&Browser::new("empty"));
    });
}
