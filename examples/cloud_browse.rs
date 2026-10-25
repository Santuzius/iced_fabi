//! A cloud directory browser without the cloud: the listings come from a fake backend that answers every request one tick late, so the `activate → NeedsLoad → set_children` round-trip and its loading states behave like a real network drive.
//!
//! The bar above the cabinet demos the configuration hooks with dropdowns: the [`iced::Theme`] (including a reproduction of Syncative's palette and [`iced_fabi::Style`]), the file/folder [`iced_fabi::RowIcons`] (Tabler, or the vendored multicolour Material Icon Theme), the [`iced_fabi::Separators`] grid, the zebra stripe behind every second row, the checkbox column, which entries are listed (all or folders only — file kinds are filtered code-side at init), whether the selection holds one entry or many, and how the boxes can be summoned at all (always, on long press, or never), and where the selection's action button sits (under the list, floating, or last in the toolbar); also what happens to toolbar tools that are not currently applicable (greyed out, or hidden outright). The toolbar itself shows the [`iced_fabi::Tool`] API (copy, paste, delete — in two groups divided by [`iced_fabi::Cabinet::tool_separator`], enabled per state, and copy/paste carries whole folder subtrees); selecting is done purely through the checkboxes. The column list gets one custom [`TypeColumn`].
//!
//! Run: `cargo run --example cloud_browse` — then switch modes with the icon in the top right, tap folders to drill down, tap the boxes to select (press and hold a row first when "Ticking" is on *hold*), and watch the parent markers turn into dots.

use std::{collections::HashMap, fmt, sync::LazyLock, time::Duration};

use iced::{
    Element, Length, Task, Theme,
    widget::{column, container},
};
use iced_fabi::{Browser, Cell, Column, ColumnWidth, Entry, EntryKind, Event, Node, Outcome, tool};

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
    Copy,
    Paste,
    Delete,
    Theme(ThemeChoice),
    Icons(IconChoice),
    Inactive(InactiveChoice),
    Lines(iced_fabi::Separators),
    Zebra(ZebraChoice),
    Checkboxes(CheckboxChoice),
    Entries(EntryChoice),
    Selection(SelectionChoice),
    Ticking(TickingChoice),
    Action(ActionChoice),
    Tick,
}

// --- the demo's configuration dropdowns ---

/// Which [`Theme`] the window and the cabinet draw with; the Syncative entries mirror `syncative/src/theme.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemeChoice {
    DefaultLight,
    DefaultDark,
    SyncativeLight,
    SyncativeDark,
}

impl fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DefaultLight => "Default light",
            Self::DefaultDark => "Default dark",
            Self::SyncativeLight => "Syncative light",
            Self::SyncativeDark => "Syncative dark",
        })
    }
}

const THEMES: &[ThemeChoice] = &[
    ThemeChoice::DefaultLight,
    ThemeChoice::DefaultDark,
    ThemeChoice::SyncativeLight,
    ThemeChoice::SyncativeDark,
];

/// Which icon pack labels the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IconChoice {
    Tabler,
    MaterialIconTheme,
}

impl fmt::Display for IconChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Tabler => "Tabler",
            Self::MaterialIconTheme => "Material Icon Theme",
        })
    }
}

const ICON_CHOICES: &[IconChoice] = &[IconChoice::Tabler, IconChoice::MaterialIconTheme];

/// How tools that are currently not applicable present themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InactiveChoice {
    /// Stay in the toolbar, muted and unclickeable.
    GreyedOut,
    /// Leave the toolbar until they apply.
    Hidden,
}

impl fmt::Display for InactiveChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GreyedOut => "greyed out",
            Self::Hidden => "hidden",
        })
    }
}

const INACTIVE_CHOICES: &[InactiveChoice] = &[InactiveChoice::GreyedOut, InactiveChoice::Hidden];

/// Whether the cabinet shows its selection-box column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckboxChoice {
    On,
    Off,
}

impl fmt::Display for CheckboxChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::On => "on",
            Self::Off => "off",
        })
    }
}

const CHECKBOX_CHOICES: &[CheckboxChoice] = &[CheckboxChoice::On, CheckboxChoice::Off];

