//! [`Browser`] unit tests, one file per topic module: [`node`] identity, [`loading`] listing and removal, [`navigation`] entering and breadcrumbs, [`selection`] ticking and markers, [`long_press`] arming, [`rows`] sorting and content filters, [`columns`] column visibility, [`tools`] tool visibility and the clipboard flag.

mod columns;
mod loading;
mod long_press;
mod navigation;
mod rows;
mod selection;
mod tools;
mod update;

/// The column at an index, for assertions: every test asks for an index it set up itself, so a missing one means a broken test, not a browser behaviour.
pub(super) fn column(browser: &crate::Browser, index: usize) -> &dyn crate::Column {
    browser
        .column(index)
        .expect("column index set up by the test itself")
}
