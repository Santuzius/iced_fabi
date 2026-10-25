//! The [`Cabinet`] builder and the widget itself: toolbar, header, rows and the action button, assembled per frame from a [`Browser`] plus the builder's settings.
//!
//! This module owns the settings (one place to read them all) and the per-frame `build`; the rendering lives in the topic files — [`paint`](paint), [`toolbar`](toolbar), [`header`](header), [`list`](list), [`action`](action) — each adding its own `impl Cabinet` block.

mod action;
mod cut;
mod header;
mod labels;
mod list;
mod paint;
mod toolbar;

use std::rc::Rc;

use iced_widget::{
    container,
    core::{Element, Font, Length, Pixels, Theme},
    scrollable,
};

use crate::{
    Mode,
    browser::Browser,
    icon::{RowIcons, TablerIcons},
    style::{Style, default_style},
    tool::Tool,
};

pub use action::ActionPlacement;
pub use labels::Labels;

/// Which separator lines the list draws; [`separators`](Cabinet::separators).
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

impl std::fmt::Display for Separators {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Both => "both",
        })
    }
}

pub(super) enum ToolbarItem<'a, Message> {
    Tool(Box<dyn Tool<Message> + 'a>),
    Separator,
}

/// A scrollbar style function as [`scrollbar`](Cabinet::scrollbar) stores it.
type ScrollbarStyle<'a> = Rc<dyn Fn(&Theme, scrollable::Status) -> scrollable::Style + 'a>;

/// The cabinet's one action button; see [`action_button`](Cabinet::action_button).
pub(super) struct Action<Message> {
    pub(super) text: String,
    pub(super) on_press: Message,
}

/// The file cabinet as a builder: call [`cabinet`] (or [`file_cabinet`] for the defaults) and configure toolbar and colours before [`build`](Self::build)ing.
pub struct Cabinet<'a, Message: Clone, Renderer> {
    pub(super) browser: &'a Browser,
    pub(super) on_event: &'a dyn Fn(crate::Event) -> Message,
    pub(super) tools: Vec<ToolbarItem<'a, Message>>,
    pub(super) mode_toggle: bool,
    pub(super) action: Option<Action<Message>>,
    pub(super) action_placement: ActionPlacement,
    pub(super) style: Rc<dyn Fn(&Theme) -> Style + 'a>,
    pub(super) icons: &'a dyn RowIcons,
    pub(super) separators: Separators,
    pub(super) labels: Labels<'a>,
    pub(super) text_size: Option<Pixels>,
    pub(super) font: Option<Font>,
    pub(super) scrollbar: Option<(scrollable::Direction, ScrollbarStyle<'a>)>,
    pub(super) _renderer: std::marker::PhantomData<fn() -> Renderer>,
}

/// Start building the widget over `browser`; `on_event` maps the widget's events to your messages.
pub fn cabinet<'a, Message, Renderer>(
    browser: &'a Browser,
    on_event: &'a dyn Fn(crate::Event) -> Message,
) -> Cabinet<'a, Message, Renderer>
where
    Message: Clone + 'a,
{
    Cabinet {
        browser,
        on_event,
        tools: Vec::new(),
        mode_toggle: true,
        action: None,
        action_placement: ActionPlacement::default(),
        style: Rc::new(default_style),
        icons: &TablerIcons,
        separators: Separators::default(),
        labels: Labels::default(),
        text_size: None,
        font: None,
        scrollbar: None,
        _renderer: std::marker::PhantomData,
    }
}

/// The default cabinet: toolbar with breadcrumbs, sort header and the row list for the browser's current mode, with the built-in mode switch only.
pub fn file_cabinet<'a, Message, Renderer>(
    browser: &'a Browser,
    on_event: &'a dyn Fn(crate::Event) -> Message,
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

    /// Whether a selection box column goes on the rows and the header right now: the browser's [`boxes_visible`](Browser::boxes_visible) — the [`Selection`](crate::Selection) allows it and the [`Checkboxes`](crate::Checkboxes) setting says it is visible now (in [`OnLongPress`](crate::Checkboxes::OnLongPress) only once a row has been held long enough).
    pub(super) fn boxes_shown(&self) -> bool {
        self.browser.boxes_visible()
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
                        .align_x(iced_widget::core::Alignment::End)
                        .align_y(iced_widget::core::Alignment::End)
                        .padding(16)
                ]
                .into()
            }
            _ => list.into(),
        };
        let mut body = iced_widget::column![
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
                    .padding([8.0, paint::EDGE]),
            );
        }
        body.into()
    }
}