/// Which kinds of entries the lists show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryChoice {
    All,
    FoldersOnly,
}

impl fmt::Display for EntryChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::All => "files & folders",
            Self::FoldersOnly => "folders only",
        })
    }
}

const ENTRY_CHOICES: &[EntryChoice] = &[EntryChoice::All, EntryChoice::FoldersOnly];

/// How many entries the selection can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionChoice {
    Multiple,
    Single,
}

impl fmt::Display for SelectionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Multiple => "multiple",
            Self::Single => "single",
        })
    }
}

const SELECTION_CHOICES: &[SelectionChoice] = &[SelectionChoice::Multiple, SelectionChoice::Single];

/// Whether and how the widget lets you tick entries at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TickingChoice {
    Never,
    Hold,
    Tap,
}

impl fmt::Display for TickingChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Never => "never",
            Self::Hold => "hold",
            Self::Tap => "tap",
        })
    }
}

const TICKING_CHOICES: &[TickingChoice] = &[
    TickingChoice::Never,
    TickingChoice::Hold,
    TickingChoice::Tap,
];

/// Where the action button triggered by a selection sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionChoice {
    Wide,
    Floating,
    Toolbar,
}

impl fmt::Display for ActionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Wide => "under list",
            Self::Floating => "floating",
            Self::Toolbar => "toolbar",
        })
    }
}

const ACTION_CHOICES: &[ActionChoice] = &[
    ActionChoice::Wide,
    ActionChoice::Floating,
    ActionChoice::Toolbar,
];

const LINES: &[iced_fabi::Separators] = &[
    iced_fabi::Separators::None,
    iced_fabi::Separators::Horizontal,
    iced_fabi::Separators::Vertical,
    iced_fabi::Separators::Both,
];

/// How strongly every second row stands off from the background ([`iced_fabi::Style::stripe`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZebraChoice {
    Off,
    Subtle,
    Strong,
}

impl ZebraChoice {
    fn stripe(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Subtle => 0.12,
            Self::Strong => 0.35,
        }
    }
}

impl fmt::Display for ZebraChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Off => "off",
            Self::Subtle => "subtle",
            Self::Strong => "strong",
        })
    }
}

const ZEBRA: &[ZebraChoice] = &[ZebraChoice::Off, ZebraChoice::Subtle, ZebraChoice::Strong];

const fn hex(hex: u32) -> iced::Color {
    iced::Color::from_rgb(
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    )
}

/// The colours Syncative's widgets draw with: a cut of the `Tones` in `syncative/src/theme.rs` (bare colour values carry no copyright).
struct Tones {
    background: iced::Color,
    nav: iced::Color,
    surface: iced::Color,
    surface_hover: iced::Color,
    border: iced::Color,
    text: iced::Color,
    muted: iced::Color,
    accent: iced::Color,
    accent_hover: iced::Color,
    on_accent: iced::Color,
    success: iced::Color,
    warning: iced::Color,
    danger: iced::Color,
    selection: iced::Color,
    /// Not in Syncative's `Tones`: the blue-grey of the folder glyphs in the cabinet.
    folder: iced::Color,
}

const SYNCATIVE_TONES_LIGHT: Tones = Tones {
    background: hex(0xf3f3f4),
    nav: hex(0xebebed),
    surface: hex(0xffffff),
    surface_hover: hex(0xf5f6f7),
    border: hex(0xdcdde0),
    text: hex(0x1c1c1f),
    muted: hex(0x606368),
    accent: hex(0x2a73c5),
    accent_hover: hex(0x2464ad),
    on_accent: hex(0xffffff),
    success: hex(0x2a8a4f),
    warning: hex(0xb7790f),
    danger: hex(0xc4372f),
    selection: hex(0xdde7f4),
    folder: hex(0x5b7bb5),
};

const SYNCATIVE_TONES_DARK: Tones = Tones {
    background: hex(0x1f2023),
    nav: hex(0x18191b),
    surface: hex(0x2a2b2f),
    surface_hover: hex(0x323337),
    border: hex(0x3a3b40),
    text: hex(0xe6e6e8),
    muted: hex(0xa0a2a8),
    accent: hex(0x4c9be8),
    accent_hover: hex(0x63a9ee),
    on_accent: hex(0x0e1520),
    success: hex(0x4cbb73),
    warning: hex(0xe3a93a),
    danger: hex(0xe5584f),
    selection: hex(0x2c3b4f),
    folder: hex(0x6a7db0),
};

