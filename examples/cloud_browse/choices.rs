//! The demo's configuration dropdowns: one enum per setting on the bar above the cabinet, each with the list of choices its pick list offers and the `Display` text the button shows.

use std::fmt;

/// Which [`Theme`] the window and the cabinet draw with; the Syncative entries mirror `syncative/src/theme.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeChoice {
    DefaultLight,
    DefaultDark,
    SyncativeLight,
    SyncativeDark,
}

impl fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DefaultLight => "Default light",
            Self::DefaultDark => "Default dark",
            Self::SyncativeLight => "Syncative light",
            Self::SyncativeDark => "Syncative dark",
        })
    }
}

pub(crate) const THEMES: &[ThemeChoice] = &[
    ThemeChoice::DefaultLight,
    ThemeChoice::DefaultDark,
    ThemeChoice::SyncativeLight,
    ThemeChoice::SyncativeDark,
];

/// Which icon pack labels the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IconChoice {
    Tabler,
    MaterialIconTheme,
}

impl fmt::Display for IconChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Tabler => "Tabler",
            Self::MaterialIconTheme => "Material Icon Theme",
        })
    }
}

pub(crate) const ICON_CHOICES: &[IconChoice] = &[IconChoice::Tabler, IconChoice::MaterialIconTheme];

/// How tools that are currently not applicable present themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InactiveChoice {
    /// Stay in the toolbar, muted and unclickeable.
    GreyedOut,
    /// Leave the toolbar until they apply.
    Hidden,
}

impl fmt::Display for InactiveChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GreyedOut => "greyed out",
            Self::Hidden => "hidden",
        })
    }
}

pub(crate) const INACTIVE_CHOICES: &[InactiveChoice] =
    &[InactiveChoice::GreyedOut, InactiveChoice::Hidden];

/// How the selection boxes appear (the [`iced_fabi::Checkboxes`] setting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoxesChoice {
    Shown,
    OnLongPress,
    Hidden,
}

impl fmt::Display for BoxesChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Shown => "shown",
            Self::OnLongPress => "on long press",
            Self::Hidden => "hidden",
        })
    }
}

pub(crate) const BOXES_CHOICES: &[BoxesChoice] = &[
    BoxesChoice::Shown,
    BoxesChoice::OnLongPress,
    BoxesChoice::Hidden,
];

/// Which kinds of entries the lists show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryChoice {
    All,
    FoldersOnly,
}

impl fmt::Display for EntryChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::All => "files & folders",
            Self::FoldersOnly => "folders only",
        })
    }
}

pub(crate) const ENTRY_CHOICES: &[EntryChoice] = &[EntryChoice::All, EntryChoice::FoldersOnly];

/// How many entries the selection can hold (the [`iced_fabi::Selection`] setting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionChoice {
    Multiple,
    Single,
    None,
}

impl fmt::Display for SelectionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Multiple => "multiple",
            Self::Single => "single",
            Self::None => "none (view)",
        })
    }
}

pub(crate) const SELECTION_CHOICES: &[SelectionChoice] = &[
    SelectionChoice::Multiple,
    SelectionChoice::Single,
    SelectionChoice::None,
];

/// Where the action button triggered by a selection sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionChoice {
    Wide,
    Floating,
    Toolbar,
}

impl fmt::Display for ActionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Wide => "under list",
            Self::Floating => "floating",
            Self::Toolbar => "toolbar",
        })
    }
}

pub(crate) const ACTION_CHOICES: &[ActionChoice] = &[
    ActionChoice::Wide,
    ActionChoice::Floating,
    ActionChoice::Toolbar,
];

pub(crate) const LINES: &[iced_fabi::Separators] = &[
    iced_fabi::Separators::None,
    iced_fabi::Separators::HeaderOnly,
    iced_fabi::Separators::Horizontal,
    iced_fabi::Separators::Vertical,
    iced_fabi::Separators::Both,
];

/// How strongly every second row stands off from the background ([`iced_fabi::Style::stripe`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZebraChoice {
    Off,
    Subtle,
    Strong,
}

impl ZebraChoice {
    pub(crate) fn stripe(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Subtle => 0.12,
            Self::Strong => 0.35,
        }
    }
}

impl fmt::Display for ZebraChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Off => "off",
            Self::Subtle => "subtle",
            Self::Strong => "strong",
        })
    }
}

pub(crate) const ZEBRA: &[ZebraChoice] =
    &[ZebraChoice::Off, ZebraChoice::Subtle, ZebraChoice::Strong];
