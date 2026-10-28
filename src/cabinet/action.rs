//! The cabinet's one action button: where it goes ([`ActionPlacement`]) and how it renders.

use std::rc::Rc;

use iced_widget::{
    button, container,
    core::{Background, Border, Color, Element, Font, Length, Theme},
};

use super::Cabinet;

use crate::browser::Browser;

/// Where the [`action`](crate::Cabinet::action) button goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionPlacement {
    /// Wide and centred below the row list.
    #[default]
    WideCenter,
    /// Floating over the rows in the cabinet's bottom-right corner.
    Floating,
    /// As the last item of the toolbar, right of the tool groups.
    ToolbarEnd,
    /// Below the row list at the right edge, beside the [`cancel`](crate::Cabinet::cancel) button, drawn like the confirm button of the cabinet's dialogs — the footer of a picker.
    Footer,
}

/// Corner radius, in px, that renders as completely round corners. Both iced 0.14 renderers clamp every corner radius to half the shorter side of the box before drawing it (tiny-skia: `iced_tiny_skia::engine::Engine::draw_quad`, "Make sure the border radius is not larger than the bounds"; wgpu: `iced_wgpu`'s `shader/quad/solid.wgsl`), so one value larger than the tallest button the cabinet draws is enough — and it needs neither the button's height (which differs by [`ActionPlacement`]) nor the renderer's default text size, which a style closure never sees (only `(&Theme, Status)` arrives there).
const ROUND_PILL: f32 = 1_000.0;

/// An enable rule as [`when`](ActionButton::when) stores it.
type WhenRule<'a> = Rc<dyn Fn(&Browser) -> bool + 'a>;

/// The cabinet's one action button: label, click message, where it sits and when it is enabled. Build it with [`new`](Self::new), place it with [`placement`](Self::placement), gate it with [`when`](Self::when), and hand it to [`Cabinet::action`].
pub struct ActionButton<'a, Message> {
    pub(super) text: String,
    pub(super) on_press: Message,
    pub(super) placement: ActionPlacement,
    pub(super) when: Option<WhenRule<'a>>,
}

impl<'a, Message: Clone> ActionButton<'a, Message> {
    /// The button's label and the message a click sends. Default placement [`WideCenter`](ActionPlacement::WideCenter), enabled while the browser [has a pick](Browser::has_pick): something is selected, or a folder picker's open folder stands in for it.
    #[must_use]
    pub fn new(text: impl Into<String>, on_press: Message) -> Self {
        Self {
            text: text.into(),
            on_press,
            placement: ActionPlacement::default(),
            when: None,
        }
    }