/// Syncative's corner radius of cards, buttons and pick lists.
const SYNCATIVE_RADIUS: f32 = 6.0;
/// Syncative's body text size; it sets it on every widget, so the cabinet gets it through [`iced_fabi::Cabinet::text_size`].
const SYNCATIVE_TEXT: f32 = 14.0;
/// Syncative's UI font on the desktop (found through fontconfig).
const SYNCATIVE_FONT: iced::Font = iced::Font::with_name("Noto Sans");

fn syncative_tones(theme: &Theme) -> &'static Tones {
    if theme.extended_palette().is_dark {
        &SYNCATIVE_TONES_DARK
    } else {
        &SYNCATIVE_TONES_LIGHT
    }
}

fn syncative_palette(t: &Tones) -> iced::theme::Palette {
    iced::theme::Palette {
        background: t.background,
        text: t.text,
        primary: t.accent,
        success: t.success,
        warning: t.warning,
        danger: t.danger,
    }
}

static SYNCATIVE_LIGHT: LazyLock<Theme> =
    LazyLock::new(|| Theme::custom("Syncative Light", syncative_palette(&SYNCATIVE_TONES_LIGHT)));
static SYNCATIVE_DARK: LazyLock<Theme> =
    LazyLock::new(|| Theme::custom("Syncative Dark", syncative_palette(&SYNCATIVE_TONES_DARK)));

/// The cabinet colours Syncative's own widgets use, showing what [`iced_fabi::Cabinet::style`] is for: the palette alone would derive `muted` and the row tint differently, and the cabinet sits on a card, not on the theme background. Hover and press backgrounds are those of Syncative's flat buttons.
fn syncative_style(theme: &Theme) -> iced_fabi::Style {
    let t = syncative_tones(theme);
    iced_fabi::Style {
        text: t.text,
        muted: t.muted,
        accent: t.accent,
        on_accent: t.on_accent,
        accent_hover: t.accent_hover,
        selection: t.selection,
        folder: t.folder,
        row_hover: iced::Color {
            a: 0.5,
            ..t.selection
        },
        header: t.nav,
        line: t.border,
        stripe: 0.0,
        background: t.surface,
        hover: iced::Color { a: 0.08, ..t.text },
        pressed: iced::Color { a: 0.14, ..t.text },
        radius: SYNCATIVE_RADIUS,
    }
}

/// Syncative's page header (`theme::header_bar`).
fn syncative_header_bar(theme: &Theme) -> iced::widget::container::Style {
    let t = syncative_tones(theme);
    iced::widget::container::Style {
        background: Some(t.nav.into()),
        text_color: Some(t.text),
        ..Default::default()
    }
}

/// Syncative's page background (`theme::page`).
fn syncative_page(theme: &Theme) -> iced::widget::container::Style {
    let t = syncative_tones(theme);
    iced::widget::container::Style {
        background: Some(t.background.into()),
        text_color: Some(t.text),
        ..Default::default()
    }
}

/// Syncative's raised card (`theme::card`).
fn syncative_card(theme: &Theme) -> iced::widget::container::Style {
    let t = syncative_tones(theme);
    iced::widget::container::Style {
        background: Some(t.surface.into()),
        text_color: Some(t.text),
        border: iced::Border {
            width: 1.0,
            radius: SYNCATIVE_RADIUS.into(),
            color: t.border,
        },
        ..Default::default()
    }
}

/// Syncative's hairline under the header (`theme::separator`).
fn syncative_separator(theme: &Theme) -> iced::widget::rule::Style {
    iced::widget::rule::Style {
        color: syncative_tones(theme).border,
        radius: 0.0.into(),
        fill_mode: iced::widget::rule::FillMode::Full,
        snap: true,
    }
}

