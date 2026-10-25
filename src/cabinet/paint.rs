//! The shared painting plumbing of the cabinet: layout geometry, the [`ColourField`] colour selector, flat-button styles, rules and the column [`template`](Cabinet::template) that header and rows alike run through, so their columns line up by construction.

use std::rc::Rc;

use iced_widget::{
    button, container,
    core::{Alignment, Background, Color, Element, Font, Length, Theme, border, font},
    text,
};

use super::Cabinet;
use crate::{column::ColumnWidth, icon, style::Style};

// --- geometry: one template for the header and every row, so the columns line up by construction ---

/// Height of the toolbar.
pub(super) const BAR_HEIGHT: f32 = 44.0;
/// Height of the column header and of every list row.
pub(super) const ROW_HEIGHT: f32 = 36.0;
/// Inset of the first and last cell from the cabinet's edges.
pub(super) const EDGE: f32 = 8.0;
/// The selection box column at the left edge.
pub(super) const MARKER_CELL: f32 = 36.0;
/// Horizontal padding inside every column cell, header and rows alike.
pub(super) const CELL_PAD: f32 = 8.0;
/// The tree caret's slot in front of the row glyph.
pub(super) const CARET: f32 = 20.0;
/// Tree indent per level.
pub(super) const INDENT: f32 = 20.0;
/// The gap between caret, glyph and name inside the Name cell.
pub(super) const GAP: f32 = 8.0;
/// Size of the file and folder glyphs.
pub(super) const GLYPH: f32 = 18.0;

/// Which [`Style`] colour a piece of chrome wants.
#[derive(Clone, Copy)]
pub(super) enum ColourField {
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
pub(super) type Colour<'a> = Rc<dyn Fn(&Theme) -> Color + 'a>;

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    pub(super) fn colour(&self, field: ColourField) -> Colour<'a> {
        let style = Rc::clone(&self.style);
        Rc::new(move |theme: &Theme| field.get(style(theme)))
    }

    pub(super) fn glyph(
        &self,
        glyph: icondata::Icon,
        size: f32,
        field: ColourField,
    ) -> iced_widget::Svg<'a> {
        icon::glyph(glyph, size, self.colour(field))
    }

    /// A label at the cabinet's text size and font.
    pub(super) fn label(
        &self,
        content: impl text::IntoFragment<'a>,
    ) -> text::Text<'a, Theme, Renderer> {
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
    pub(super) fn strong(
        &self,
        content: impl text::IntoFragment<'a>,
    ) -> text::Text<'a, Theme, Renderer> {
        self.label(content).font(Font {
            weight: font::Weight::Semibold,
            ..self.font.unwrap_or(Font::DEFAULT)
        })
    }

    /// A label cut with an ellipsis to the width of its cell.
    pub(super) fn cut(
        &self,
        label: String,
        field: ColourField,
    ) -> Element<'a, Message, Theme, Renderer> {
        super::cut::Cut::new(label, self.colour(field))
            .size(self.text_size)
            .font(self.font)
            .into()
    }

    /// A flat button's style: `colour` for its content, the hover and press backgrounds from the cabinet's [`Style`].
    pub(super) fn flat(
        &self,
        colour: Colour<'a>,
    ) -> impl Fn(&Theme, button::Status) -> button::Style + 'a {
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
    pub(super) fn icon_button(
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

    /// A 1-px rule in the [`Style::line`] colour, across.
    pub(super) fn hrule(&self) -> Element<'a, Message, Theme, Renderer> {
        let line = self.colour(ColourField::Line);
        iced_widget::rule::horizontal(1.0)
            .style(move |theme: &Theme| iced_widget::rule::Style {
                color: line(theme),
                ..iced_widget::rule::default(theme)
            })
            .into()
    }

    /// A 1-px rule in the [`Style::line`] colour, upright.
    pub(super) fn vrule(&self) -> Element<'a, Message, Theme, Renderer> {
        let line = self.colour(ColourField::Line);
        iced_widget::rule::vertical(1.0)
            .style(move |theme: &Theme| iced_widget::rule::Style {
                color: line(theme),
                ..iced_widget::rule::default(theme)
            })
            .into()
    }

    /// The row template: selection box (when the boxes are shown), the Name cell and the other columns (with optional rules between them). The header and every row go through here, so their columns always share the same boundaries. With vertical separators the row is framed by upright rules at its very left and right too, so the table's edge lines are solid from the header to the last row — as overlays: a fixed-width child at the end of a filling row misplaces itself in iced's flex, while a container aligned to the edge of a stack cannot.
    pub(super) fn template(
        &self,
        height: Length,
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
            .height(height)
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
        if !self.separators.vertical() {
            return row.into();
        }
        // The edge rules go on top as full-height overlays: a fixed child at the end of a filling row misplaces itself in iced's flex, but a container aligned to the edge of a stack cannot. The stack is as big as the row, so both rules span exactly the row's height.
        iced_widget::stack![
            row,
            container(self.vrule())
                .width(Length::Fill)
                .height(height)
                .align_x(Alignment::Start),
            container(self.vrule())
                .width(Length::Fill)
                .height(height)
                .align_x(Alignment::End),
        ]
        .width(Length::Fill)
        .height(height)
        .into()
    }

    /// Blank cells, one per column after the Name column — for rows that carry no data (status lines, the selection-count header) yet must still draw the column rules.
    pub(super) fn empty_cells(&self) -> Vec<Element<'a, Message, Theme, Renderer>> {
        (1..self.browser.columns().len())
            .map(|_| iced_widget::Space::new().into())
            .collect()
    }
}

/// The zebra background for one background colour at the given [`Style::stripe`] factor: a step towards black (dark backgrounds) or white (light ones).
pub(super) fn stripe_of(background: Color, stripe: f32) -> Color {
    let towards = if (background.r + background.g + background.b) / 3.0 < 0.5 {
        Color::BLACK
    } else {
        Color::WHITE
    };
    crate::style::mix(background, towards, stripe)
}

fn length(width: ColumnWidth) -> Length {
    match width {
        ColumnWidth::Fill => Length::Fill,
        ColumnWidth::Portion(portion) => Length::FillPortion(portion),
        ColumnWidth::Fixed(pixels) => Length::Fixed(pixels),
    }
}
