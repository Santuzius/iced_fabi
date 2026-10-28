//! The tool confirmation modal: opened by a tool press (or the app), confirmed into `ToolConfirmed`, cancelled into nothing, and never side by side with the new folder dialog.

use crate::browser::{Event, Outcome};
use crate::{Browser, ToolKey};

/// Sample switch id of a tool that asks first.
const DELETE: &str = "delete";

#[test]
fn a_pressed_tool_opens_the_modal_and_names_itself() {
    let mut browser = Browser::sample();
    assert_eq!(browser.pending_tool_confirmation(), None);
    let request = Event::ToolConfirmationRequested {
        tool: ToolKey::custom(DELETE),
    };
    assert_eq!(browser.update(request), Outcome::None);
    assert_eq!(
        browser.pending_tool_confirmation(),
        Some(ToolKey::custom(DELETE))
    );
}

#[test]
fn confirming_hands_the_tool_back_exactly_once() {
    let mut browser = Browser::sample();
    let request = Event::ToolConfirmationRequested {
        tool: ToolKey::custom(DELETE),
    };
    let _ = browser.update(request);
    assert_eq!(
        browser.update(Event::ToolConfirmationConfirmed),
        Outcome::ToolConfirmed {
            tool: ToolKey::custom(DELETE),
        }
    );
    assert_eq!(browser.pending_tool_confirmation(), None);
    // A confirm that reaches the browser twice must not run the tool twice.
    assert_eq!(
        browser.update(Event::ToolConfirmationConfirmed),
        Outcome::None
    );
}

#[test]
fn cancelling_runs_nothing_and_a_later_confirm_is_silent() {
    let mut browser = Browser::sample();
    let request = Event::ToolConfirmationRequested {
        tool: ToolKey::custom(DELETE),
    };
    let _ = browser.update(request);
    assert_eq!(
        browser.update(Event::ToolConfirmationCancelled),
        Outcome::None
    );
    assert_eq!(browser.pending_tool_confirmation(), None);
    assert_eq!(
        browser.update(Event::ToolConfirmationConfirmed),
        Outcome::None
    );
}

#[test]
fn events_on_a_closed_modal_do_nothing() {
    let mut browser = Browser::sample();
    assert_eq!(
        browser.update(Event::ToolConfirmationConfirmed),
        Outcome::None
    );
    assert_eq!(
        browser.update(Event::ToolConfirmationCancelled),
        Outcome::None
    );
    assert_eq!(browser.pending_tool_confirmation(), None);
}

#[test]
fn only_one_modal_is_open_at_a_time() {
    let mut browser = Browser::sample();
    let request = Event::ToolConfirmationRequested {
        tool: ToolKey::custom(DELETE),
    };
    let _ = browser.update(request.clone());
    // The app's own create-folder shortcut replaces the pending confirmation…
    browser.open_new_folder_dialog();
    assert_eq!(browser.pending_tool_confirmation(), None);
    assert!(browser.new_folder_dialog_open());
    // …and a pressed asking tool replaces the new folder dialog.
    let _ = browser.update(request.clone());
    assert!(!browser.new_folder_dialog_open());
    assert_eq!(
        browser.pending_tool_confirmation(),
        Some(ToolKey::custom(DELETE))
    );
    // The app's dismiss methods do the same from the other side.
    browser.close_tool_confirmation();
    browser.open_new_folder_dialog();
    browser.close_new_folder_dialog();
    assert!(!browser.new_folder_dialog_open());
    assert_eq!(browser.pending_tool_confirmation(), None);
}