/// Syncative's pick list (`theme::pick_list`).
fn syncative_pick_list(
    theme: &Theme,
    status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    use iced::widget::pick_list::Status;
    let t = syncative_tones(theme);
    iced::widget::pick_list::Style {
        text_color: t.text,
        placeholder_color: t.muted,
        handle_color: t.muted,
        background: match status {
            Status::Hovered => t.surface_hover.into(),
            _ => t.surface.into(),
        },
        border: iced::Border {
            width: 1.0,
            radius: SYNCATIVE_RADIUS.into(),
            color: if matches!(status, Status::Opened { .. }) {
                t.accent
            } else {
                t.border
            },
        },
    }
}

/// Syncative's drop-down menu (`theme::menu`).
fn syncative_menu(theme: &Theme) -> iced::overlay::menu::Style {
    let t = syncative_tones(theme);
    iced::overlay::menu::Style {
        background: t.surface.into(),
        border: iced::Border {
            width: 1.0,
            radius: SYNCATIVE_RADIUS.into(),
            color: t.border,
        },
        text_color: t.text,
        selected_text_color: t.text,
        selected_background: t.selection.into(),
        shadow: iced::Shadow {
            color: iced::Color {
                a: 0.25,
                ..iced::Color::BLACK
            },
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
    }
}

/// Syncative's slim overlay scrollbar (`theme::scrollbar`): no rail, a muted thumb that firms up on hover and drag.
fn syncative_scrollbar(
    theme: &Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    use iced::widget::scrollable::{self, Status};
    let t = syncative_tones(theme);
    let alpha = match status {
        Status::Active { .. } => 0.30,
        Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        }
        | Status::Dragged {
            is_vertical_scrollbar_dragged: true,
            ..
        } => 0.65,
        _ => 0.42,
    };
    let rail = scrollable::Rail {
        background: None,
        border: iced::Border::default(),
        scroller: scrollable::Scroller {
            background: iced::Color {
                a: alpha,
                ..t.muted
            }
            .into(),
            border: iced::border::rounded(4),
        },
    };
    scrollable::Style {
        container: iced::widget::container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        ..scrollable::default(theme, status)
    }
}

/// Geometry matching [`syncative_scrollbar`] (`theme::slim_scrollbar`).
fn syncative_scroll_direction() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new()
            .width(6)
            .scroller_width(6)
            .margin(2),
    )
}

// --- the Material Icon Theme option ---

/// Turn one vendored SVG's static file contents into icondata path data: the file's own `viewBox` and everything between the outer tags. The widget wraps that in a document with no fill or stroke of its own, so every path keeps the colours the theme authored.
fn svg_glyph(svg: &'static str) -> icondata_core::IconData {
    let view_box = match svg.find(r#"viewBox=""#) {
        Some(start) => {
            let rest = &svg[start + 9..];
            &rest[..rest.find('"').unwrap()]
        }
        None => "0 0 24 24",
    };
    let inner = &svg[svg.find('>').unwrap() + 1..svg.rfind("</svg>").unwrap()];
    icondata_core::IconData {
        style: None,
        x: None,
        y: None,
        width: None,
        height: None,
        view_box: Some(view_box),
        stroke_linecap: None,
        stroke_linejoin: None,
        stroke_width: None,
        stroke: Some("none"),
        fill: Some("none"),
        data: inner,
    }
}

