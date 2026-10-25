//! The cabinet's one action button: where it goes ([`ActionPlacement`]) and how it renders.

use std::rc::Rc;

use iced_widget::{
    button, container,
    core::{Background, Border, Color, Element, Font, Length, Theme},
};

use super::{Action, Cabinet};

/// Where the [`action_button`](crate::Cabinet::action_button) goes.
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

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    /// The action button as an element, styled like Syncative's `button_primary`: an accent-filled pill with the [`on_accent`](crate::Style::on_accent) label and a same-colour 1-px border; hover swaps in [`accent_hover`](crate::Style::accent_hover), and pressed is the base again. The corners are completely round (a radius of half the button's height). Without a selection it sits greyed like an inactive tool: the fill, label and border at 50 %, 45 % and 50 % alpha, and no click. `placement` sizes it: full-width and centred text under the list, content-sized and compact in the toolbar.
    pub(super) fn action_button_element(
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
}
