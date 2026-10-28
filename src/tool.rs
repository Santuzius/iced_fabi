//! Toolbar tools an app adds to the cabinet's bar: the [`Tool`] trait and the closure-built [`SimpleTool`].

use crate::browser::Browser;
use crate::Labels;

/// How the confirm button of a [`ToolConfirmation`] is coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmTone {
    /// Filled with the palette's [`accent`](crate::Style::accent) — the ordinary choice.
    #[default]
    Accent,
    /// Filled with the palette's [`danger`](crate::Style::danger): the button warns the way a destructive action does (the built-in Delete does this).
    Danger,
}

/// The modal a tool asks for before its click does its work: drawn by the cabinet as a centred card — [`title`](Self::title), an optional [`body`](Self::body) paragraph, and a confirm button labelled [`confirm`](Self::confirm) next to the cabinet's own Cancel. See [`Tool::confirmation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolConfirmation {
    /// Card heading, typically the tool's own name.
    pub title: String,
    /// The question under the heading, e.g. `Are you sure?`. `None` draws just the heading.
    pub body: Option<String>,
    /// The confirm button's label — name the action ("Delete"), not "OK", so the question is answerable at a glance.
    pub confirm: String,
    /// Colour of that confirm button: [`Danger`](ConfirmTone::Danger) for a destructive action, [`Accent`](ConfirmTone::Accent) otherwise.
    pub tone: ConfirmTone,
}

/// A toolbar button the programmer adds with [`tool`](crate::Cabinet::tool).
///
/// Implement it on your own type (per-app state decides `enabled`), or build a quick one from closures with [`tool`]. Its click yields your message; what happens next (copy, paste, share, delete, …) is the application's business — the widget just renders the button and sends the message. The one exception inside the widget: a tool may [route its click through `Browser::update`](Tool::event_on_press) instead, and the cabinet answers that event itself.
pub trait Tool<Message> {
    /// The toolbar glyph.
    fn glyph(&self) -> icondata::Icon;

    /// Stable identity for the tool visibility configuration ([`Browser::hide_tool`](Browser::hide_tool)), which names tools instead of counting positions. Default: `None`, meaning the tool is always drawn and can never be matched by a hidden id; the built-in tools and any tool the app wants switchable return `Some` with a fixed id.
    fn id(&self) -> Option<String> {
        None
    }

    /// Whether the button is clickable (grey when not).
    fn enabled(&self, _browser: &Browser) -> bool {
        true
    }

    /// The message a click produces.
    fn message(&self, browser: &Browser) -> Message;

    /// The [`Event`](crate::Event) a click sends to [`update`](crate::Browser::update) *instead of* [`message`](Self::message), for tools the cabinet answers itself. Default: `None` — the click yields `message`. [`BuiltInTool::CreateFolder`] uses this: its button opens the cabinet's new folder dialog, and the app only hears about it as [`CreateFolderRequested`](crate::Outcome::CreateFolderRequested).
    fn event_on_press(&self) -> Option<crate::Event> {
        None
    }

    /// The modal the click must get past *before* [`message`](Self::message) is sent: return `Some` and the button opens the cabinet's confirmation card instead; confirming reaches the app as [`Outcome::ToolConfirmed`](crate::Outcome::ToolConfirmed) naming this tool's [`id`](Self::id), and the app runs the action then. Default: `None` — the click messages straight through. Asking needs an `id`: without one nothing can name the tool back, and an anonymous tool's click simply goes through. The cabinet asks [`confirmation`](crate::Tool::confirmation) again while drawing, so the text may count the current selection; the Cancel button and the darkened backdrop dismiss. [`BuiltInTool::Delete`] asks with `Are you sure?` — its [labels](crate::Labels) supply the words. An [`event_on_press`](Self::event_on_press) tool is answered inside the widget and is not gated by this.
    fn confirmation(&self, _browser: &Browser, _labels: &Labels) -> Option<ToolConfirmation> {
        None
    }
}

