//! The cabinet as a picker in a modal: a settings form whose fields are filled through "Browse…" instead of typed paths. Three pickers over one fake drive:
//!
//! - **Folder** — one folder ([`Pick::Folder`]): navigate into it and press "Use this folder", or tick it from its parent; the root means the whole drive.
//! - **Excluded** — any files and folders ([`Pick::Entries`]), opened with the saved exclusions ticked even though their folders are not listed yet.
//! - **Hidden from the gallery** — any folders ([`Pick::Folders`]), the `.nomedia` case.
//!
//! Run: `cargo run --example picker`. Cancel, the darkened backdrop or Escape close a picker without touching the form.

use iced::{
    Alignment, Background, Border, Color, Element, Length, Subscription, Task, Theme, keyboard,
    widget::{button, center, column, container, mouse_area, opaque, row, stack, text, text_input},
};
use iced_fabi::{ActionButton, ActionPlacement, Browser, Entry, Event, NodeId, Outcome, Pick};

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .window_size((900.0, 620.0))
        .run()
}

/// The drive the pickers browse: every entry by its full path, folders with a trailing slash.
const DRIVE: &[&str] = &[
    "Documents/",
    "Documents/Scans/",
    "Documents/Scans/2025/",
    "Documents/Scans/2026/",
    "Documents/tax.pdf",
    "Music/",
    "Music/Live/",
    "Music/Studio/",
    "Pictures/",
    "Pictures/Camera/",
    "Pictures/Screenshots/",
    "Pictures/Wallpapers/",
    "Videos/",
    "Videos/Cut/",
    "Videos/Raw/",
    "Videos/clip.mp4",
    "readme.md",
];

/// The entries directly inside the folder at `path` (names below the root, `/`-joined; `""` is the root).
fn listing(path: &str) -> Vec<Entry> {
    let prefix = if path.is_empty() {
        String::new()
    } else {
        format!("{path}/")
    };
    DRIVE
        .iter()
        .filter_map(|entry| entry.strip_prefix(prefix.as_str()))
        .filter(|rest| !rest.is_empty())
        .filter_map(|rest| match rest.split_once('/') {
            Some((name, "")) => Some(Entry::folder(name)),
            Some(_) => None,
            None => Some(Entry::file(rest).size(48_000)),
        })
        .collect()
}

/// Which form field a picker fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Folder,
    Excluded,
    Hidden,
}

#[derive(Debug, Clone)]
enum Message {
    FolderEdited(String),
    Browse(Field),
    Picker(Event),
    /// The listing for a node the picker asked for.
    Listed(NodeId, Vec<Entry>),
    Confirm,
    Cancel,
}

struct App {
    folder: String,
    excluded: Vec<String>,
    hidden: Vec<String>,
    /// The open picker and the field it fills.
    picker: Option<(Field, Browser)>,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let app = App {
            folder: String::new(),
            excluded: vec!["Videos/Raw".into(), "Documents/tax.pdf".into()],
            hidden: vec!["Pictures/Screenshots".into()],
            picker: None,
        };
        (app, Task::none())
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            } => Some(Message::Cancel),
            _ => None,
        })
    }

    /// Answer a `NeedsLoad` with the fake drive's listing.
    fn load(browser: &Browser, outcome: Outcome) -> Task<Message> {
        match outcome {
            Outcome::NeedsLoad(id) => {
                Task::done(Message::Listed(id, listing(&browser.path_of(id))))
            }
            _ => Task::none(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FolderEdited(folder) => self.folder = folder,
            Message::Browse(field) => {
                let mut browser = match field {
                    Field::Folder => Browser::picker("My drive", Pick::Folder),
                    Field::Excluded => {
                        let mut browser = Browser::picker("My drive", Pick::Entries);
                        browser.select_paths(&self.excluded);
                        browser
                    }
                    Field::Hidden => {
                        let mut browser = Browser::picker("My drive", Pick::Folders);
                        browser.select_paths(&self.hidden);
                        browser
                    }
                };
                let outcome = browser.open();
                let task = Self::load(&browser, outcome);
                self.picker = Some((field, browser));
                return task;
            }
            Message::Picker(event) => {
                if let Some((_, browser)) = &mut self.picker {
                    let outcome = browser.update(event);
                    return Self::load(browser, outcome);
                }
            }
            Message::Listed(id, entries) => {
                if let Some((_, browser)) = &mut self.picker {
                    browser.set_children(id, entries);
                }
            }
            Message::Confirm => {
                if let Some((field, browser)) = self.picker.take() {
                    let picked = browser.picked();
                    match field {
                        Field::Folder => {
                            self.folder = picked.into_iter().next().unwrap_or_default()
                        }
                        Field::Excluded => self.excluded = picked,
                        Field::Hidden => self.hidden = picked,
                    }
                }
            }
            Message::Cancel => self.picker = None,
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let browse = |field| {
            button(text("Browse…"))
                .padding([6, 12])
                .on_press(Message::Browse(field))
        };
        let list = |paths: &[String]| {
            text(if paths.is_empty() {
                "—".to_owned()
            } else {
                paths.join(", ")
            })
            .width(Length::Fill)
        };
        let line = |label, content: Element<'static, Message>, field| {
            row![
                text(label).width(Length::Fixed(200.0)),
                content,
                browse(field)
            ]
            .spacing(12)
            .align_y(Alignment::Center)
        };
        let form = column![
            text("Add a folder").size(20),
            line(
                "Folder on the drive",
                text_input("e.g. Documents/Scans (empty = whole drive)", &self.folder)
                    .on_input(Message::FolderEdited)
                    .padding(7)
                    .into(),
                Field::Folder,
            ),
            line("Excluded", list(&self.excluded).into(), Field::Excluded),
            line(
                "Hidden from the gallery",
                list(&self.hidden).into(),
                Field::Hidden
            ),
        ]
        .spacing(16)
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill);

        let Some((field, browser)) = &self.picker else {
            return form.into();
        };
        let (title, confirm) = match field {
            Field::Folder => ("Choose a folder", "Use this folder"),
            Field::Excluded => ("Exclude from sync", "Save"),
            Field::Hidden => ("Hide from the gallery", "Save"),
        };
        let mut action =
            ActionButton::new(confirm, Message::Confirm).placement(ActionPlacement::Footer);
        if *field != Field::Folder {
            // Unticking every saved entry is an answer too.
            action = action.when(|_| true);
        }
        let cabinet = iced_fabi::cabinet(browser, Message::Picker)
            .show_mode_toggle(*field != Field::Folder)
            .action(action)
            .cancel(Message::Cancel)
            .height(Length::Fixed(440.0));
        let card = container(column![
            container(text(title).size(18)).padding([12, 16]),
            cabinet,
        ])
        .width(Length::Fixed(620.0))
        .style(|theme: &Theme| {
            let style = iced_fabi::Style::from_theme(theme);
            container::Style {
                background: Some(Background::Color(style.background)),
                text_color: Some(style.text),
                border: Border {
                    radius: style.radius.into(),
                    width: 1.0,
                    color: style.line,
                },
                ..container::Style::default()
            }
        });
        let scrim = mouse_area(
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.45,
                        ..Color::BLACK
                    })),
                    ..container::Style::default()
                }),
        )
        .on_press(Message::Cancel);
        stack![form, opaque(scrim), center(opaque(card))].into()
    }
}
