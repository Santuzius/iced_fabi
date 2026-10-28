//! A cloud directory browser without the cloud: the listings come from a fake backend that answers every request one tick late, so the `activate → NeedsLoad → set_children` round-trip and its loading states behave like a real network drive.
//!
//! The bar above the cabinet demos the configuration hooks with dropdowns: the [`iced::Theme`] (including a reproduction of Syncative's palette and [`iced_fabi::Style`]), the file/folder [`iced_fabi::RowIcons`] (Tabler, or the vendored multicolour Material Icon Theme), the [`iced_fabi::Separators`] grid, the zebra stripe behind every second row, which entries are listed (all or folders only — file kinds are filtered code-side at init), how many entries the selection holds (many, one, or none), how the boxes appear (always, on long press, or hidden), and where the selection's action button sits (under the list, floating, or last in the toolbar), whether the browse/tree mode switch button is drawn at all ([`Cabinet::show_mode_toggle`](iced_fabi::Cabinet::show_mode_toggle), off by the bar's *Mode switch* dropdown); also what happens to toolbar tools that are not currently applicable (greyed out, or hidden outright). The toolbar itself shows the [`iced_fabi::Tool`] API: the built-in [`iced_fabi::BuiltInTool`] set (create folder, copy, paste) arrives with one `built_in_tools` call, the custom [`DuplicateHere`] is the demo's own `Tool` carrying a selection's subtree and sits among the custom tools, and the destructive delete joins last behind a [`iced_fabi::Cabinet::tool_separator`]; Delete (the built-in's own [`iced_fabi::Tool::confirmation`]) and Duplicate (the demo's) first ask in a confirmation modal, and the confirmed tool comes back as [`iced_fabi::Outcome::ToolConfirmed`]; a `Tools` dropdown switches any of them off ([`Browser::hide_tool`](iced_fabi::Browser::hide_tool)); selecting is done purely through the checkboxes. The column list gets one custom [`TypeColumn`], and a `Columns` dropdown switches which of the columns are drawn ([`Browser::hide_column`] / [`Browser::show_column`]; [`Browser::set_visible_columns`] is the same in one call at init).
//!
//! Run: `cargo run --example cloud_browse` — then switch modes with the icon in the top right, tap folders to drill down, tap the boxes to select (press and hold a row first when "Boxes" is on *on long press*), and watch the parent markers turn into dots.

use std::{collections::HashMap, fmt, time::Duration};

use iced::{
    Element, Length, Task, Theme,
    widget::{column, container},
};
use iced_fabi::{
    Browser, BuiltInColumn, BuiltInTool, Cell, Column, ColumnKey, ColumnWidth, Entry, EntryKind,
    Event, Node, Outcome, Tool, ToolKey,
};

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        // Syncative's UI font; the default themes show it too, since iced fixes the font for the whole run.
        .default_font(SYNCATIVE_FONT)
        // Wide enough for the whole configuration bar.
        .window_size((1300.0, 760.0))
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Cabinet(Event),
    CreateFolder,
    Copy,
    Paste,
    Delete,
    Theme(ThemeChoice),
    Icons(IconChoice),
    Inactive(InactiveChoice),
    Lines(iced_fabi::Separators),
    Zebra(ZebraChoice),
    Boxes(BoxesChoice),
    Entries(EntryChoice),
    Selection(SelectionChoice),
    Action(ActionChoice),
    ModeSwitch(ModeSwitchChoice),
    ColumnToggle(ColumnKey),
    ToolToggle(ToolKey),
    Duplicate,
    Tick,
}

impl From<BuiltInTool> for Message {
    /// The built-in toolbar buttons yield their variant; this maps them onto the demo's messages.
    fn from(tool: BuiltInTool) -> Self {
        match tool {
            BuiltInTool::CreateFolder => Self::CreateFolder,
            BuiltInTool::Copy => Self::Copy,
            BuiltInTool::Paste => Self::Paste,
            BuiltInTool::Delete => Self::Delete,
        }
    }
}

// --- the pieces the demo is built from ---

mod choices;
mod icons;
mod theme;

use choices::{
    ACTION_CHOICES, ActionChoice, BOXES_CHOICES, BoxesChoice, ENTRY_CHOICES, EntryChoice,
    ICON_CHOICES, INACTIVE_CHOICES, IconChoice, InactiveChoice, LINES, MODE_SWITCH,
    ModeSwitchChoice, SELECTION_CHOICES, SelectionChoice, THEMES, ThemeChoice, ZEBRA, ZebraChoice,
};
use icons::MaterialIconTheme;
use theme::{
    SYNCATIVE_DARK, SYNCATIVE_FONT, SYNCATIVE_LIGHT, SYNCATIVE_TEXT, syncative_card,
    syncative_header_bar, syncative_menu, syncative_page, syncative_pick_list,
    syncative_scroll_direction, syncative_scrollbar, syncative_separator, syncative_style,
};

