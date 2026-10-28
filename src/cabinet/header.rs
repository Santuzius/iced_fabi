//! The column header: the select-all box, the sort buttons and the selection count.

use std::rc::Rc;

use iced_widget::{
    Space, button, container,
    core::{Alignment, Background, Element, Font, Length, Theme},
    row,
};

use super::Cabinet;
use super::paint::{CARET, ColourField, GAP, ROW_HEIGHT};
use crate::{Marker, browser::Mode};

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    /// The select-all box and the sort headers — or, while something is selected, the selection count. Under [`Selection::Single`](crate::Selection::Single) the box cell stays empty: there is no "all" to select, and the open folder it would stand for is a picker's [fallback](crate::Browser::set_pick_open_folder) already.
    pub(super) fn column_header(&self) -> Element<'a, Message, Theme, Renderer> {
        let select_all = self.boxes_shown() && self.browser.selection() != crate::Selection::Single;
        let marker = select_all.then(|| {
            self.marker_button(
                self.browser.scope_marker(),
                crate::Event::HeaderMarkerPressed,
            )
        });
        let tree = self.browser.mode() == Mode::Tree;
        let content = if self.browser.has_selection() {
            let count = self.browser.selection_count();
            // The count row keeps the full template (marker cell, Name column, rules) so the grid does not fall apart while something is selected.
            self.template(
                Length::Fixed(ROW_HEIGHT),
                self.separators.header_rules(),
                marker.unwrap_or_else(|| iced_widget::Space::new().into()),
                container(self.strong((self.labels.selected)(count))).into(),
                self.empty_cells(),
            )
        } else {
            let mut headers: Vec<Element<'a, Message, Theme, Renderer>> =
                (0..self.browser.column_count())
                    .map(|index| self.sort_button(index))
                    .collect();
            let mut name = headers.remove(0);
            if tree {
                // In tree mode the label starts where the top level's glyphs do, behind the caret slot.
                name = row![Space::new().width(Length::Fixed(CARET + GAP)), name].into();
            }
            self.template(
                Length::Fixed(ROW_HEIGHT),
                self.separators.header_rules(),
                marker.unwrap_or_else(|| iced_widget::Space::new().into()),
                name,
                headers,
            )
        };
        // The colour band sits behind the content and stops 1 px short of the box's edges, so the host's frame line stays visible (see `paint::FRAME`).
        let style = Rc::clone(&self.style);
        let band = container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |theme: &Theme| container::Style {
                background: Some(Background::Color(style(theme).header)),
                ..container::Style::default()
            });
        iced_widget::stack![self.framed(Length::Fixed(ROW_HEIGHT), band.into()), content]
            .width(Length::Fill)
            .into()
    }

    /// One column's header: its title, plus the direction arrow when the list is sorted by it.
    fn sort_button(&self, index: usize) -> Element<'a, Message, Theme, Renderer> {
        let (sort_column, ascending) = self.browser.sort();
        let mut title = row![
            self.label(
                self.browser
                    .column(index)
                    .expect("header index taken from column_count")
                    .header()
            )
        ]
        .spacing(4)
        .align_y(Alignment::Center);
        if sort_column == index {
            let arrow = if ascending {
                crate::icon::SORT_ASCENDING
            } else {
                crate::icon::SORT_DESCENDING
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
        .on_press((self.on_event)(crate::Event::SortPressed(index)))
        .into()
    }

    pub(super) fn marker_button(
        &self,
        marker: Marker,
        event: crate::Event,
    ) -> Element<'a, Message, Theme, Renderer> {
        let (glyph, field) = match marker {
            Marker::Unselected => (crate::icon::SQUARE_OUTLINE, ColourField::Muted),
            Marker::Selected => (crate::icon::CHECKBOX, ColourField::Accent),
            Marker::Partial => (crate::icon::SQUARE_DOT, ColourField::Accent),
        };
        self.icon_button(glyph, 20.0, 28.0, field)
            .on_press((self.on_event)(event))
            .into()
    }
}
