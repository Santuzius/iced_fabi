//! Toolbar tools an app adds to the cabinet's bar: the [`Tool`] trait and the closure-built [`SimpleTool`].

use crate::browser::Browser;

/// A toolbar button the programmer adds with [`tool`](crate::Cabinet::tool).
///
/// Implement it on your own type (per-app state decides `enabled`), or build a quick one from closures with [`tool`]. Its click yields your message; what happens next (copy, paste, share, delete, …) is the application's business — the widget just renders the button and sends the message.
pub trait Tool<Message> {
    /// The toolbar glyph.
    fn glyph(&self) -> icondata::Icon;

    /// Stable identity for the tool visibility configuration ([`Browser::hide_tool`](Browser::hide_tool)), which names tools instead of counting positions. Default: the empty string, meaning the tool is always drawn and cannot be named there; the built-in tools and any tool the app wants switchable return a fixed id.
    fn id(&self) -> String {
        String::new()
    }

    /// Whether the button is clickable (grey when not).
    fn enabled(&self, _browser: &Browser) -> bool {
        true
    }

    /// The message a click produces.
    fn message(&self, browser: &Browser) -> Message;
}

/// The built-in tools as a unit enum that is itself a [`Tool`]: one variant per tool, drawn with its glyph and enabled from browser state — Copy and Delete while something is selected, Paste while the browser carries a [clipboard](Browser::set_has_clipboard). Its click yields the variant itself, so the message type needs an `impl From<BuiltInTool>`; [`Cabinet::built_in_tools`](crate::Cabinet::built_in_tools) adds the whole set. For tools of your own, implement [`Tool`] (the demo's `DuplicateHere` shows how) or use [`tool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInTool {
    /// Copy the selection into the app's clipboard.
    Copy,
    /// Hand the clipboard to the app for pasting here.
    Paste,
    /// Remove the selection.
    Delete,
}

impl BuiltInTool {
    /// The switch id of this tool: `copy`, `paste` or `delete`.
    pub fn id(&self) -> String {
        match self {
            Self::Copy => "copy".into(),
            Self::Paste => "paste".into(),
            Self::Delete => "delete".into(),
        }
    }

    /// The tool's name, for menus that list the tools.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::Delete => "Delete",
        }
    }

    fn glyph(&self) -> icondata::Icon {
        match self {
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

    fn id(&self) -> String {
        BuiltInTool::id(self)
    }

    fn enabled(&self, browser: &Browser) -> bool {
        match self {
            Self::Copy | Self::Delete => browser.has_selection(),
            Self::Paste => browser.has_clipboard(),
        }
    }

    fn message(&self, _browser: &Browser) -> Message {
        Message::from(*self)
    }
}

/// A tool named in the visibility configuration: [`ToolKey::built_in`] names one of the built-ins, [`ToolKey::custom`] a custom tool by the [`id`](Tool::id) it returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolKey(String);

impl ToolKey {
    /// The key of a built-in tool.
    pub fn built_in(tool: BuiltInTool) -> Self {
        Self(tool.id())
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

/// A [`Tool`] made from closures; see [`tool`].
pub struct SimpleTool<Message> {
    glyph: icondata::Icon,
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
        message: Box::new(message),
        enabled: Box::new(|_| true),
    }
}

impl<Message> SimpleTool<Message> {
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

    fn enabled(&self, browser: &Browser) -> bool {
        (self.enabled)(browser)
    }

    fn message(&self, browser: &Browser) -> Message {
        (self.message)(browser)
    }
}