// --- the app ---

/// Pending fake listings, each answering after a number of ticks.
struct Pending {
    node: iced_fabi::NodeId,
    path: String,
    ticks: u8,
}

/// A copied entry together with its whole subtree — the fake cloud keys its listings by path, so a folder's contents have to be walked out of the map to travel with it.
#[derive(Clone)]
struct Copied {
    entry: Entry,
    children: Vec<Copied>,
}

type Clipboard = Vec<Copied>;

struct App {
    browser: Browser,
    /// The "cloud": path → listing. The root's key is "" (paths below the root, as `path_below_root` gives them).
    cloud: HashMap<String, Vec<Entry>>,
    pending: Vec<Pending>,
    clipboard: Clipboard,
    theme: ThemeChoice,
    icons: IconChoice,
    inactive: InactiveChoice,
    lines: iced_fabi::Separators,
    zebra: ZebraChoice,
    boxes: BoxesChoice,
    entries: EntryChoice,
    selection: SelectionChoice,
    action: ActionChoice,
    mode_switch: ModeSwitchChoice,
    /// The Columns dropdown's entries, refreshed whenever the columns change.
    column_options: Vec<ColumnOption>,
    /// The Tools dropdown's entries, refreshed whenever the tools change.
    tool_options: Vec<ToolOption>,
}

impl App {
    /// Rebuild the Columns dropdown's entries from the browser's columns (all but the always-on Name anchor).
    fn refresh_columns(&mut self) {
        self.column_options = (1..self.browser.column_count())
            .map(|index| {
                let column = self
                    .browser
                    .column(index)
                    .expect("index inside column_count");
                let key = ColumnKey::custom(column.id());
                let visible = !self.browser.is_column_hidden(key.clone());
                ColumnOption {
                    key,
                    label: column.header(),
                    visible,
                }
            })
            .collect();
    }

    /// Rebuild the Tools dropdown's entries: the library's built-in set and the demo's own tool, checked against the browser's hidden ids.
    fn refresh_tools(&mut self) {
        let mut options: Vec<ToolOption> = BuiltInTool::ALL
            .into_iter()
            .map(|tool| ToolOption {
                key: ToolKey::from(tool),
                label: tool.label().to_string(),
                visible: !self.browser.is_tool_hidden(tool),
            })
            .collect();
        options.push(ToolOption {
            key: ToolKey::custom(DuplicateHere::ID),
            label: "Duplicate".into(),
            visible: !self.browser.is_tool_hidden(DuplicateHere::ID),
        });
        self.tool_options = options;
    }

    /// The selected entries snapshotted with their subtrees, as the clipboard and a duplicate both need them.
    fn snapshot_selection(&self) -> Vec<Copied> {
        self.browser
            .selected()
            .filter_map(|id| {
                let below = self.browser.path_below_root(id);
                let name = (*below.last()?).to_string();
                let parent = below[..below.len() - 1].join("/");
                let entry = self
                    .cloud
                    .get(&parent)?
                    .iter()
                    .find(|entry| entry.name == name)?
                    .clone();
                Some(snapshot(&self.cloud, &parent, &entry))
            })
            .collect()
    }

