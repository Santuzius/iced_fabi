//! The tool confirmation modal: any [tool](crate::Tool::confirmation) can put a question in front of its click; the cabinet draws it as the shared [dialog shell](super::dialog) — heading, optional question, Cancel and an action-named confirm button.

use iced_widget::{
    column,
    core::{Element, Font, Theme},
};

use super::{Cabinet, ToolbarItem};
use crate::ToolConfirmation;

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    /// The modal for one tool's [`ToolConfirmation`], stacked over the finished body by [`build`](Self::build) while the browser has that tool pending. Confirm and Cancel go through [`ToolConfirmationConfirmed`](crate::Event::ToolConfirmationConfirmed) and [`ToolConfirmationCancelled`](crate::Event::ToolConfirmationCancelled) — the app sees the confirmed one as [`ToolConfirmed`](crate::Outcome::ToolConfirmed) and runs the tool's action itself.
    pub(super) fn confirmation_dialog(
        &self,
        confirmation: &ToolConfirmation,
    ) -> Element<'a, Message, Theme, Renderer> {
        let confirm_press = (self.on_event)(crate::Event::ToolConfirmationConfirmed);
        let cancel = (self.on_event)(crate::Event::ToolConfirmationCancelled);

        let mut lines: iced_widget::Column<'a, Message, Theme, Renderer> =
            column![self.strong(confirmation.title.clone())];
        if let Some(body) = &confirmation.body {
            lines = lines.push(self.label(body.clone()));
        }
        let card = self.dialog_card(
            lines
                .push(self.dialog_buttons(
                    self.cancel_button(cancel.clone()),
                    self.confirm_button(
                        confirmation.confirm.clone(),
                        confirmation.tone,
                        confirm_press,
                    ),
                ))
                .spacing(super::dialog::LINE_GAP),
        );
        self.dialog_layer(card, cancel)
    }

    /// The modal for the browser's pending tool: the tool is looked up among the frame's toolbar items by switch id and asked [again](crate::Tool::confirmation), so the words track the live selection and follow the app's labels. No such tool in the bar — hidden or dropped since it was pressed — means no modal: the pending id sits harmlessly until the next modal press replaces or clears it.
    pub(super) fn pending_confirmation_dialog(
        &self,
    ) -> Option<Element<'a, Message, Theme, Renderer>> {
        let key = self.browser.pending_tool_confirmation()?;
        let tool = self.tools.iter().find_map(|item| match item {
            ToolbarItem::Tool(tool) => (tool.id().as_deref() == Some(key.id())).then_some(&**tool),
            ToolbarItem::Separator => None,
        })?;
        tool.confirmation(self.browser, &self.labels)
            .map(|confirmation| self.confirmation_dialog(&confirmation))
    }
}