/// The built-in tools as a unit enum that is itself a [`Tool`]: one variant per tool, drawn with its glyph and enabled from browser state — Create folder always, Copy and Delete while something is selected, Paste while the browser carries a [clipboard](Browser::set_has_clipboard). Its click yields the variant itself, so the message type needs an `impl From<BuiltInTool>`; only [`CreateFolder`](Self::CreateFolder) is answered inside the widget (its click [opens the new folder dialog](Tool::event_on_press)), and [`Delete`](Self::Delete) [asks first](Tool::confirmation) with a confirmation modal before the message goes out. [`Cabinet::built_in_tools`](crate::Cabinet::built_in_tools) adds Create folder, Copy and Paste. For tools of your own, implement [`Tool`] (the demo's `DuplicateHere` shows how) or use [`tool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInTool {
    /// Offer the app a new folder in the open folder. The click does not reach the app as a message: the cabinet opens a [dialog](crate::Outcome::CreateFolderRequested) asking the name, and the app gets the confirmed one as an `Outcome`.
    CreateFolder,
    /// Copy the selection into the app's clipboard.
    Copy,
    /// Hand the clipboard to the app for pasting here.
    Paste,
    /// Remove the selection.
    Delete,
}

impl BuiltInTool {
    /// Every built-in tool, in toolbar order — Create folder, Copy, Paste, Delete. [`Cabinet::built_in_tools`](crate::Cabinet::built_in_tools) adds the first three; the destructive Delete typically follows custom tools behind a [`tool_separator`](crate::Cabinet::tool_separator).
    pub const ALL: [Self; 4] = [Self::CreateFolder, Self::Copy, Self::Paste, Self::Delete];

    /// The switch id of this tool: `create-folder`, `copy`, `paste` or `delete`.
    pub fn id(&self) -> &'static str {
        match self {
            Self::CreateFolder => "create-folder",
            Self::Copy => "copy",
            Self::Paste => "paste",
            Self::Delete => "delete",
        }
    }

    /// The tool's name, for menus that list the tools.
    pub fn label(&self) -> &'static str {
        match self {
            Self::CreateFolder => "Create folder",
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::Delete => "Delete",
        }
    }

    fn glyph(&self) -> icondata::Icon {
        match self {
            Self::CreateFolder => icondata::TbFolderPlusOutline,
            Self::Copy => icondata::TbCopyOutline,
            Self::Paste => icondata::TbClipboardFilled,
            Self::Delete => icondata::TbTrashOutline,
        }
    }
}

impl<Message: From<BuiltInTool>> Tool<Message> for BuiltInTool {
    fn glyph(&self) -> icondata::Icon {
        BuiltInTool::glyph(self)
    }

    fn id(&self) -> Option<String> {
        Some(BuiltInTool::id(self).to_owned())
    }

    fn enabled(&self, browser: &Browser) -> bool {
        match self {
            Self::CreateFolder => true,
            Self::Copy | Self::Delete => browser.has_selection(),
            Self::Paste => browser.has_clipboard(),
        }
    }

    fn message(&self, _browser: &Browser) -> Message {
        Message::from(*self)
    }

    fn event_on_press(&self) -> Option<crate::Event> {
        match self {
            // The cabinet answers this one itself: the click opens its new folder dialog.
            Self::CreateFolder => Some(crate::Event::CreateFolderPressed),
            Self::Copy | Self::Paste | Self::Delete => None,
        }
    }

    fn confirmation(&self, browser: &Browser, labels: &Labels) -> Option<ToolConfirmation> {
        match self {
            // Deleting is the destructive one: ask first, name the button after the act, and
            // spell out on a second line what the selection really removes — selected folders
            // stand for their whole subtree, so the raw tick count would understate it.
            Self::Delete => {
                let (folders, files) = browser.selected_entry_counts();
                let body = if folders + files == 0 {
                    labels.delete_question.clone()
                } else {
                    format!(
                        "{}\n{}",
                        labels.delete_question,
                        (labels.delete_count)(folders, files)
                    )
                };
                Some(ToolConfirmation {
                    title: labels.delete.clone(),
                    body: Some(body),
                    confirm: labels.delete.clone(),
                    tone: ConfirmTone::Danger,
                })
            }
            Self::CreateFolder | Self::Copy | Self::Paste => None,
        }
    }
}