    /// Land snapshots as copies in the current folder, each under a free copy name, subtree re-homed; used by Paste and by the demo's Duplicate tool.
    fn put_copies(&mut self, copies: Vec<Copied>) {
        let key = self
            .browser
            .path_below_root(self.browser.current())
            .join("/");
        let mut listing = self.cloud.get(&key).cloned().unwrap_or_default();
        for copied in copies {
            let mut top = copied;
            top.entry.name = unique_copy_name(&top.entry.name, &listing);
            listing.push(top.entry.clone());
            // The subtree is re-homed at the renamed top; opening the copy later refetches these listings normally.
            write_copy(&mut self.cloud, &key, &top);
        }
        self.cloud.insert(key.clone(), listing.clone());
        self.browser.set_children(self.browser.current(), listing);
    }
    fn boot() -> (Self, Task<Message>) {
        let mut browser = Browser::new("My files");
        browser.add_column(TypeColumn);
        // All four columns on. At init a whitelist in one call is enough instead: writing only
        // `[ColumnKey::built_in(BuiltInColumn::Size), ColumnKey::custom("Type")]` would start with Modified hidden.
        browser.set_visible_columns([
            ColumnKey::built_in(BuiltInColumn::Modified),
            ColumnKey::built_in(BuiltInColumn::Size),
            ColumnKey::custom("Type"),
        ]);
        // The file kinds the cabinet may show — an app-side decision made here at init; to offer only PDFs, write ["pdf"].
        browser.set_file_extensions(["pdf", "docx", "md", "txt"]);
        // Ask for the root immediately; everything else loads on navigation.
        let outcome = browser.open();
        let mut app = App {
            browser,
            cloud: fake_cloud(),
            pending: Vec::new(),
            clipboard: Vec::new(),
            theme: ThemeChoice::SyncativeDark,
            icons: IconChoice::MaterialIconTheme,
            inactive: InactiveChoice::GreyedOut,
            lines: iced_fabi::Separators::Both,
            zebra: ZebraChoice::Off,
            boxes: BoxesChoice::Shown,
            entries: EntryChoice::All,
            selection: SelectionChoice::Multiple,
            action: ActionChoice::Wide,
            mode_switch: ModeSwitchChoice::Shown,
            column_options: Vec::new(),
            tool_options: Vec::new(),
        };
        app.refresh_columns();
        app.refresh_tools();
        let task = Task::done(app.accept(outcome));
        (app, task)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Cabinet(event) => {
                let outcome = self.browser.update(event);
                Task::done(self.accept(outcome))
            }
            Message::ColumnToggle(column) => {
                if self.browser.is_column_hidden(column.clone()) {
                    self.browser.show_column(column);
                } else {
                    self.browser.hide_column(column);
                }
                self.refresh_columns();
                Task::none()
            }
            Message::ToolToggle(key) => {
                if self.browser.is_tool_hidden(key.id()) {
                    self.browser.show_tool(key);
                } else {
                    self.browser.hide_tool(key);
                }
                self.refresh_tools();
                Task::none()
            }
            Message::Boxes(choice) => {
                self.boxes = choice;
                self.browser.set_checkboxes(match choice {
                    BoxesChoice::Shown => iced_fabi::Checkboxes::Shown,
                    BoxesChoice::OnLongPress => iced_fabi::Checkboxes::OnLongPress,
                    BoxesChoice::Hidden => iced_fabi::Checkboxes::Hidden,
                });
                Task::none()
            }
            Message::Entries(choice) => {
                self.entries = choice;
                self.browser
                    .set_folders_only(choice == EntryChoice::FoldersOnly);
                Task::none()
            }
            Message::Selection(choice) => {
                self.selection = choice;
                self.browser.set_selection(match choice {
                    SelectionChoice::Multiple => iced_fabi::Selection::Multiple,
                    SelectionChoice::Single => iced_fabi::Selection::Single,
                    SelectionChoice::None => iced_fabi::Selection::None,
                });
                Task::none()
            }
            Message::Action(choice) => {
                self.action = choice;
                Task::none()
            }
            Message::ModeSwitch(choice) => {
                self.mode_switch = choice;
                Task::none()
            }
            Message::CreateFolder => {
                // This message now only comes from the demo's own shortcut paths: the built-in toolbar button opens the dialog itself. The dialog asks the name; the confirmed one arrives as `Outcome::CreateFolderRequested`.
                self.browser.open_new_folder_dialog();
                Task::none()
            }
            Message::Copy => {
                self.clipboard = self.snapshot_selection();
                // The built-in Paste button reads this flag; the file data itself stays in the fake cloud.
                self.browser.set_has_clipboard(!self.clipboard.is_empty());
                // A tool that acted is done acting: the boxes go back to empty.
                self.browser.clear_selection();
                Task::none()
            }
            Message::Paste => {
                self.put_copies(self.clipboard.clone());
                self.browser.clear_selection();
                Task::none()
            }
            Message::Duplicate => {
                // The toolbar no longer emits this: Duplicate asks first and the confirmed click comes back as `Outcome::ToolConfirmed`.
                self.duplicate_selection();
                Task::none()
            }
            Message::Delete => {
                // Both delete affordances — the toolbar's Delete (the built-in asks on its own) and this action button — go through the confirmation modal first; the work happens when `Outcome::ToolConfirmed` arrives.
                self.browser
                    .open_tool_confirmation(BuiltInTool::Delete.into());
                Task::none()
            }
            Message::Theme(choice) => {
                self.theme = choice;
                Task::none()
            }
            Message::Icons(choice) => {
                self.icons = choice;
                Task::none()
            }
            Message::Inactive(choice) => {
                self.inactive = choice;
                Task::none()
            }
            Message::Lines(choice) => {
                self.lines = choice;
                Task::none()
            }
            Message::Zebra(choice) => {
                self.zebra = choice;
                Task::none()
            }
            Message::Tick => {
                let mut ready = Vec::new();
                let mut still = Vec::new();
                for mut item in self.pending.drain(..) {
                    if item.ticks == 0 {
                        ready.push(item);
                    } else {
                        item.ticks -= 1;
                        still.push(item);
                    }
                }
                self.pending = still;
                for item in ready {
                    // A listing that raced a refresh just no-ops: set_children does nothing for pruned nodes.
                    let listing = self.cloud.get(&item.path).cloned().unwrap_or_default();
                    self.browser.set_children(item.node, listing);
                }
                Task::none()
            }
        }
    }

    /// Queue the fetch a [`Outcome::NeedsLoad`] asks for.
    fn accept(&mut self, outcome: Outcome) -> Message {
        match outcome {
            Outcome::NeedsLoad(id) => {
                let path = self.browser.path_below_root(id).join("/");
                self.pending.push(Pending {
                    node: id,
                    path,
                    ticks: 2,
                });
            }
            // This example has no files to open; tapping a file selects it instead.
            Outcome::FileOpened(id) => {
                self.browser.toggle(id);
            }
            Outcome::SelectionChanged | Outcome::None => {}
            Outcome::CreateFolderRequested { parent, name } => {
                // The new folder goes into the fake cloud under `parent`; a real backend would get the create call instead. The browser already made `name` free of the siblings it has listed.
                let key = self.browser.path_below_root(parent).join("/");
                let mut listing = self.cloud.get(&key).cloned().unwrap_or_default();
                listing.push(Entry::folder(name).modified(now_epoch()));
                self.cloud.insert(key.clone(), listing.clone());
                self.browser.set_children(parent, listing);
            }
            Outcome::ToolConfirmed { tool } => match tool.id() {
                // A confirmed tool's action: its button press never messaged the app, so the held-back work runs here, named by the tool's switch id.
                "delete" => self.delete_selected(),
                DuplicateHere::ID => self.duplicate_selection(),
                _ => {}
            },
        }
        Message::Tick
    }

    /// Drop the selected entries from the fake cloud (with their subtrees) and from the browser; a real backend would get the delete call instead.
    fn delete_selected(&mut self) {
        for id in self.browser.selected().collect::<Vec<_>>() {
            let path = self.browser.path_below_root(id).join("/");
            delete_paths(&mut self.cloud, &path);
        }
        self.browser.remove_selected();
        let key = self
            .browser
            .path_below_root(self.browser.current())
            .join("/");
        let listing = self.cloud.get(&key).cloned().unwrap_or_default();
        self.browser.set_children(self.browser.current(), listing);
    }

    /// The demo's custom tool's work: copy and paste in one step — the selection lands back beside itself under a copy name.
    fn duplicate_selection(&mut self) {
        let copies = self.snapshot_selection();
        self.put_copies(copies);
        self.browser.clear_selection();
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        // The fake backend's tick pump only runs while a listing is in flight.
        let mut subscriptions = Vec::new();
        if !self.pending.is_empty() {
            subscriptions
                .push(iced::time::every(Duration::from_millis(120)).map(|_| Message::Tick));
        }
        iced::Subscription::batch(subscriptions)
    }

    fn theme(&self) -> Theme {
        match self.theme {
            ThemeChoice::DefaultLight => Theme::Light,
            ThemeChoice::DefaultDark => Theme::Dark,
            ThemeChoice::SyncativeLight => (*SYNCATIVE_LIGHT).clone(),
            ThemeChoice::SyncativeDark => (*SYNCATIVE_DARK).clone(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let syncative = matches!(
            self.theme,
            ThemeChoice::SyncativeLight | ThemeChoice::SyncativeDark
        );
        // The cabinet's own header shows "N selected" while something is checked, so the window title stays put (and "My files" does not appear twice — the breadcrumb already says it).
        let title = "iced_fabi demo";
        // Under the Syncative themes the bar's labels and drop-downs take Syncative's size and styles, like the rest of the window.
        let label = |content: &'static str| {
            let label = iced::widget::text(content);
            if syncative {
                label.size(SYNCATIVE_TEXT)
            } else {
                label
            }
        };
        fn dropdown<'a, T: fmt::Display + PartialEq + Clone + 'a>(
            options: &'a [T],
            selected: &T,
            on_select: impl Fn(T) -> Message + 'a,
            width: f32,
            syncative: bool,
        ) -> Element<'a, Message> {
            let list = iced::widget::pick_list(options, Some(selected.clone()), on_select)
                .width(Length::Fixed(width));
            if syncative {
                list.text_size(SYNCATIVE_TEXT)
                    .style(syncative_pick_list)
                    .menu_style(syncative_menu)
                    .into()
            } else {
                list.into()
            }
        }
        fn columns_list<'a>(
            options: &'a [ColumnOption],
            syncative: bool,
        ) -> Element<'a, Message> {
            let list = iced::widget::pick_list(options, None::<&ColumnOption>, |option| {
                Message::ColumnToggle(option.key.clone())
            })
            .width(Length::Fixed(150.0))
            .placeholder("pick columns");
            if syncative {
                list.text_size(SYNCATIVE_TEXT)
                    .style(syncative_pick_list)
                    .menu_style(syncative_menu)
                    .into()
            } else {
                list.into()
            }
        }
        fn tools_list<'a>(
            options: &'a [ToolOption],
            syncative: bool,
        ) -> Element<'a, Message> {
            let list = iced::widget::pick_list(options, None::<&ToolOption>, |option| {
                Message::ToolToggle(option.key.clone())
            })
            .width(Length::Fixed(150.0))
            .placeholder("pick tools");
            if syncative {
                list.text_size(SYNCATIVE_TEXT)
                    .style(syncative_pick_list)
                    .menu_style(syncative_menu)
                    .into()
            } else {
                list.into()
            }
        }
        let bar = iced::widget::column![
            iced::widget::text(title).size(if syncative { 24 } else { 20 }),
            iced::widget::row![
                label("Theme"),
                dropdown(THEMES, &self.theme, Message::Theme, 150.0, syncative),
                label("Icons"),
                dropdown(ICON_CHOICES, &self.icons, Message::Icons, 190.0, syncative),
                label("Inactive tools"),
                dropdown(
                    INACTIVE_CHOICES,
                    &self.inactive,
                    Message::Inactive,
                    130.0,
                    syncative
                ),
                label("Lines"),
                dropdown(LINES, &self.lines, Message::Lines, 130.0, syncative),
                label("Columns"),
                columns_list(&self.column_options, syncative),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            iced::widget::row![
                label("Zebra"),
                dropdown(ZEBRA, &self.zebra, Message::Zebra, 100.0, syncative),
                label("Entries"),
                dropdown(
                    ENTRY_CHOICES,
                    &self.entries,
                    Message::Entries,
                    140.0,
                    syncative
                ),
                label("Selection"),
                dropdown(
                    SELECTION_CHOICES,
                    &self.selection,
                    Message::Selection,
                    110.0,
                    syncative
                ),
                label("Boxes"),
                dropdown(BOXES_CHOICES, &self.boxes, Message::Boxes, 120.0, syncative),
                label("Action"),
                dropdown(
                    ACTION_CHOICES,
                    &self.action,
                    Message::Action,
                    120.0,
                    syncative
                ),
                label("Tools"),
                tools_list(&self.tool_options, syncative),
                label("Mode switch"),
                dropdown(
                    MODE_SWITCH,
                    &self.mode_switch,
                    Message::ModeSwitch,
                    100.0,
                    syncative
                ),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8);
        // Tool state is snapshotted into the `when` closures: the toolbar rebuilds every frame.
        let has_selection = self.browser.has_selection();
        let has_clipboard = !self.clipboard.is_empty();
        // With the hidden setting, a tool that does not apply is left out of the toolbar entirely; otherwise it stays and `when` greys it.
        let show = |enabled: bool| self.inactive == InactiveChoice::GreyedOut || enabled;
        let (base_style, stripe_factor): (fn(&Theme) -> iced_fabi::Style, f32) = match self.theme {
            ThemeChoice::DefaultLight | ThemeChoice::DefaultDark => {
                (iced_fabi::Style::from_theme, self.zebra.stripe())
            }
            ThemeChoice::SyncativeLight | ThemeChoice::SyncativeDark => {
                (syncative_style, self.zebra.stripe())
            }
        };
        // The dropdown nudges only the zebra strength of the theme's own style.
        let style = move |theme: &Theme| iced_fabi::Style {
            stripe: stripe_factor,
            ..base_style(theme)
        };
        let icons: &dyn iced_fabi::RowIcons = match self.icons {
            IconChoice::Tabler => &iced_fabi::TablerIcons,
            IconChoice::MaterialIconTheme => &MaterialIconTheme,
        };
        let mut cabinet = iced_fabi::cabinet(&self.browser, Message::Cabinet)
            .style(style)
            .icons(icons)
            .separators(self.lines)
            // The app's default_font is Noto Sans under every theme (see `main`), so the cabinet labels with it too; without it its semibold breadcrumb and "N selected" fall back to iced's default family.
            .font(SYNCATIVE_FONT)
            // The Mode switch dropdown: off drops the browse/tree button and its divider from the bar entirely.
            .show_mode_toggle(self.mode_switch == ModeSwitchChoice::Shown);
        if syncative {
            cabinet = cabinet
                .text_size(SYNCATIVE_TEXT)
                .scrollbar(syncative_scroll_direction(), syncative_scrollbar);
        }
        // The library's built-in set — Create folder, Copy, Paste — in one call, then the demo's own `DuplicateHere` among the custom tools, then a divider and the destructive Delete as its own group; each button greys itself from browser state (selection, the clipboard flag; Create folder always applies), and the Tools dropdown switches any of them off entirely. With "hidden" inactive tools the app leaves out what does not apply instead, and since the built-ins are plain values that works one by one — the cabinet drops the stray dividers itself.
        if self.inactive == InactiveChoice::GreyedOut {
            cabinet = cabinet
                .built_in_tools()
                .tool(DuplicateHere)
                .tool_separator()
                .tool(BuiltInTool::Delete);
        } else {
            cabinet = cabinet.tool(BuiltInTool::CreateFolder);
            if show(has_selection) {
                cabinet = cabinet.tool(BuiltInTool::Copy);
            }
            if show(has_clipboard) {
                cabinet = cabinet.tool(BuiltInTool::Paste);
            }
            if show(has_selection) {
                cabinet = cabinet.tool(DuplicateHere);
            }
            cabinet = cabinet.tool_separator();
            if show(has_selection) {
                cabinet = cabinet.tool(BuiltInTool::Delete);
            }
        }
        // The selection's action button: text and message are the app's, placement is picked in the bar.
        if show(has_selection) {
            cabinet = cabinet.action(
                iced_fabi::ActionButton::new("Delete selected", Message::Delete).placement(
                    match self.action {
                        ActionChoice::Wide => iced_fabi::ActionPlacement::WideCenter,
                        ActionChoice::Floating => iced_fabi::ActionPlacement::Floating,
                        ActionChoice::Toolbar => iced_fabi::ActionPlacement::ToolbarEnd,
                    },
                ),
            );
        }
        if !syncative {
            return column![bar, cabinet].padding(16).spacing(8).into();
        }
        // Syncative's page: the header bar over a hairline, the content on a card on the page background (paddings from `syncative/src/screens/remote_page.rs`). The card hugs the cabinet and takes its corner radius from the cabinet's own style, so the modal scrim's darkening covers the card to its rounded border.
        let header = container(bar)
            .padding(iced::Padding {
                top: 18.0,
                left: 24.0,
                right: 24.0,
                bottom: 14.0,
            })
            .width(Length::Fill)
            .style(syncative_header_bar);
        let card = container(cabinet.build())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme| syncative_card(theme, syncative_style(theme).radius));
        container(column![
            header,
            iced::widget::rule::horizontal(1).style(syncative_separator),
            container(card).padding(iced::Padding {
                top: 16.0,
                left: 24.0,
                right: 24.0,
                bottom: 24.0,
            }),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(syncative_page)
        .into()
    }
}

/// One entry of the Columns dropdown: which column it stands for, and how it reads there (a check mark for a drawn column, a cross for a hidden one).
#[derive(Clone, PartialEq)]
struct ColumnOption {
    key: ColumnKey,
    label: String,
    visible: bool,
}

impl fmt::Display for ColumnOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.visible { "✓ " } else { "✗ " })?;
        f.write_str(&self.label)
    }
}

