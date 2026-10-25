//! iced File cABInet — a widget for browsing directories and selecting items.
//!
//! Two views over one state ([`Browser`]):
//!
//! - [`Mode::Browse`]: one directory per screen with a breadcrumb bar, Android Documents-App style.
//! - [`Mode::Tree`]: the whole tree in place, every branch expandable — selection can span several levels at once.
//!
//! Material-style selection on every row: a box, a tick when the item is selected as a unit, and a dot when something inside it is selected ([`Marker`]); the header's box selects or clears the whole list. The listings themselves come from the application: the widget only says which nodes need loading ([`Outcome::NeedsLoad`]), the app answers with [`Browser::set_children`] — from disk, rclone, or an HTTP API.
//!
//! The widget is configured through the [`Cabinet`] builder: toolbar [`Tool`]s in groups ([`Cabinet::tool_separator`] between them, besides the built-in mode switch), a text [`action_button`](Cabinet::action_button) for the selection, custom [`Column`]s (registered on the [`Browser`]), a [`Style`] hook so an app's own theme (its palette, or a fork's) drives every colour, and a [`RowIcons`] hook for the file/folder glyphs (its own icon pack, tinted or multicolour).
//!
//! ```no_run
//! use iced_fabi::{Browser, Event, file_cabinet};
//!
//! #[derive(Clone)]
//! enum Message { Cabinet(Event) }
//!
//! let browser = Browser::new("My files");
//! let _element = file_cabinet::<Message, iced::Renderer>(&browser, &Message::Cabinet);
//! ```

mod browser;
mod cut;
mod icon;

pub use browser::{
    Browser, Cell, Column, ColumnWidth, Entry, EntryKind, FormatModified, FormatSize, LONG_PRESS,
    Marker, Mode, ModifiedColumn, NameColumn, Node, NodeId, Outcome, SelectionMode, SelectionUi,
    SizeColumn, format_bytes, format_epoch,
};

use std::fmt;
use std::rc::Rc;

use iced_widget::{
    Space, button, column, container,
    core::{
        Alignment, Background, Border, Color, Element, Font, Length, Pixels, Theme, border, font,
    },
    row, scrollable, text,
};

use crate::browser::Row;
use crate::cut::Cut;

/// An interaction the widget produced; map it to your message and apply it to the [`Browser`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Row clicked (browse) or caret clicked (tree): folders navigate or expand, files report [`Outcome::FileActivated`].
    Activate(NodeId),
    /// Selection marker clicked; see [`Browser::toggle`].
    Toggle(NodeId),
    /// A row's press began — only emitted in [`SelectionUi::LongPress`] while the boxes are hidden; feed it to [`Browser::hold_started`].
    HoldStarted(NodeId),
    /// A row's press ended — feed it to [`Browser::hold_ended`], which reveals the boxes and ticks the held row when the row was held long enough.
    HoldEnded(NodeId),
    /// The header's select-all box clicked; see [`Browser::toggle_all`].
    ToggleAll,
    /// Breadcrumb clicked.
    GoTo(NodeId),
    /// Sort header clicked; index into [`Browser::columns`]. See [`Browser::sort_by_column`].
    Sort(usize),
    /// List/tree switch clicked.
    ToggleMode,
}

/// A toolbar button the programmer adds with [`Cabinet::tool`].
///
/// Implement it on your own type (per-app state decides `enabled`), or build a quick one from closures with [`tool`]. Its click yields your message; what happens next (copy, paste, share, delete, …) is the application's business — the widget just renders the button and sends the message.
pub trait Tool<Message> {
    /// The toolbar glyph.
    fn glyph(&self) -> icondata::Icon;

    /// Whether the button is clickable (grey when not).
    fn enabled(&self, _browser: &Browser) -> bool {
        true
    }

    /// The message a click produces.
    fn message(&self, browser: &Browser) -> Message;
}

/// A [`Tool`] made from closures; see [`tool`].
pub struct SimpleTool<Message> {
    glyph: icondata::Icon,
    message: Box<dyn Fn(&Browser) -> Message>,
    enabled: Box<dyn Fn(&Browser) -> bool>,
}

/// Start a tool from a closure: `tool(icondata::TbCopyOutline, |_| Message::Copy)`.
pub fn tool<Message>(
    glyph: icondata::Icon,
    message: impl Fn(&Browser) -> Message + 'static,
) -> SimpleTool<Message> {
    SimpleTool {
        glyph,
        message: Box::new(message),
        enabled: Box::new(|_| true),
    }
}

impl<Message> SimpleTool<Message> {
    /// Restrict when the button is clickable: `.when(|browser| browser.has_selection())`. The closure sees the latest browser state on every frame, so it may also capture app state by value at view time.
    pub fn when(mut self, enabled: impl Fn(&Browser) -> bool + 'static) -> Self {
        self.enabled = Box::new(enabled);
        self
    }
}

impl<Message> Tool<Message> for SimpleTool<Message> {
    fn glyph(&self) -> icondata::Icon {
        self.glyph
    }

    fn enabled(&self, browser: &Browser) -> bool {
        (self.enabled)(browser)
    }

    fn message(&self, browser: &Browser) -> Message {
        (self.message)(browser)
    }
}