/// A tool named in the visibility configuration: [`ToolKey::built_in`] names one of the built-ins, [`ToolKey::custom`] a custom tool by the [`id`](Tool::id) it returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolKey(String);

impl ToolKey {
    /// The key of a built-in tool.
    pub fn built_in(tool: BuiltInTool) -> Self {
        Self(tool.id().to_owned())
    }

    /// The key of a custom tool: the [`id`](Tool::id) it returns.
    pub fn custom(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The tool id this key names.
    pub fn id(&self) -> &str {
        &self.0
    }
}

impl From<BuiltInTool> for ToolKey {
    fn from(tool: BuiltInTool) -> Self {
        Self::built_in(tool)
    }
}

impl From<&str> for ToolKey {
    fn from(id: &str) -> Self {
        Self::custom(id)
    }
}

impl From<String> for ToolKey {
    fn from(id: String) -> Self {
        Self::custom(id)
    }
}

/// A [`Tool`] made from closures; see [`tool`]. Without an [`id`](SimpleTool::id) it is anonymous and always drawn.
pub struct SimpleTool<Message> {
    glyph: icondata::Icon,
    id: Option<String>,
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
        id: None,
        message: Box::new(message),
        enabled: Box::new(|_| true),
    }
}

impl<Message> SimpleTool<Message> {
    /// Give the tool a fixed switch id so the visibility configuration can name it: `.id("duplicate")`. Without it [`Tool::id`] is `None` and the tool always draws.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

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

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn enabled(&self, browser: &Browser) -> bool {
        (self.enabled)(browser)
    }

    fn message(&self, browser: &Browser) -> Message {
        (self.message)(browser)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in message type: the built-in tools need one that can name them, exactly as an app's does.
    #[derive(Debug, Clone)]
    struct Msg;

    impl From<BuiltInTool> for Msg {
        fn from(_tool: BuiltInTool) -> Self {
            Msg
        }
    }

    #[test]
    fn built_in_delete_asks_with_the_frame_labels_and_the_others_do_not() {
        let mut browser = crate::Browser::sample();
        browser.toggle(crate::NodeId::derive(
            crate::NodeId::ROOT,
            crate::EntryKind::Folder,
            "Finanzen",
        ));
        browser.toggle(crate::NodeId::derive(
            crate::NodeId::ROOT,
            crate::EntryKind::File,
            "notes.txt",
        ));
        let asked = Tool::<Msg>::confirmation(&BuiltInTool::Delete, &browser, &Labels::default())
            .expect("Delete asks first");
        assert_eq!(asked.title, "Delete");
        assert_eq!(asked.confirm, "Delete");
        assert_eq!(
            asked.tone,
            crate::ConfirmTone::Danger,
            "the destructive confirm is drawn in the danger colour"
        );
        assert_eq!(
            asked.body.as_deref(),
            Some("Are you sure you want to delete the selected entries?\n1 folder, 1 file"),
            "the body names the question and what the delete removes"
        );
        let german = Tool::<Msg>::confirmation(&BuiltInTool::Delete, &browser, &Labels::german())
            .expect("Delete asks first in German too");
        assert_eq!(german.title, "Löschen");
        assert!(
            german
                .body
                .as_deref()
                .is_some_and(|body| body.contains("1 Ordner, 1 Datei"))
        );
        for tool in [
            BuiltInTool::CreateFolder,
            BuiltInTool::Copy,
            BuiltInTool::Paste,
        ] {
            assert_eq!(
                Tool::<Msg>::confirmation(&tool, &browser, &Labels::default()),
                None
            );
        }
    }
}