/// A custom column: the file extension in capitals, folders as "Folder"; sorted by kind first, then extension.
#[derive(Debug)]
struct TypeColumn;

/// One entry of the Tools dropdown: which tool it stands for, and how it reads there (a check mark for a drawn tool, a cross for a hidden one).
#[derive(Clone, PartialEq)]
struct ToolOption {
    key: ToolKey,
    label: String,
    visible: bool,
}

impl fmt::Display for ToolOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.visible { "✓ " } else { "✗ " })?;
        f.write_str(&self.label)
    }
}

/// The demo's own toolbar tool — the bar-side counterpart of the custom column: the [`iced_fabi::Tool`] implementation supplies glyph, switch id, applicability, message and a [confirmation modal](iced_fabi::Tool::confirmation); the id is what [`Browser::hide_tool`](iced_fabi::Browser::hide_tool) and the Tools dropdown match on, and what the confirmed modal comes back as. What the click does (copy and paste in one step) is entirely the demo's.
struct DuplicateHere;

impl DuplicateHere {
    /// The switch id the Tools dropdown and [`Browser::hide_tool`](iced_fabi::Browser::hide_tool) name this tool by.
    const ID: &'static str = "duplicate";
}

impl Tool<Message> for DuplicateHere {
    fn glyph(&self) -> icondata::Icon {
        icondata::TbCopyPlusOutline
    }