/// The widget's colours. Defaults from the theme's palette (see [`default_style`]); override the whole thing per theme in [`Cabinet::style`] to apply your app's design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// Primary text and icons (file kind glyphs).
    pub text: Color,
    /// Secondary text and icons (markers, carets, breadcrumb ancestors, cells).
    pub muted: Color,
    /// Selected markers and the sort column's arrow.
    pub accent: Color,
    /// Text on accent-filled surfaces (the action button's label).
    pub on_accent: Color,
    /// Hover variant of [`accent`](Self::accent) (the action button under the pointer).
    pub accent_hover: Color,
    /// Tint of the folder glyphs of tinted icon packs.
    pub folder: Color,
    /// Row background while selected.
    pub selection: Color,
    /// Row background under the pointer; keep it weaker than [`selection`](Self::selection).
    pub row_hover: Color,
    /// Background of the column header row.
    pub header: Color,
    /// Separator lines: under the toolbar and the header, and the optional row and column [`Separators`].
    pub line: Color,
    /// Zebra striping: every second row's background shifts this much (0 = off, 1 = a full step of grey) away from [`background`](Self::background) — darker on dark backgrounds, lighter on light ones.
    pub stripe: f32,
    /// What the cabinet sits on, the base of the zebra stripe. The theme background by default, your card or panel colour when the cabinet is placed on one.
    pub background: Color,
    /// Background of the flat buttons (breadcrumbs, tools, markers, carets) under the pointer.
    pub hover: Color,
    /// Background of a flat button while it is pressed.
    pub pressed: Color,
    /// Corner radius of the hover and press backgrounds.
    pub radius: f32,
}

/// `a` moved towards `b` by `t` (0 = `a`, 1 = `b`), opaque.
fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    }
}

/// [`Style`] from the theme's palette.
pub fn default_style(theme: &Theme) -> Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let selection = extended.primary.weak.color;
    Style {
        text: palette.text,
        muted: Color {
            a: 0.6,
            ..palette.text
        },
        accent: palette.primary,
        on_accent: if 0.299 * palette.primary.r
            + 0.587 * palette.primary.g
            + 0.114 * palette.primary.b
            < 0.5
        {
            Color::WHITE
        } else {
            Color::BLACK
        },
        accent_hover: mix(palette.primary, palette.text, 0.12),
        folder: mix(palette.primary, palette.background, 0.3),
        selection,
        row_hover: Color {
            a: 0.35,
            ..selection
        },
        header: mix(palette.background, Color::BLACK, 0.06),
        line: Color {
            a: 0.15,
            ..palette.text
        },
        stripe: 0.0,
        background: palette.background,
        hover: Color {
            a: 0.08,
            ..palette.text
        },
        pressed: Color {
            a: 0.14,
            ..palette.text
        },
        radius: 4.0,
    }
}

/// Which separator lines the list draws; [`Cabinet::separators`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Separators {
    /// Plain rows, no lines.
    #[default]
    None,
    /// A line between each item and below the last.
    Horizontal,
    /// A line between each column.
    Vertical,
    /// A grid.
    Both,
}

impl Separators {
    pub fn horizontal(&self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }

    pub fn vertical(&self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }
}

impl fmt::Display for Separators {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Both => "both",
        })
    }
}

/// Chooses the file and folder glyphs shown on the rows; see [`Cabinet::icons`].
///
/// The built-in default is [`TablerIcons`]. An app with its own icon theme implements this and hands it to the cabinet; the demo wires the same up to a dropdown.
pub trait RowIcons {
    /// The folder row's glyph; `open` when its branch is expanded (tree mode).
    fn folder(&self, node: &Node, open: bool) -> icondata::Icon;

    /// The file row's glyph for this entry — pick by extension or MIME, whatever your theme keys on.
    fn file(&self, node: &Node) -> icondata::Icon;

    /// Whether glyphs are repainted in the widget's colours ([`Style::folder`], [`Style::text`]). `true` suits monochrome packs (Tabler); `false` keeps a multicolour pack's own fills (Material Icon Theme).
    fn tinted(&self) -> bool {
        true
    }
}

/// The built-in [`RowIcons`]: Tabler folder / open-folder / file, tinted like the rest of the widget.
pub struct TablerIcons;

impl RowIcons for TablerIcons {
    fn folder(&self, _node: &Node, open: bool) -> icondata::Icon {
        if open {
            icon::FOLDER_OPEN
        } else {
            icon::FOLDER
        }
    }

    fn file(&self, _node: &Node) -> icondata::Icon {
        icon::FILE
    }
}

// --- geometry: one template for the header and every row, so the columns line up by construction ---

/// Height of the toolbar.
const BAR_HEIGHT: f32 = 44.0;
/// Height of the column header and of every list row.
const ROW_HEIGHT: f32 = 36.0;
/// Inset of the first and last cell from the cabinet's edges.
const EDGE: f32 = 8.0;
/// The selection box column at the left edge.
const MARKER_CELL: f32 = 36.0;
/// Horizontal padding inside every column cell, header and rows alike.
const CELL_PAD: f32 = 8.0;
/// The tree caret's slot in front of the row glyph.
const CARET: f32 = 20.0;
/// Tree indent per level.
const INDENT: f32 = 20.0;
/// The gap between caret, glyph and name inside the Name cell.
const GAP: f32 = 8.0;
/// Size of the file and folder glyphs.
const GLYPH: f32 = 18.0;