/// A cut of the Material Icon Theme for VS Code (© Material Extensions, MIT — the license text sits next to the vendored files, their provenance in the root NOTICE).
static MATERIAL_ICONS: LazyLock<MatIcons> = LazyLock::new(|| MatIcons {
    folder: svg_glyph(include_str!("assets/material-icon-theme/folder.svg")),
    folder_open: svg_glyph(include_str!("assets/material-icon-theme/folder-open.svg")),
    file: svg_glyph(include_str!("assets/material-icon-theme/file.svg")),
    by_extension: HashMap::from_iter(
        [
            (
                "md",
                include_str!("assets/material-icon-theme/markdown.svg"),
            ),
            ("pdf", include_str!("assets/material-icon-theme/pdf.svg")),
            ("doc", include_str!("assets/material-icon-theme/word.svg")),
            ("docx", include_str!("assets/material-icon-theme/word.svg")),
            ("xls", include_str!("assets/material-icon-theme/table.svg")),
            ("xlsx", include_str!("assets/material-icon-theme/table.svg")),
            (
                "ppt",
                include_str!("assets/material-icon-theme/powerpoint.svg"),
            ),
            (
                "pptx",
                include_str!("assets/material-icon-theme/powerpoint.svg"),
            ),
            ("json", include_str!("assets/material-icon-theme/json.svg")),
            (
                "js",
                include_str!("assets/material-icon-theme/javascript.svg"),
            ),
            (
                "ts",
                include_str!("assets/material-icon-theme/typescript.svg"),
            ),
            ("py", include_str!("assets/material-icon-theme/python.svg")),
            ("rs", include_str!("assets/material-icon-theme/rust.svg")),
            ("css", include_str!("assets/material-icon-theme/css.svg")),
            ("html", include_str!("assets/material-icon-theme/html.svg")),
            ("zip", include_str!("assets/material-icon-theme/zip.svg")),
            ("png", include_str!("assets/material-icon-theme/image.svg")),
            ("mp4", include_str!("assets/material-icon-theme/video.svg")),
            ("mp3", include_str!("assets/material-icon-theme/audio.svg")),
            ("svg", include_str!("assets/material-icon-theme/svg.svg")),
            ("sh", include_str!("assets/material-icon-theme/console.svg")),
            ("lock", include_str!("assets/material-icon-theme/lock.svg")),
            ("toml", include_str!("assets/material-icon-theme/toml.svg")),
            ("yaml", include_str!("assets/material-icon-theme/yaml.svg")),
            ("yml", include_str!("assets/material-icon-theme/yaml.svg")),
            ("xml", include_str!("assets/material-icon-theme/xml.svg")),
            ("nix", include_str!("assets/material-icon-theme/nix.svg")),
            ("go", include_str!("assets/material-icon-theme/go.svg")),
            ("java", include_str!("assets/material-icon-theme/java.svg")),
            (
                "txt",
                include_str!("assets/material-icon-theme/document.svg"),
            ),
            ("7z", include_str!("assets/material-icon-theme/zip.svg")),
            ("rar", include_str!("assets/material-icon-theme/zip.svg")),
        ]
        .map(|(extension, svg)| (extension, svg_glyph(svg))),
    ),
});

struct MatIcons {
    folder: icondata_core::IconData,
    folder_open: icondata_core::IconData,
    file: icondata_core::IconData,
    by_extension: HashMap<&'static str, icondata_core::IconData>,
}

/// [`iced_fabi::RowIcons`] over [`MATERIAL_ICONS`], looking files up by extension; the glyphs keep their own colours.
struct MaterialIconTheme;

impl iced_fabi::RowIcons for MaterialIconTheme {
    fn folder(&self, _node: &Node, open: bool) -> icondata::Icon {
        let icons = &*MATERIAL_ICONS;
        if open {
            &icons.folder_open
        } else {
            &icons.folder
        }
    }

    fn file(&self, node: &Node) -> icondata::Icon {
        let icons = &*MATERIAL_ICONS;
        icons
            .by_extension
            .get(extension(&node.name).unwrap_or(""))
            .unwrap_or(&icons.file)
    }

