//! Toolbar tools an app adds to the cabinet's bar: the [`Tool`] trait and the closure-built [`SimpleTool`].

use crate::browser::Browser;

/// A toolbar button the programmer adds with [`tool`](crate::Cabinet::tool).
///
/// Implement it on your own type (per-app state decides `enabled`), or build a quick one from closures with [`tool`]. Its click yields your message; what happens next (copy, paste, share, delete, …) is the application's business — the widget just renders the button and sends the message.
pub trait Tool<Message> {
    /// The toolbar glyph.
    fn glyph(&self) -> icondata::Icon;

    /// Whether the button is clickable (grey when not).
    fn enabled(&self, _browser: &Browser) -> bool {
        true
    }

    /// The message a click produces.
    fn message(&self, browser: &Browser) -> Message;
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