enum ToolbarItem<'a, Message> {
    Tool(Box<dyn Tool<Message> + 'a>),
    Separator,
}

/// Which [`Style`] colour a piece of chrome wants.
#[derive(Clone, Copy)]
enum ColourField {
    Text,
    Muted,
    Accent,
    Folder,
    Line,
}

impl ColourField {
    fn get(self, style: Style) -> Color {
        match self {
            ColourField::Text => style.text,
            ColourField::Muted => style.muted,
            ColourField::Accent => style.accent,
            ColourField::Folder => style.folder,
            ColourField::Line => style.line,
        }
    }
}

/// A colour resolved per frame from the theme.
type Colour<'a> = Rc<dyn Fn(&Theme) -> Color + 'a>;

/// A scrollbar style function as [`Cabinet::scrollbar`] stores it.
type ScrollbarStyle<'a> = Rc<dyn Fn(&Theme, scrollable::Status) -> scrollable::Style + 'a>;

/// Where the [`Cabinet::action_button`] goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionPlacement {
    /// Wide and centred below the row list.
    #[default]
    WideCenter,
    /// Floating over the rows in the cabinet's bottom-right corner.
    Floating,
    /// As the last item of the toolbar, before the mode switch.
    ToolbarEnd,
}

/// The cabinet's one action button; see [`Cabinet::action_button`].
struct Action<Message> {
    text: String,
    on_press: Message,
}

/// The file cabinet as a builder: call [`cabinet`] (or [`file_cabinet`] for the defaults) and configure toolbar and colours before [`build`](Self::build)ing.
pub struct Cabinet<'a, Message: Clone, Renderer> {
    browser: &'a Browser,
    on_event: &'a dyn Fn(Event) -> Message,
    tools: Vec<ToolbarItem<'a, Message>>,
    mode_toggle: bool,
    checkboxes: bool,
    action: Option<Action<Message>>,
    action_placement: ActionPlacement,
    style: Rc<dyn Fn(&Theme) -> Style + 'a>,
    icons: &'a dyn RowIcons,
    separators: Separators,
    text_size: Option<Pixels>,
    font: Option<Font>,
    scrollbar: Option<(scrollable::Direction, ScrollbarStyle<'a>)>,
    _renderer: std::marker::PhantomData<fn() -> Renderer>,
}

/// Start building the widget over `browser`; `on_event` maps the widget's events to your messages.
pub fn cabinet<'a, Message, Renderer>(
    browser: &'a Browser,
    on_event: &'a dyn Fn(Event) -> Message,
) -> Cabinet<'a, Message, Renderer>
where
    Message: Clone + 'a,
{
    Cabinet {
        browser,
        on_event,
        tools: Vec::new(),
        mode_toggle: true,
        checkboxes: true,
        action: None,
        action_placement: ActionPlacement::default(),
        style: Rc::new(default_style),
        icons: &TablerIcons,
        separators: Separators::default(),
        text_size: None,
        font: None,
        scrollbar: None,
        _renderer: std::marker::PhantomData,
    }
}