    fn id(&self) -> Option<String> {
        Some(Self::ID.to_owned())
    }

    fn enabled(&self, browser: &Browser) -> bool {
        browser.has_selection()
    }

    fn message(&self, _browser: &Browser) -> Message {
        Message::Duplicate
    }

    fn confirmation(&self, browser: &Browser, _labels: &iced_fabi::Labels) -> Option<iced_fabi::ToolConfirmation> {
        // A custom modal, the demo's own words: the cabinet asks again each frame, so the count tracks the live selection. The confirm button names the act.
        Some(iced_fabi::ToolConfirmation {
            title: "Duplicate here?".into(),
            body: Some(format!(
                "Copy the {} selected entries into this folder.",
                browser.selected().count()
            )),
            confirm: "Duplicate".into(),
            tone: iced_fabi::ConfirmTone::Accent,
        })
    }
}

impl Column for TypeColumn {
    fn header(&self) -> String {
        "Type".into()
    }

    fn width(&self) -> ColumnWidth {
        ColumnWidth::Portion(1)
    }

    fn cell(&self, _browser: &Browser, node: &Node) -> Cell {
        Cell::Text {
            text: match node.kind {
                EntryKind::Folder => "Folder".into(),
                EntryKind::File => match extension(&node.name) {
                    Some(extension) => extension.to_uppercase(),
                    None => "File".into(),
                },
            },
            muted: true,
        }
    }

