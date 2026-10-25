//! The smallest complete app: the listings come straight from the local disk via `std::fs`. Set-up is one chain, `update` is one call plus one `match` on the [`iced_fabi::Outcome`], `view` is one builder chain — that is the whole integration surface.
//!
//! Run: `cargo run --example minimal -- <directory>` (defaults to the current one). Selecting happens through the checkboxes; the wide button moves the selected entries up one level (out of their folder); rows are browsed by pressing them.

use std::{path::PathBuf, time::UNIX_EPOCH};

use iced::{Element, Task, widget::column};
use iced_fabi::{ActionButton, Browser, Entry, Event, NodeId, Outcome};

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view).run()
}

#[derive(Clone)]
enum Message {
    Cabinet(Event),
    /// Re-read the directory behind this node from disk (the answer to a `NeedsLoad`).
    Refresh(NodeId),
    MoveUp,
}

struct App {
    browser: Browser,
    /// Where the browser's root sits on disk.
    root: PathBuf,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let root = std::env::args()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let title = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        let mut browser = Browser::new(title);
        let outcome = browser.open();
        let task = match outcome {
            Outcome::NeedsLoad(id) => Task::done(Message::Refresh(id)),
            _ => Task::none(),
        };
        (App { browser, root }, task)
    }

    /// Disk path of a browser node: the root plus the names below it — exactly what `path_below_root` lists.
    fn dir_of(&self, id: NodeId) -> PathBuf {
        self.browser
            .path_below_root(id)
            .iter()
            .fold(self.root.clone(), |path, name| path.join(name))
    }

    /// Answer a [`NeedsLoad`](Outcome::NeedsLoad) by listing the node's directory right on the spot.
    fn set_children(&mut self, id: NodeId) {
        let read = std::fs::read_dir(self.dir_of(id)).map(|dir| {
            dir.filter_map(Result::ok)
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let md = e.metadata().ok()?;
                    let modified = md
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_secs() as i64);
                    Some(if md.is_dir() {
                        Entry::folder(name).modified(modified)
                    } else {
                        Entry::file(name).size(md.len()).modified(modified)
                    })
                })
                .collect()
        });
        match read {
            Ok(entries) => self.browser.set_children(id, entries),
            Err(_) => self.browser.load_failed(id),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Cabinet(event) => match self.browser.update(event) {
                Outcome::NeedsLoad(id) => Task::done(Message::Refresh(id)),
                // Nothing further for a synchronous disk app — a networked app would fetch on NeedsLoad here.
                _ => Task::none(),
            },
            Message::Refresh(id) => {
                self.set_children(id);
                Task::none()
            }
            Message::MoveUp => {
                if self.browser.current() != NodeId::ROOT {
                    // Move every selected entry out of its own folder, then re-read whatever directory the browser is looking at.
                    for id in self.browser.selected().collect::<Vec<_>>() {
                        let from = self.dir_of(id);
                        let into = self
                            .browser
                            .node(id)
                            .and_then(|n| n.parent)
                            .map_or(self.root.clone(), |parent| self.dir_of(parent));
                        let _ =
                            std::fs::rename(&from, into.join(from.file_name().unwrap_or_default()));
                    }
                    self.browser.remove_selected();
                    let current = self.browser.current();
                    return Task::done(Message::Refresh(current));
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let cabinet = iced_fabi::cabinet(&self.browser, Message::Cabinet)
            .action(ActionButton::new("Move up", Message::MoveUp));
        column![cabinet].padding(12).into()
    }
}
