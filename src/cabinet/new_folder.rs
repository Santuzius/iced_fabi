//! The new folder dialog the built-in Create folder button opens: the shared [dialog shell](super::dialog) filled with the [drafted name](crate::Browser::open_new_folder_dialog) for the new folder.

use std::rc::Rc;

use iced_widget::{
    column, text_input,
    core::{Element, Font, Length, Theme},
};

use super::Cabinet;

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    /// The dialog layer, stacked over the finished body by [`build`](Self::build) while the browser holds a draft. The field types through [`NewFolderNameEdited`](crate::Event::NewFolderNameEdited) into the browser, Enter and Create confirm, Cancel dismisses — all via the cabinet's `on_event`, so an app that routes events to [`Browser::update`](crate::Browser::update) needs no dialog code of its own.
    pub(super) fn new_folder_dialog(&self, draft: &str) -> Element<'a, Message, Theme, Renderer> {
        let on_event = Rc::clone(&self.on_event);
        let confirm = (self.on_event)(crate::Event::NewFolderConfirmed);
        let cancel = (self.on_event)(crate::Event::NewFolderCancelled);

        let mut field = text_input("", draft)
            .on_input(move |name| on_event(crate::Event::NewFolderNameEdited(name)))
            .on_submit(confirm.clone())
            .width(Length::Fill)
            .padding([6.0, 8.0]);
        if let Some(size) = self.text_size {
            field = field.size(size);
        }
        if let Some(font) = self.font {
            field = field.font(font);
        }

        let lines: iced_widget::Column<'a, Message, Theme, Renderer> = column![
            self.strong(self.labels.new_folder.clone()),
            self.label(self.labels.new_folder_question.clone()),
            field
        ];
        let card = self.dialog_card(
            lines
                .push(
                    self.dialog_buttons(
                        self.cancel_button(cancel.clone()),
                        self.confirm_button(
                            self.labels.create.clone(),
                            crate::ConfirmTone::Accent,
                            confirm,
                        ),
                    ),
                )
                .spacing(super::dialog::LINE_GAP),
        );
        self.dialog_layer(card, cancel)
    }
}
