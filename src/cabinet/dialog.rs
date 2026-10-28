//! The shared shell of the cabinet's centred modals — the new folder dialog and the tool confirmation: a dimmed, click-to-dismiss backdrop over the whole cabinet and one bordered card. Each modal fills the card with its own lines and wires its own buttons; the card chrome, the accent-filled confirm button and the Cancel label behave the same in both.

use std::rc::Rc;

use iced_widget::{
    button, container, mouse_area, opaque, row, Column, Space,
    core::{Alignment, Background, Color, Element, Font, Length, Theme, border},
};

use super::paint::ColourField;
use super::Cabinet;

/// Width of the dialog card.
pub(super) const CARD_WIDTH: f32 = 300.0;
/// Inner padding of the card and the gap between its lines.
pub(super) const CARD_PADDING: f32 = 16.0;
pub(super) const LINE_GAP: f32 = 12.0;
/// Alpha of the black scrim the backdrop darkens the cabinet with — black in every theme like Syncative's modal backdrop, so a dark theme darkens too instead of lightening.
pub(super) const SCRIM: f32 = 0.45;
// The card's buttons match the bar's in height.
const BUTTON_V_PADDING: f32 = 6.0;
const BUTTON_H_PADDING: f32 = 12.0;

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    /// The centred card holding a modal's lines, spaced and padded.
    pub(super) fn dialog_card(
        &self,
        lines: Column<'a, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        let card_style = Rc::clone(&self.style);
        container(lines.spacing(LINE_GAP).padding(CARD_PADDING))
        .width(Length::Fixed(CARD_WIDTH))
        .style(move |theme| {
            let appearance = card_style(theme);
            container::Style {
                background: Some(Background::Color(appearance.background)),
                text_color: Some(appearance.text),
                border: border::rounded(appearance.radius)
                    .width(1.0)
                    .color(appearance.line),
                ..container::Style::default()
            }
        })
        .into()
    }

    /// The modal's action button: accent-filled, so the eye lands on it.
    pub(super) fn confirm_button(
        &self,
        label: String,
        press: Message,
    ) -> Element<'a, Message, Theme, Renderer> {
        let confirm_style = Rc::clone(&self.style);
        button(self.strong(label))
            .padding([BUTTON_V_PADDING, BUTTON_H_PADDING])
            .style(move |theme, status| {
                let appearance = confirm_style(theme);
                let background = match status {
                    button::Status::Hovered | button::Status::Pressed => appearance.accent_hover,
                    _ => appearance.accent,
                };
                button::Style {
                    background: Some(Background::Color(background)),
                    text_color: appearance.on_accent,
                    border: border::rounded(appearance.radius),
                    ..button::Style::default()
                }
            })
            .on_press(press)
            .into()
    }

    /// The modal's dismiss button, drawn from the shared [`cancel`](crate::Labels::cancel) label.
    pub(super) fn cancel_button(&self, press: Message) -> Element<'a, Message, Theme, Renderer> {
        button(self.label(self.labels.cancel.clone()))
            .padding([BUTTON_V_PADDING, BUTTON_H_PADDING])
            .style(self.flat(self.colour(ColourField::Muted)))
            .on_press(press)
            .into()
    }

    /// The button row: `filler` on the left, the two buttons at the right edge.
    pub(super) fn dialog_buttons(
        &self,
        cancel: Element<'a, Message, Theme, Renderer>,
        confirm: Element<'a, Message, Theme, Renderer>,
    ) -> iced_widget::Row<'a, Message, Theme, Renderer> {
        row![Space::new().width(Length::Fill), cancel, confirm]
            .spacing(8.0)
            .align_y(Alignment::Center)
    }

    /// The backdrop over the finished body with the card on top. The scrim doubles as the cancel hit-area — a click on it sends `cancel` — and `opaque` keeps the cabinet below from reacting at all, so a tool press behind an open modal cannot fire. The scrim paints as a rounded rectangle in the cabinet's own corner radius ([`Style::radius`](crate::Style::radius)); an app that wraps the cabinet in a bordered card gives that card the same radius and no padding of its own, so the darkening follows the card's rounded corners while the click-to-dismiss hit-area stays the full rectangle.
    pub(super) fn dialog_layer(
        &self,
        card: Element<'a, Message, Theme, Renderer>,
        cancel: Message,
    ) -> Element<'a, Message, Theme, Renderer> {
        let scrim_style = Rc::clone(&self.style);
        iced_widget::stack![
            opaque(
                mouse_area(
                    container(Space::new().width(Length::Fill).height(Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(move |theme| {
                            let appearance = scrim_style(theme);
                            container::Style {
                                background: Some(Background::Color(Color {
                                    a: SCRIM,
                                    ..Color::BLACK
                                })),
                                border: border::rounded(appearance.radius),
                                ..container::Style::default()
                            }
                        }),
                )
                .on_press(cancel),
            ),
            iced_widget::center(card),
        ]
        .into()
    }
}
