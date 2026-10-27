//! Tool configuration: the built-in enum, the clipboard flag and hiding.

use crate::browser::{Browser, EntryKind, NodeId};
use crate::tool::Tool;
use crate::{BuiltInTool, ToolKey};

/// A stand-in message type: the built-in tools need one that can name them, exactly as an app's does.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Msg;

impl From<BuiltInTool> for Msg {
    fn from(_tool: BuiltInTool) -> Self {
        Msg
    }
}

#[test]
fn the_clipboard_flag_answers_the_app() {
    let mut browser = Browser::new("My files");
    assert!(!browser.has_clipboard());
    browser.set_has_clipboard(true);
    assert!(browser.has_clipboard());
}

#[test]
fn the_built_ins_enable_from_browser_state() {
    let browser = Browser::sample();
    assert!(
        !<BuiltInTool as Tool<Msg>>::enabled(&BuiltInTool::Copy, &browser),
        "nothing selected yet"
    );
    let notes = NodeId::derive(NodeId::ROOT, EntryKind::File, "notes.txt");
    let mut browser = browser;
    browser.toggle(notes);
    assert!(<BuiltInTool as Tool<Msg>>::enabled(&BuiltInTool::Copy, &browser));
    assert!(<BuiltInTool as Tool<Msg>>::enabled(&BuiltInTool::Delete, &browser));
    assert!(
        !<BuiltInTool as Tool<Msg>>::enabled(&BuiltInTool::Paste, &browser),
        "the clipboard belongs to the app"
    );
    browser.set_has_clipboard(true);
    assert!(<BuiltInTool as Tool<Msg>>::enabled(&BuiltInTool::Paste, &browser));
    assert_eq!(
        <BuiltInTool as Tool<Msg>>::message(&BuiltInTool::Copy, &browser),
        Msg,
        "the click yields the message the app maps the variant to"
    );
}

#[test]
fn tools_hide_and_show_by_key() {
    let mut browser = Browser::new("My files");
    browser.hide_tool(BuiltInTool::Copy);
    assert!(browser.is_tool_hidden("copy"));
    browser.show_tool(BuiltInTool::Copy);
    assert!(!browser.is_tool_hidden("copy"));
    // Custom tools are named by the id their Tool returns; a plain string is the same key.
    browser.hide_tool("duplicate");
    assert!(browser.is_tool_hidden("duplicate"));
    browser.show_tool(ToolKey::custom("duplicate"));
    assert!(!browser.is_tool_hidden("duplicate"));
}

#[test]
fn an_anonymous_tool_cannot_be_hidden() {
    let mut browser = Browser::new("My files");
    browser.hide_tool("");
    browser.hide_tool(ToolKey::custom(String::new()));
    assert!(!browser.is_tool_hidden(""));
}

#[test]
fn the_built_in_keys_are_their_ids() {
    let key = ToolKey::from(BuiltInTool::Paste);
    assert_eq!(key.id(), BuiltInTool::Paste.id());
    assert_eq!(BuiltInTool::Delete.label(), "Delete");
}
