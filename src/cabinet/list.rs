//! The row list: browse and tree listings, one row at a time (the whole row is the activate button, with box and caret buttons on top), status lines, and cell rendering.

use std::rc::Rc;

use iced_widget::{
    Space, button, column,
    core::{Alignment, Background, Element, Font, Length, Theme},
    row, text,
};

use super::Cabinet;
use super::paint::{CARET, ColourField, GAP, GLYPH, INDENT, ROW_HEIGHT};
use crate::{
    Marker,
    browser::{EntryKind, NodeId, rows::Row},
    column::Cell,
    icon,
};

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    pub(super) fn browse_list(&self) -> Element<'a, Message, Theme, Renderer> {
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
            list = list.push(self.status_line(self.labels.loading.clone(), None, 0.0));
            status = true;
        } else if current.is_some_and(|n| n.failed) {
            list = list.push(self.status_line(
                self.labels.retry.clone(),
                Some(self.browser.current()),
                0.0,
            ));
            status = true;
        } else if current.is_some_and(|n| n.children_known && n.children.is_empty()) {
            list = list.push(self.status_line(self.labels.empty.clone(), None, 0.0));
            status = true;
        }
        // The last entry gets a line below it too, the same colour as the ones between the rows.
        if self.separators.horizontal() && (!rows.is_empty() || status) {
            list = list.push(self.hrule());
        }
        list.into()
    }

    pub(super) fn tree_list(&self) -> Element<'a, Message, Theme, Renderer> {
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
                    list = list.push(self.status_line(self.labels.loading.clone(), None, indent));
                } else if node.failed {
                    list = list.push(self.status_line(
                        self.labels.retry.clone(),
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
        label: String,
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
            .on_press((self.on_event)(crate::Event::RetryPressed(id)))
            .into(),
        };
        // Through the template, so the upright rules stay solid across a status line too; only the indent is the status line's own.
        self.template(
            Length::Fixed(ROW_HEIGHT),
            self.separators.vertical(),
            iced_widget::Space::new().into(),
            row![Space::new().width(Length::Fixed(indent)), content]
                .align_y(Alignment::Center)
                .into(),
            self.empty_cells(),
        )
    }

    /// One list row: a layer behind the content carries the row's colour (selection, hover, zebra stripe) and the activate click, highlighting under the pointer; the box and the caret are buttons of their own inside that content.
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
                    .on_press((self.on_event)(crate::Event::CaretPressed(row.id)))
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
            Length::Fixed(ROW_HEIGHT),
            self.separators.vertical(),
            self.marker_button(row.marker, crate::Event::MarkerPressed(row.id)),
            name.into(),
            cells,
        );
        let selected = row.marker == Marker::Selected;
        let style = Rc::clone(&self.style);
        // One layer behind the content carries the row's colour and the activate click, kept clear of the box's edge so the host's frame line survives (see `paint::FRAME`). The box and caret buttons sit inside that content, so their press still reaches them before the row's: a `Button` forwards the event to its content and stops once it is captured.
        let line: Element<'a, Message, Theme, Renderer> = iced_widget::stack![
            self.framed(
                Length::Fixed(ROW_HEIGHT),
                button(Space::new().width(Length::Fill).height(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(0)
                    .style(move |theme, status| {
                        let appearance = style(theme);
                        let background = if selected {
                            Some(appearance.selection)
                        } else if matches!(
                            status,
                            button::Status::Hovered | button::Status::Pressed
                        ) {
                            Some(appearance.row_hover)
                        } else if stripe && appearance.stripe > 0.0 {
                            Some(super::paint::stripe_of(
                                appearance.background,
                                appearance.header,
                                appearance.stripe,
                            ))
                        } else {
                            None
                        };
                        button::Style {
                            background: background.map(Background::Color),
                            ..button::Style::default()
                        }
                    })
                    .on_press((self.on_event)(crate::Event::RowPressed(row.id)))
                    .into(),
            ),
            content,
        ]
        .width(Length::Fill)
        .into();
        if self.browser.checkboxes() == crate::Checkboxes::OnLongPress && !self.browser.is_armed() {
            // While the boxes are hidden, a transparent layer on top of the row records press and release.
            // Stack children are handled top-down, so `HoldEnded` reaches the app before the row's own
            // activation — and `Browser::hold_ended` then swallows that activation for a revealing hold.
            iced_widget::stack![
                line,
                iced_widget::MouseArea::new(Space::new().width(Length::Fill).height(Length::Fill))
                    .on_press((self.on_event)(crate::Event::HoldStarted(row.id)))
                    .on_release((self.on_event)(crate::Event::HoldEnded(row.id)))
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