    /// Where the button goes.
    #[must_use]
    pub fn placement(mut self, placement: ActionPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Replace the enable rule (mirrors a tool's `when`); false renders it grey like an inactive tool. Default: [`Browser::has_pick`]. A picker that preselects the app's saved state passes `.when(|_| true)`, since confirming an emptied selection is an answer too.
    #[must_use]
    pub fn when(mut self, f: impl Fn(&Browser) -> bool + 'a) -> Self {
        self.when = Some(Rc::new(f));
        self
    }
}

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    /// Whether the action's enable rule lets it be clicked this frame.
    pub(super) fn action_enabled(&self, action: &ActionButton<'a, Message>) -> bool {
        match &action.when {
            Some(f) => f(self.browser),
            None => self.browser.has_pick(),
        }
    }

    /// The bar below the row list, when one is needed: the [`WideCenter`](ActionPlacement::WideCenter) button full-width, or the [`Footer`](ActionPlacement::Footer) button at the right edge — each with the [`cancel`](Cabinet::cancel) button before it, which otherwise sits alone at the right edge.
    pub(super) fn footer(&self) -> Option<Element<'a, Message, Theme, Renderer>> {
        let cancel = self
            .cancel
            .as_ref()
            .map(|press| self.cancel_button(press.clone()));
        let placement = self.action.as_ref().map(|action| action.placement);
        let bar: iced_widget::Row<'a, Message, Theme, Renderer> = match (&self.action, placement) {
            (Some(action), Some(ActionPlacement::WideCenter)) => {
                let wide = self.action_button_element(action, ActionPlacement::WideCenter);
                iced_widget::Row::new().push(cancel).push(wide)
            }
            (Some(action), Some(ActionPlacement::Footer)) => {
                let confirm = self.confirm_button(
                    action.text.clone(),
                    crate::ConfirmTone::Accent,
                    self.action_enabled(action).then(|| action.on_press.clone()),
                );
                self.dialog_buttons_maybe(cancel, confirm)
            }
            _ => {
                let cancel = cancel?;
                iced_widget::row![iced_widget::Space::new().width(Length::Fill), cancel]
            }
        };
        Some(
            container(
                bar.spacing(8.0)
                    .align_y(iced_widget::core::Alignment::Center),
            )
            .width(Length::Fill)
            .padding([8.0, super::paint::EDGE])
            .into(),
        )
    }

    /// The action button as an element: an accent-filled pill with the [`on_accent`](crate::Style::on_accent) label and a same-colour 1-px border; hover swaps in [`accent_hover`](crate::Style::accent_hover), and pressed is the base again. The corners are completely round (the radius is clamped by the renderer to half the button's height). Without a selection it sits greyed like an inactive tool: the fill, label and border at 50 %, 45 % and 50 % alpha, and no click. `placement` sizes it: full-width and centred text under the list, content-sized and compact in the toolbar.
    pub(super) fn action_button_element(
        &self,
        action: &ActionButton<'a, Message>,
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
        let enabled = self.action_enabled(action);
        if enabled {
            btn = btn.on_press(action.on_press.clone());
        }
        let style = Rc::clone(&self.style);
        btn.style(move |theme, status| {
            let appearance = style(theme);
            let ghost = status == button::Status::Disabled;
            let bg = if status == button::Status::Hovered {
                appearance.accent_hover
            } else {
                appearance.accent
            };
            let fade = |c: Color, a: f32| Color { a, ..c };
            // Completely round corners at any placement and text size: the renderer clamps this to half the button's height.
            let pill = ROUND_PILL;
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

#[cfg(test)]
mod tests {
    use super::{ActionButton, ActionPlacement, Cabinet};
    use crate::Browser;

    #[test]
    fn the_action_button_carries_its_placement_and_rule() {
        let browser = Browser::new("t");
        let cabinet: Cabinet<'_, (), iced::Renderer> = crate::cabinet(&browser, |_| ()).action(
            ActionButton::new("Go", ())
                .placement(ActionPlacement::Floating)
                .when(|b| b.has_selection()),
        );
        let action = cabinet.action.expect("set");
        assert_eq!(action.placement, ActionPlacement::Floating);
        assert!(
            action.when.is_some(),
            "the enable rule survived the builder"
        );
    }

    /// The bar below the list exists exactly when something needs it: a button placed there, or a Cancel.
    #[test]
    fn the_footer_appears_for_its_placements_and_for_cancel() {
        let browser = Browser::new("t");
        let footer = |action: Option<ActionPlacement>, cancel: bool| {
            let mut cabinet: Cabinet<'_, (), iced::Renderer> = crate::cabinet(&browser, |_| ());
            if let Some(placement) = action {
                cabinet = cabinet.action(ActionButton::new("Go", ()).placement(placement));
            }
            if cancel {
                cabinet = cabinet.cancel(());
            }
            cabinet.footer().is_some()
        };
        assert!(!footer(None, false));
        assert!(footer(None, true));
        assert!(footer(Some(ActionPlacement::WideCenter), false));
        assert!(footer(Some(ActionPlacement::Footer), false));
        assert!(!footer(Some(ActionPlacement::Floating), false));
        assert!(!footer(Some(ActionPlacement::ToolbarEnd), false));
        assert!(footer(Some(ActionPlacement::ToolbarEnd), true));
    }
}