    fn tinted(&self) -> bool {
        false
    }
}

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
    /// The "cloud": path → listing. The root's key is "My files".
    cloud: HashMap<String, Vec<Entry>>,
    pending: Vec<Pending>,
    clipboard: Clipboard,
    theme: ThemeChoice,
    icons: IconChoice,
    inactive: InactiveChoice,
    lines: iced_fabi::Separators,
    zebra: ZebraChoice,
    checkboxes: CheckboxChoice,
    entries: EntryChoice,
    selection: SelectionChoice,
    ticking: TickingChoice,
    action: ActionChoice,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let mut browser = Browser::new("My files");
        browser.add_column(TypeColumn);
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
            checkboxes: CheckboxChoice::On,
            entries: EntryChoice::All,
            selection: SelectionChoice::Multiple,
            ticking: TickingChoice::Tap,
            action: ActionChoice::Wide,
        };
        let task = Task::done(app.accept(outcome));
        (app, task)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Cabinet(event) => {
                let outcome = self.browser.update(event);
                Task::done(self.accept(outcome))
            }
            Message::Checkboxes(choice) => {
                self.checkboxes = choice;
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
                self.browser.set_selection_mode(match choice {
                    SelectionChoice::Multiple => iced_fabi::SelectionMode::Multiple,
                    SelectionChoice::Single => iced_fabi::SelectionMode::Single,
                });
                Task::none()
            }
            Message::Ticking(choice) => {
                self.ticking = choice;
                self.browser.set_selection_ui(match choice {
                    TickingChoice::Never => iced_fabi::SelectionUi::ViewOnly,
                    TickingChoice::Hold => iced_fabi::SelectionUi::LongPress,
                    TickingChoice::Tap => iced_fabi::SelectionUi::Select,
                });
                Task::none()
            }
            Message::Action(choice) => {
                self.action = choice;
                Task::none()
            }
            Message::Copy => {
                self.clipboard = self
                    .browser
                    .selected()
                    .filter_map(|id| {
                        let chain = self.browser.name_chain(id);
                        let (parent, name) = chain.split_at(chain.len() - 1);
                        let name = (*name.first()?).to_string();
                        let parent = parent.join("/");
                        let entry = self
                            .cloud
                            .get(&parent)?
                            .iter()
                            .find(|entry| entry.name == name)?
                            .clone();
                        Some(snapshot(&self.cloud, &parent, &entry))
                    })
                    .collect();
                // A tool that acted is done acting: the boxes go back to empty.
                self.browser.clear_selection();
                Task::none()
            }
            Message::Paste => {
                let key = self.browser.name_chain(self.browser.current()).join("/");
                let mut listing = self.cloud.get(&key).cloned().unwrap_or_default();
                for copied in self.clipboard.clone() {
                    let mut top = copied;
                    top.entry.name = unique_copy_name(&top.entry.name, &listing);
                    listing.push(top.entry.clone());
                    // The subtree is re-homed at the renamed top; opening the copy later refetches these listings normally.
                    write_copy(&mut self.cloud, &key, &top);
                }
                self.cloud.insert(key.clone(), listing.clone());
                self.browser.set_children(self.browser.current(), listing);
                self.browser.clear_selection();
                Task::none()
            }
            Message::Delete => {
                // Drop the selected entries from the fake cloud (with their subtrees) and from the browser; a real backend would get the delete call instead.
                for id in self.browser.selected().collect::<Vec<_>>() {
                    let chain = self.browser.name_chain(id);
                    let path = chain.join("/");
                    delete_paths(&mut self.cloud, &path);
                }
                self.browser.remove_selected();
                let key = self.browser.name_chain(self.browser.current()).join("/");
                let listing = self.cloud.get(&key).cloned().unwrap_or_default();
                self.browser.set_children(self.browser.current(), listing);
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
                let path = self.browser.name_chain(id).join("/");
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
        }
        Message::Tick
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
                dropdown(LINES, &self.lines, Message::Lines, 120.0, syncative),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            iced::widget::row![
                label("Zebra"),
                dropdown(ZEBRA, &self.zebra, Message::Zebra, 100.0, syncative),
                label("Checkboxes"),
                dropdown(
                    CHECKBOX_CHOICES,
                    &self.checkboxes,
                    Message::Checkboxes,
                    90.0,
                    syncative
                ),
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
                label("Ticking"),
                dropdown(
                    TICKING_CHOICES,
                    &self.ticking,
                    Message::Ticking,
                    100.0,
                    syncative
                ),
                label("Action"),
                dropdown(
                    ACTION_CHOICES,
                    &self.action,
                    Message::Action,
                    120.0,
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
                (iced_fabi::default_style, self.zebra.stripe())
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
        let mut cabinet = iced_fabi::cabinet(&self.browser, &Message::Cabinet)
            .style(style)
            .icons(icons)
            .separators(self.lines)
            .checkboxes(self.checkboxes == CheckboxChoice::On)
            // The app's default_font is Noto Sans under every theme (see `main`), so the cabinet labels with it too; without it its semibold breadcrumb and "N selected" fall back to iced's default family.
            .font(SYNCATIVE_FONT);
        if syncative {
            cabinet = cabinet
                .text_size(SYNCATIVE_TEXT)
                .scrollbar(syncative_scroll_direction(), syncative_scrollbar);
        }
        // Two tool groups — clipboard and delete — divided by a thin upright line; selection itself is driven only by the checkboxes. With "hidden" inactive tools a whole group can be missing; the cabinet then drops the stray dividers itself.
        if show(has_selection) {
            cabinet = cabinet.tool(
                tool(icondata::TbCopyOutline, |_| Message::Copy).when(move |_| has_selection),
            );
        }
        if show(has_clipboard) {
            cabinet = cabinet.tool(
                tool(icondata::TbClipboardFilled, |_| Message::Paste).when(move |_| has_clipboard),
            );
        }
        cabinet = cabinet.tool_separator();
        if show(has_selection) {
            cabinet = cabinet.tool(
                tool(icondata::TbTrashOutline, |_| Message::Delete).when(move |_| has_selection),
            );
        }
        // The selection's action button: text and message are the app's, placement is picked in the bar.
        if show(has_selection) {
            cabinet = cabinet
                .action_button("Delete selected", Message::Delete)
                .action_placement(match self.action {
                    ActionChoice::Wide => iced_fabi::ActionPlacement::WideCenter,
                    ActionChoice::Floating => iced_fabi::ActionPlacement::Floating,
                    ActionChoice::Toolbar => iced_fabi::ActionPlacement::ToolbarEnd,
                });
        }
        if !syncative {
            return column![bar, cabinet.build()].padding(16).spacing(8).into();
        }
        // Syncative's page: the header bar over a hairline, the content on a card on the page background (paddings from `syncative/src/screens/remote_page.rs`).
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
            .padding([4, 0])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(syncative_card);
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

/// A custom column: the file extension in capitals, folders as "Folder"; sorted by kind first, then extension.
#[derive(Debug)]
struct TypeColumn;

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
fn snapshot(cloud: &HashMap<String, Vec<Entry>>, parent: &str, entry: &Entry) -> Copied {
    let children = if matches!(entry.kind, EntryKind::Folder) {
        let path = format!("{parent}/{}", entry.name);
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
    let path = format!("{parent}/{}", copied.entry.name);
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
        // A top-level path is the root scope: it itself stays (the browser refuses to delete it), only its subtree keys go.
        None => {}
        Some((parent, name)) => {
            if let Some(listing) = cloud.get_mut(parent) {
                listing.retain(|entry| entry.name != name);
            }
        }
    }
}

/// `report.pdf` → `report copy.pdf`, `report copy.pdf` → `report copy 2.pdf`, …
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
    let file = |name: &str, size: u64, modified: i64| Entry {
        name: name.into(),
        kind: EntryKind::File,
        size: Some(size),
        modified: Some(modified),
    };
    let folder = |name: &str, modified: i64| Entry {
        name: name.into(),
        kind: EntryKind::Folder,
        size: None,
        modified: Some(modified),
    };
    let day: i64 = 86_400;
    let base = 1_745_000_000; // spring 2025.
    HashMap::from([
        (
            "My files".into(),
            vec![
                folder("Finanzen", base),
                folder("Dokumente", base + day),
                folder("Projekte", base + 2 * day),
                file("readme.md", 512, base - day),
            ],
        ),
        (
            "My files/Projekte".into(),
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
            "My files/Finanzen".into(),
            vec![
                folder("Krypto", base + 3 * day),
                file("Steuer 2024.pdf", 1_204_000, base),
                file("Vertragsdokument.docx", 13_670, base + 4 * day),
            ],
        ),
        (
            "My files/Finanzen/Krypto".into(),
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
            "My files/Dokumente".into(),
            vec![
                folder("Verträge", base + day),
                file("Scheck.pdf", 90_120, base + day),
            ],
        ),
        (
            "My files/Dokumente/Verträge".into(),
            vec![
                file("Miete.pdf", 240_000, base - 30 * day),
                file("Strom.pdf", 180_500, base - 12 * day),
            ],
        ),
    ])
}