    fn compare(&self, a: &Node, b: &Node) -> std::cmp::Ordering {
        let key = |node: &Node| match node.kind {
            EntryKind::Folder => String::new(),
            EntryKind::File => extension(&node.name).unwrap_or("").to_lowercase(),
        };
        key(a).cmp(&key(b))
    }
}

fn extension(name: &str) -> Option<&str> {
    let (stem, extension) = name.rsplit_once('.')?;
    (!stem.is_empty() && extension.len() <= 5).then_some(extension)
}

/// Recursively snapshot an entry with its subtree out of the fake cloud; a real backend would fetch these listings on demand instead.
/// The fake cloud's key for the folder `name` inside the folder keyed `parent`: the same join the reads use (`path_below_root(..).join("/")`), so the root's empty parent key never becomes a leading slash.
fn cloud_key(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

fn snapshot(cloud: &HashMap<String, Vec<Entry>>, parent: &str, entry: &Entry) -> Copied {
    let children = if matches!(entry.kind, EntryKind::Folder) {
        let path = cloud_key(parent, &entry.name);
        cloud
            .get(&path)
            .map(|listing| {
                listing
                    .iter()
                    .map(|child| snapshot(cloud, &path, child))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    Copied {
        entry: entry.clone(),
        children,
    }
}

/// Re-home a copied subtree in the fake cloud: the folder's listing at its new path, the children's at theirs, and so on down.
fn write_copy(cloud: &mut HashMap<String, Vec<Entry>>, parent: &str, copied: &Copied) {
    let path = cloud_key(parent, &copied.entry.name);
    let listing = copied
        .children
        .iter()
        .map(|child| child.entry.clone())
        .collect();
    cloud.insert(path.clone(), listing);
    for child in &copied.children {
        write_copy(cloud, &path, child);
    }
}

/// Drop the listing at `path` and every listing below it (a delete in a real backend takes the whole subtree with the entry), and — like a real backend — remove the entry itself from its parent's listing, so the refresh after the delete does not bring it straight back.
fn delete_paths(cloud: &mut HashMap<String, Vec<Entry>>, path: &str) {
    let prefix = format!("{path}/");
    let below: Vec<String> = cloud
        .keys()
        .filter(|key| key.starts_with(&prefix))
        .cloned()
        .collect();
    for key in below {
        cloud.remove(&key);
    }
    match path.rsplit_once('/') {
        // A path without a slash sits in the root scope (whose key is ""); the root scope itself stays (the browser refuses to delete it), only its subtree keys go.
        None => {
            if let Some(listing) = cloud.get_mut("") {
                listing.retain(|entry| entry.name != path);
            }
        }
        Some((parent, name)) => {
            if let Some(listing) = cloud.get_mut(parent) {
                listing.retain(|entry| entry.name != name);
            }
        }
    }
}

/// Seconds since the Unix epoch, for the `modified` stamp of entries the app creates.
fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}

fn unique_copy_name(name: &str, listing: &[Entry]) -> String {
    let (stem, extension) = match extension(name) {
        Some(extension) => (
            &name[..name.len() - extension.len() - 1],
            Some(name[name.len() - extension.len()..].to_string()),
        ),
        None => (name, None),
    };
    let mut candidate = format!("{stem} copy");
    let mut counter = 2;
    while listing
        .iter()
        .any(|entry| entry.name == with_extension(&candidate, extension.as_deref()))
    {
        candidate = format!("{stem} copy {counter}");
        counter += 1;
    }
    with_extension(&candidate, extension.as_deref())
}

fn with_extension(stem: &str, extension: Option<&str>) -> String {
    match extension {
        Some(extension) => format!("{stem}.{extension}"),
        None => stem.to_string(),
    }
}

/// The fake drive: three levels deep, with the folder/file names and sizes modelled on Material Files' screenshots; the Projekte folder adds the file kinds beyond that model to show more of the Material icon set.
fn fake_cloud() -> HashMap<String, Vec<Entry>> {
    let file =
        |name: &str, size: u64, modified: i64| Entry::file(name).size(size).modified(modified);
    let folder = |name: &str, modified: i64| Entry::folder(name).modified(modified);
    let day: i64 = 86_400;
    let base = 1_745_000_000; // spring 2025.
    HashMap::from([
        (
            "".into(),
            vec![
                folder("Finanzen", base),
                folder("Dokumente", base + day),
                folder("Projekte", base + 2 * day),
                file("readme.md", 512, base - day),
            ],
        ),
        (
            "Projekte".into(),
            vec![
                file("main.rs", 4_096, base),
                file("main.go", 2_048, base + 2 * day),
                file("App.java", 3_072, base + day),
                file("install.sh", 640, base - 2 * day),
                file("Cargo.toml", 820, base),
                file("Cargo.lock", 18_432, base),
                file("flake.nix", 1_536, base - day),
                file("config.yaml", 512, base + 6 * day),
                file("feed.xml", 2_048, base + 7 * day),
                file("Folien.pptx", 1_048_576, base + 2 * day),
                file("Notizen.txt", 256, base - day),
                file("Backup 2025.7z", 45_875_200, base - 5 * day),
            ],
        ),
        (
            "Finanzen".into(),
            vec![
                folder("Krypto", base + 3 * day),
                file("Steuer 2024.pdf", 1_204_000, base),
                file("Vertragsdokument.docx", 13_670, base + 4 * day),
            ],
        ),
        (
            "Finanzen/Krypto".into(),
            vec![
                file(
                    "Risikoinformationen_BWWB_Digital_Exchange.pdf",
                    372_820,
                    1_617_817_500,
                ),
                file("ABBA.pdf", 58_230, base + 5 * day),
                file("HODL.pdf", 58_230, base + 5 * day),
                file("KEYS.pdf", 58_230, base + 5 * day),
                file("Trading.xlsx", 8_890, base + 5 * day),
                file("Rebalancing.xlsx", 40_320, base + 8 * day),
                file("Krypto-Strategie.docx", 13_670, base + 8 * day),
                file("2023-12-16-solaris-document.pdf", 69_760, base + 9 * day),
            ],
        ),
        (
            "Dokumente".into(),
            vec![
                folder("Verträge", base + day),
                file("Scheck.pdf", 90_120, base + day),
            ],
        ),
        (
            "Dokumente/Verträge".into(),
            vec![
                file("Miete.pdf", 240_000, base - 30 * day),
                file("Strom.pdf", 180_500, base - 12 * day),
            ],
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo's own listing keys follow `path_below_root(..).join("/")`, so the root scope's key is `""`: a folder pasted into the root must get its subtree under `"name copy"`, without a leading slash — that is the key the browser later loads it under.
    #[test]
    fn a_folder_copied_into_the_root_keeps_its_subtree() {
        let mut cloud = fake_cloud();
        let root = cloud.get("").cloned().unwrap_or_default();
        let source = root
            .iter()
            .find(|entry| entry.name == "Finanzen")
            .cloned()
            .expect("the fake cloud's root lists Finanzen");
        let mut top = snapshot(&cloud, "", &source);
        assert!(
            !top.children.is_empty(),
            "the snapshot itself carries the subtree"
        );
        // The rename and write exactly as `put_copies` does them when pasting into the root.
        top.entry.name = unique_copy_name(&top.entry.name, &root);
        write_copy(&mut cloud, "", &top);
        let names = |key: &str| {
            cloud.get(key).map(|listing| {
                listing
                    .iter()
                    .map(|entry| entry.name.clone())
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(
            names("Finanzen copy"),
            names("Finanzen"),
            "opening the copy lists what the source lists"
        );
        assert!(
            names("Finanzen copy/Krypto").is_some_and(|listing| !listing.is_empty()),
            "nested folders keep their listings one level below the copy"
        );
        assert!(
            !cloud.keys().any(|key| key.starts_with('/')),
            "the root's empty parent key must never become a leading slash"
        );
    }
}