/// The default cabinet: toolbar with breadcrumbs, sort header and the row list for the browser's current mode, with the built-in mode switch only.
pub fn file_cabinet<'a, Message, Renderer>(
    browser: &'a Browser,
    on_event: &'a dyn Fn(Event) -> Message,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    cabinet(browser, on_event).build()
}

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    /// Add a toolbar button; the buttons land on the right of the bar, divided from the mode switch.
    pub fn tool(mut self, tool: impl Tool<Message> + 'a) -> Self {
        self.tools.push(ToolbarItem::Tool(Box::new(tool)));
        self
    }

    /// A divider between the toolbar buttons added before and after it, to group them. A divider that would sit beside a group whose tools were all left out is itself left out when the toolbar is built, so groups can be added conditionally.
    pub fn tool_separator(mut self) -> Self {
        self.tools.push(ToolbarItem::Separator);
        self
    }

    /// Show or hide the built-in browse/tree switch button; it sits by the breadcrumbs, a divider from the tools. Default: on.
    pub fn show_mode_toggle(mut self, yes: bool) -> Self {
        self.mode_toggle = yes;
        self
    }

    /// Show or hide the selection-box column (the header box and the box on every row). With it off the rows keep the space where the boxes sat, so they still align with the columns; selection state is only visible through row backgrounds and the "N selected" count. Default: on.
    pub fn checkboxes(mut self, yes: bool) -> Self {
        self.checkboxes = yes;
        self
    }

    /// The cabinet's one text button for the app's main action (download, share, delete, …): it is clickable while at least one entry is selected and otherwise grey like an inactive tool; `on_press` is sent on a click and what follows is the application's business. It is a pill with completely round corners; where it sits: see [`action_placement`](Self::action_placement).
    pub fn action_button(mut self, text: impl Into<String>, on_press: Message) -> Self {
        self.action = Some(Action {
            text: text.into(),
            on_press,
        });
        self
    }

    /// Where the [`action_button`](Self::action_button) goes; ignored without one. Default: [`ActionPlacement::WideCenter`].
    pub fn action_placement(mut self, placement: ActionPlacement) -> Self {
        self.action_placement = placement;
        self
    }

    /// Colours for this cabinet, resolved per frame from the current theme — pass a function that maps the theme (including a fork's custom palettes) to your [`Style`].
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self {
        self.style = Rc::new(style);
        self
    }

    /// Where the row file/folder glyphs come from; see [`RowIcons`]. Default: [`TablerIcons`].
    pub fn icons(mut self, icons: &'a dyn RowIcons) -> Self {
        self.icons = icons;
        self
    }

    /// Separator lines between items and columns. Default: none.
    pub fn separators(mut self, separators: Separators) -> Self {
        self.separators = separators;
        self
    }

    /// Size of every label in the cabinet. Default: the renderer's default text size (the application's `default_text_size`).
    pub fn text_size(mut self, size: impl Into<Pixels>) -> Self {
        self.text_size = Some(size.into());
        self
    }

    /// Font of every label in the cabinet; the current breadcrumb and the selection count use its semibold weight. Pass your application's `default_font`, otherwise those two fall back to iced's default family. Default: the renderer's default font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Scrollbar geometry and style of the row list, e.g. your app's slim overlay scrollbar. Default: iced's.
    pub fn scrollbar(
        mut self,
        direction: scrollable::Direction,
        style: impl Fn(&Theme, scrollable::Status) -> scrollable::Style + 'a,
    ) -> Self {
        self.scrollbar = Some((direction, Rc::new(style)));
        self
    }

    pub fn build(self) -> Element<'a, Message, Theme, Renderer> {
        let list = match self.browser.mode() {
            Mode::Browse => self.browse_list(),
            Mode::Tree => self.tree_list(),
        };
        let mut list = scrollable(list).height(Length::Fill);
        if let Some((direction, style)) = &self.scrollbar {
            let style = Rc::clone(style);
            list = list
                .direction(*direction)
                .style(move |theme, status| style(theme, status));
        }
        // The floating action button hovers over the row list: stacked onto the list itself, its padding measures equal distances from the list's right and bottom edges.
        let list: Element<'a, Message, Theme, Renderer> = match &self.action {
            Some(action) if self.action_placement == ActionPlacement::Floating => {
                iced_widget::stack![
                    list,
                    container(self.action_button_element(action, ActionPlacement::Floating))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Alignment::End)
                        .align_y(Alignment::End)
                        .padding(16)
                ]
                .into()
            }
            _ => list.into(),
        };
        let mut body = column![
            self.toolbar(),
            self.hrule(),
            self.column_header(),
            self.hrule(),
            list
        ];
        if let Some(action) = self
            .action
            .as_ref()
            .filter(|_| self.action_placement == ActionPlacement::WideCenter)
        {
            body = body.push(
                container(self.action_button_element(action, ActionPlacement::WideCenter))
                    .width(Length::Fill)
                    .padding([8.0, EDGE]),
            );
        }
        body.into()
    }

    // --- styling helpers ---

    fn colour(&self, field: ColourField) -> Colour<'a> {
        let style = Rc::clone(&self.style);
        Rc::new(move |theme: &Theme| field.get(style(theme)))
    }

    fn glyph(&self, glyph: icondata::Icon, size: f32, field: ColourField) -> iced_widget::Svg<'a> {
        icon::glyph(glyph, size, self.colour(field))
    }

    /// A label at the cabinet's text size and font.
    fn label(&self, content: impl text::IntoFragment<'a>) -> text::Text<'a, Theme, Renderer> {
        let mut label = text(content);
        if let Some(size) = self.text_size {
            label = label.size(size);
        }
        if let Some(font) = self.font {
            label = label.font(font);
        }
        label
    }

    /// A label in the semibold weight of the cabinet's font.
    fn strong(&self, content: impl text::IntoFragment<'a>) -> text::Text<'a, Theme, Renderer> {
        self.label(content).font(Font {
            weight: font::Weight::Semibold,
            ..self.font.unwrap_or(Font::DEFAULT)
        })
    }

    /// A label cut with an ellipsis to the width of its cell.
    fn cut(&self, label: String, field: ColourField) -> Element<'a, Message, Theme, Renderer> {
        Cut::new(label, self.colour(field))
            .size(self.text_size)
            .font(self.font)
            .into()
    }

    /// A flat button's style: `colour` for its content, the hover and press backgrounds from the cabinet's [`Style`].
    fn flat(&self, colour: Colour<'a>) -> impl Fn(&Theme, button::Status) -> button::Style + 'a {
        let style = Rc::clone(&self.style);
        move |theme, status| {
            let appearance = style(theme);
            let background = match status {
                button::Status::Hovered => Some(appearance.hover),
                button::Status::Pressed => Some(appearance.pressed),
                button::Status::Active | button::Status::Disabled => None,
            };
            button::Style {
                background: background.map(Background::Color),
                text_color: colour(theme),
                border: border::rounded(appearance.radius),
                ..button::Style::default()
            }
        }
    }

    /// A square icon button: the glyph centred in `side` px.
    fn icon_button(
        &self,
        glyph: icondata::Icon,
        size: f32,
        side: f32,
        field: ColourField,
    ) -> iced_widget::Button<'a, Message, Theme, Renderer> {
        button(
            container(self.glyph(glyph, size, field))
                .center_x(Length::Fixed(side))
                .center_y(Length::Fixed(side)),
        )
        .padding(0)
        .style(self.flat(self.colour(field)))
    }

    /// The action button as an element, styled like Syncative's `button_primary`: an accent-filled pill with the [`on_accent`](Style::on_accent) label and a same-colour 1-px border; hover swaps in [`accent_hover`](Style::accent_hover), and pressed is the base again. The corners are completely round (a radius of half the button's height). Without a selection it sits greyed like an inactive tool: the fill, label and border at 50 %, 45 % and 50 % alpha, and no click. `placement` sizes it: full-width and centred text under the list, content-sized and compact in the toolbar.
    fn action_button_element(
        &self,
        action: &Action<Message>,
        placement: ActionPlacement,
    ) -> Element<'a, Message, Theme, Renderer> {
        let content: Element<'a, Message, Theme, Renderer> =
            if placement == ActionPlacement::WideCenter {
                container(self.label(action.text.clone()))
                    .width(Length::Fill)
                    .center_x(Length::Fill)
                    .into()
            } else {
                self.label(action.text.clone()).into()
            };
        let mut btn = button(content).padding(if placement == ActionPlacement::ToolbarEnd {
            [4.0, 6.0]
        } else {
            [8.0, 18.0]
        });
        if placement == ActionPlacement::WideCenter {
            btn = btn.width(Length::Fill);
        }
        if self.browser.has_selection() {
            btn = btn.on_press(action.on_press.clone());
        }
        let style = Rc::clone(&self.style);
        let text_size = self.text_size.unwrap_or_default();
        btn.style(move |theme, status| {
            let appearance = style(theme);
            let ghost = status == button::Status::Disabled;
            let bg = if status == button::Status::Hovered {
                appearance.accent_hover
            } else {
                appearance.accent
            };
            let fade = |c: Color, a: f32| Color { a, ..c };
            // Half the button's own height (text + vertical padding) makes the corners completely round.
            let pill = (text_size.0 + 16.0) / 2.0;
            button::Style {
                background: Some(Background::Color(fade(bg, if ghost { 0.5 } else { 1.0 }))),
                text_color: fade(appearance.on_accent, if ghost { 0.45 } else { 1.0 }),
                border: Border {
                    width: 1.0,
                    radius: pill.into(),
                    color: fade(bg, if ghost { 0.5 } else { 1.0 }),
                },
                ..button::Style::default()
            }
        })
        .into()
    }

    /// A 1-px rule in the [`Style::line`] colour, across.
    fn hrule(&self) -> Element<'a, Message, Theme, Renderer> {
        let line = self.colour(ColourField::Line);
        iced_widget::rule::horizontal(1.0)
            .style(move |theme: &Theme| iced_widget::rule::Style {
                color: line(theme),
                ..iced_widget::rule::default(theme)
            })
            .into()
    }

    /// A 1-px rule in the [`Style::line`] colour, upright.
    fn vrule(&self) -> Element<'a, Message, Theme, Renderer> {
        let line = self.colour(ColourField::Line);
        iced_widget::rule::vertical(1.0)
            .style(move |theme: &Theme| iced_widget::rule::Style {
                color: line(theme),
                ..iced_widget::rule::default(theme)
            })
            .into()
    }

    // --- chrome ---

    /// Breadcrumbs with the mode switch behind a divider on the left, the tool groups on the right.
    fn toolbar(&self) -> Element<'a, Message, Theme, Renderer> {
        let mut crumbs = row![].spacing(2).align_y(Alignment::Center);
        // Tree mode is positionless (the rows are the position), so its breadcrumb stays at the root.
        let path: &[NodeId] = match self.browser.mode() {
            Mode::Tree => &self.browser.path()[..1],
            Mode::Browse => self.browser.path(),
        };
        for (depth, id) in path.iter().enumerate() {
            if depth > 0 {
                crumbs = crumbs.push(self.glyph(icon::CHEVRON_RIGHT, 14.0, ColourField::Muted));
            }
            let name = self
                .browser
                .node(*id)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            let (label, field) = if depth + 1 == path.len() {
                (self.strong(name), ColourField::Text)
            } else {
                (self.label(name), ColourField::Muted)
            };
            crumbs = crumbs.push(
                button(label)
                    .padding([4, 6])
                    .style(self.flat(self.colour(field)))
                    .on_press((self.on_event)(Event::GoTo(*id))),
            );
        }
        let mut toolbar = row![].spacing(2).align_y(Alignment::Center);
        // The mode switch belongs by the breadcrumb, divided from the tools; no divider when the bar has no tools at all.
        // A separator only ever divides two tools: leading, doubled-up and trailing ones fall away on their own, so a group whose tools were all left out takes its divider with them.
        let mut items: Vec<&ToolbarItem<'a, Message>> = Vec::new();
        for item in &self.tools {
            match item {
                ToolbarItem::Tool(_) => items.push(item),
                ToolbarItem::Separator => {
                    if matches!(items.last(), Some(ToolbarItem::Tool(_))) {
                        items.push(item);
                    }
                }
            }
        }
        if matches!(items.last(), Some(ToolbarItem::Separator)) {
            items.pop();
        }
        for item in &items {
            toolbar = toolbar.push(match item {
                ToolbarItem::Separator => self.divider(),
                ToolbarItem::Tool(tool) => {
                    let enabled = tool.enabled(self.browser);
                    let field = if enabled {
                        ColourField::Text
                    } else {
                        ColourField::Muted
                    };
                    let button = self.icon_button(tool.glyph(), 18.0, 30.0, field);
                    if enabled {
                        button.on_press(tool.message(self.browser)).into()
                    } else {
                        button.into()
                    }
                }
            });
        }
        let has_tools = items
            .iter()
            .any(|item| matches!(item, ToolbarItem::Tool(_)));
        let mut mode_switch = row![].spacing(2).align_y(Alignment::Center);
        if self.mode_toggle {
            let mode_glyph = match self.browser.mode() {
                Mode::Browse => icon::TREE,
                Mode::Tree => icon::LIST,
            };
            mode_switch = mode_switch.push(
                self.icon_button(mode_glyph, 18.0, 30.0, ColourField::Text)
                    .on_press((self.on_event)(Event::ToggleMode)),
            );
            if has_tools {
                mode_switch = mode_switch.push(self.divider());
            }
        }
        // The toolbar variant of the action button lands after the tool groups.
        if let Some(action) = self
            .action
            .as_ref()
            .filter(|_| self.action_placement == ActionPlacement::ToolbarEnd)
        {
            if has_tools || self.mode_toggle {
                toolbar = toolbar.push(self.divider());
            }
            toolbar = toolbar.push(self.action_button_element(action, ActionPlacement::ToolbarEnd));
        }
        row![
            crumbs,
            mode_switch,
            Space::new().width(Length::Fill),
            toolbar
        ]
        .spacing(8)
        .height(Length::Fixed(BAR_HEIGHT))
        .padding([0.0, EDGE])
        .align_y(Alignment::Center)
        .into()
    }

    /// The upright line between two tool groups.
    fn divider(&self) -> Element<'a, Message, Theme, Renderer> {
        container(self.vrule())
            .height(Length::Fixed(20.0))
            .padding([0, 6])
            .into()
    }

    /// The row template: selection box (when [`Cabinet::checkboxes`] is on), the Name cell and the other columns (with optional rules between them). The header and every row go through here, so their columns always share the same boundaries.
    fn template(
        &self,
        marker: Element<'a, Message, Theme, Renderer>,
        name: Element<'a, Message, Theme, Renderer>,
        cells: Vec<Element<'a, Message, Theme, Renderer>>,
    ) -> Element<'a, Message, Theme, Renderer> {
        let columns = self.browser.columns();
        let cell = |content: Element<'a, Message, Theme, Renderer>, width: ColumnWidth| {
            container(content)
                .width(length(width))
                .height(Length::Fill)
                .padding([0.0, CELL_PAD])
                .align_y(Alignment::Center)
                .clip(true)
        };
        let mut row = iced_widget::Row::new()
            .height(Length::Fixed(ROW_HEIGHT))
            .padding([0.0, EDGE])
            .align_y(Alignment::Center);
        if self.boxes_shown() {
            row = row.push(container(marker).center_x(Length::Fixed(MARKER_CELL)));
        }
        row = row.push(cell(name, columns[0].width()));
        for (index, content) in cells.into_iter().enumerate() {
            if self.separators.vertical() {
                row = row.push(self.vrule());
            }
            row = row.push(cell(content, columns[index + 1].width()));
        }
        row.into()
    }

    /// Whether a selection box column goes on the rows and the header right now: the [`Cabinet::checkboxes`] column setting and the browser's [`SelectionUi`] (which in [`SelectionUi::LongPress`] flips on once a row has been held long enough) both have to allow it.
    fn boxes_shown(&self) -> bool {
        self.checkboxes && self.browser.boxes_visible()
    }

    /// The select-all box and the sort headers — or, while something is selected, the selection count.
    fn column_header(&self) -> Element<'a, Message, Theme, Renderer> {
        let marker = self
            .boxes_shown()
            .then(|| self.marker_button(self.browser.scope_marker(), Event::ToggleAll));
        let tree = self.browser.mode() == Mode::Tree;
        let content = if self.browser.has_selection() {
            let count = self.browser.selected().count();
            let count =
                container(self.strong(format!("{count} selected"))).padding([0.0, CELL_PAD]);
            match marker {
                Some(marker) => row![
                    container(marker).center_x(Length::Fixed(MARKER_CELL)),
                    count
                ]
                .height(Length::Fixed(ROW_HEIGHT))
                .padding([0.0, EDGE])
                .align_y(Alignment::Center)
                .into(),
                None => row![count]
                    .height(Length::Fixed(ROW_HEIGHT))
                    .padding([0.0, EDGE])
                    .align_y(Alignment::Center)
                    .into(),
            }
        } else {
            let mut headers: Vec<Element<'a, Message, Theme, Renderer>> =
                (0..self.browser.columns().len())
                    .map(|index| self.sort_button(index))
                    .collect();
            let mut name = headers.remove(0);
            if tree {
                // In tree mode the label starts where the top level's glyphs do, behind the caret slot.
                name = row![Space::new().width(Length::Fixed(CARET + GAP)), name].into();
            }
            self.template(
                marker.unwrap_or_else(|| iced_widget::Space::new().into()),
                name,
                headers,
            )
        };
        let style = Rc::clone(&self.style);
        container(content)
            .width(Length::Fill)
            .style(move |theme: &Theme| container::Style {
                background: Some(Background::Color(style(theme).header)),
                ..container::Style::default()
            })
            .into()
    }

    /// One column's header: its title, plus the direction arrow when the list is sorted by it.
    fn sort_button(&self, index: usize) -> Element<'a, Message, Theme, Renderer> {
        let (sort_column, ascending) = self.browser.sort();
        let mut title = row![self.label(self.browser.columns()[index].header())]
            .spacing(4)
            .align_y(Alignment::Center);
        if sort_column == index {
            let arrow = if ascending {
                icon::SORT_ASCENDING
            } else {
                icon::SORT_DESCENDING
            };
            title = title.push(self.glyph(arrow, 14.0, ColourField::Accent));
        }
        let colour = self.colour(ColourField::Text);
        button(
            container(title)
                .height(Length::Fill)
                .align_y(Alignment::Center),
        )
        .padding(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |theme, _| button::Style {
            text_color: colour(theme),
            ..button::Style::default()
        })
        .on_press((self.on_event)(Event::Sort(index)))
        .into()
    }

    fn marker_button(&self, marker: Marker, event: Event) -> Element<'a, Message, Theme, Renderer> {
        let (glyph, field) = match marker {
            Marker::Unselected => (icon::SQUARE_OUTLINE, ColourField::Muted),
            Marker::Selected => (icon::CHECKBOX, ColourField::Accent),
            Marker::Partial => (icon::SQUARE_DOT, ColourField::Accent),
        };
        self.icon_button(glyph, 20.0, 28.0, field)
            .on_press((self.on_event)(event))
            .into()
    }

    // --- rows ---

    fn browse_list(&self) -> Element<'a, Message, Theme, Renderer> {
        let rows = self.browser.browse_rows();
        let mut list = column![];
        for (index, row) in rows.iter().enumerate() {
            if index > 0 && self.separators.horizontal() {
                list = list.push(self.hrule());
            }
            list = list.push(self.line(row, false, index % 2 == 1));
        }
        let current = self.browser.node(self.browser.current());
        let mut status = false;
        if current.is_some_and(|n| n.loading) {
            list = list.push(self.status_line("Loading…", None, 0.0));
            status = true;
        } else if current.is_some_and(|n| n.failed) {
            list = list.push(self.status_line(
                "Could not load — tap to retry",
                Some(self.browser.current()),
                0.0,
            ));
            status = true;
        } else if current.is_some_and(|n| n.children_known && n.children.is_empty()) {
            list = list.push(self.status_line("Empty", None, 0.0));
            status = true;
        }
        // The last entry gets a line below it too, the same colour as the ones between the rows.
        if self.separators.horizontal() && (!rows.is_empty() || status) {
            list = list.push(self.hrule());
        }
        list.into()
    }

    fn tree_list(&self) -> Element<'a, Message, Theme, Renderer> {
        let rows = self.browser.tree_rows();
        let mut list = column![];
        for (index, row) in rows.iter().enumerate() {
            if index > 0 && self.separators.horizontal() {
                list = list.push(self.hrule());
            }
            list = list.push(self.line(row, true, index % 2 == 1));
            let node = self
                .browser
                .node(row.id)
                .expect("rows only come from known nodes");
            // An opened branch with no children yet is loading or failed; say so where its children's glyphs would be.
            if row.expanded && node.children.is_empty() {
                let indent = INDENT * (row.depth + 1) as f32 + CARET + GAP;
                if node.loading {
                    list = list.push(self.status_line("Loading…", None, indent));
                } else if node.failed {
                    list = list.push(self.status_line(
                        "Could not load — tap to retry",
                        Some(row.id),
                        indent,
                    ));
                }
            }
        }
        if self.separators.horizontal() && !rows.is_empty() {
            list = list.push(self.hrule());
        }
        list.into()
    }

    /// A muted note in the Name column (loading, empty, failed with retry), `indent` px into the cell.
    fn status_line(
        &self,
        label: &'a str,
        retry: Option<NodeId>,
        indent: f32,
    ) -> Element<'a, Message, Theme, Renderer> {
        let muted = self.colour(ColourField::Muted);
        let note = self.label(label).style(move |theme: &Theme| text::Style {
            color: Some(muted(theme)),
        });
        let content: Element<'a, Message, Theme, Renderer> = match retry {
            None => note.into(),
            Some(id) => button(
                row![self.glyph(icon::ERROR, 16.0, ColourField::Muted), note]
                    .spacing(6)
                    .align_y(Alignment::Center),
            )
            .padding([4, 6])
            .style(self.flat(self.colour(ColourField::Muted)))
            .on_press((self.on_event)(Event::Activate(id)))
            .into(),
        };
        row![
            Space::new().width(Length::Fixed(
                (if self.boxes_shown() { MARKER_CELL } else { 0.0 }) + CELL_PAD + indent,
            )),
            content
        ]
        .height(Length::Fixed(ROW_HEIGHT))
        .padding([0.0, EDGE])
        .align_y(Alignment::Center)
        .into()
    }

    /// One list row: the whole row is a button (activate) that highlights under the pointer, with the box and the caret as buttons of their own on top.
    fn line(&self, row: &Row, tree: bool, stripe: bool) -> Element<'a, Message, Theme, Renderer> {
        let node = self
            .browser
            .node(row.id)
            .expect("rows only come from known nodes");
        let mut name = iced_widget::Row::new()
            .spacing(GAP)
            .align_y(Alignment::Center);
        if tree {
            name = name.push(Space::new().width(Length::Fixed(INDENT * row.depth as f32)));
            name = name.push(if row.expandable {
                let caret = if row.expanded {
                    icon::CHEVRON_DOWN
                } else {
                    icon::CHEVRON_RIGHT
                };
                self.icon_button(caret, 14.0, CARET, ColourField::Muted)
                    .on_press((self.on_event)(Event::Activate(row.id)))
                    .into()
            } else {
                Element::from(Space::new().width(Length::Fixed(CARET)))
            });
        }
        let (kind_glyph, field) = match node.kind {
            EntryKind::Folder => (
                self.icons.folder(node, tree && row.expanded),
                ColourField::Folder,
            ),
            EntryKind::File => (self.icons.file(node), ColourField::Text),
        };
        name = name.push(if self.icons.tinted() {
            self.glyph(kind_glyph, GLYPH, field)
        } else {
            icon::natural(kind_glyph, GLYPH)
        });
        name = name.push(self.cut(node.name.clone(), ColourField::Text));
        // Every column after the built-in Name column renders its own cell.
        let cells = self.browser.columns()[1..]
            .iter()
            .map(|col| self.cell_content(col.cell(self.browser, node)))
            .collect();
        let content = self.template(
            self.marker_button(row.marker, Event::Toggle(row.id)),
            name.into(),
            cells,
        );
        let selected = row.marker == Marker::Selected;
        let style = Rc::clone(&self.style);
        let line: Element<'a, Message, Theme, Renderer> = button(content)
            .width(Length::Fill)
            .padding(0)
            .style(move |theme, status| {
                let appearance = style(theme);
                let background = if selected {
                    Some(appearance.selection)
                } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                    Some(appearance.row_hover)
                } else if stripe && appearance.stripe > 0.0 {
                    Some(stripe_of(appearance.background, appearance.stripe))
                } else {
                    None
                };
                button::Style {
                    background: background.map(Background::Color),
                    text_color: appearance.text,
                    ..button::Style::default()
                }
            })
            .on_press((self.on_event)(Event::Activate(row.id)))
            .into();
        if self.checkboxes
            && self.browser.selection_ui() == SelectionUi::LongPress
            && !self.browser.is_armed()
        {
            // While the boxes are hidden, a transparent layer on top of the row records press and release.
            // Stack children are handled top-down, so `HoldEnded` reaches the app before the row's own
            // activation — and `Browser::hold_ended` then swallows that activation for a revealing hold.
            iced_widget::stack![
                line,
                iced_widget::MouseArea::new(Space::new().width(Length::Fill).height(Length::Fill))
                    .on_press((self.on_event)(Event::HoldStarted(row.id)))
                    .on_release((self.on_event)(Event::HoldEnded(row.id)))
            ]
            .into()
        } else {
            line
        }
    }

    fn cell_content(&self, cell: Cell) -> Element<'a, Message, Theme, Renderer> {
        let field = |muted| {
            if muted {
                ColourField::Muted
            } else {
                ColourField::Text
            }
        };
        match cell {
            Cell::Empty => Space::new().into(),
            Cell::Text { text: label, muted } => self.cut(label, field(muted)),
            Cell::Glyph(glyph) => self.glyph(glyph, GLYPH, ColourField::Text).into(),
            Cell::Both {
                glyph,
                text: label,
                muted,
            } => row![
                self.glyph(glyph, GLYPH, ColourField::Text),
                self.cut(label, field(muted))
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into(),
        }
    }
}

/// The zebra background for one background colour at the given [`Style::stripe`] factor: a step towards black (dark backgrounds) or white (light ones).
fn stripe_of(background: Color, stripe: f32) -> Color {
    let towards = if (background.r + background.g + background.b) / 3.0 < 0.5 {
        Color::BLACK
    } else {
        Color::WHITE
    };
    mix(background, towards, stripe)
}

fn length(width: ColumnWidth) -> Length {
    match width {
        ColumnWidth::Fill => Length::Fill,
        ColumnWidth::Portion(portion) => Length::FillPortion(portion),
        ColumnWidth::Fixed(pixels) => Length::Fixed(pixels),
    }
}
