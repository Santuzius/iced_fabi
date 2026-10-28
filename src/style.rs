//! The widget's palette ([`Style`]) and its theme-derived default.

use iced_widget::core::{Color, Theme};

/// The widget's colours. Defaults from the theme's palette (see [`from_theme`](Self::from_theme)); override the whole thing per theme in [`style`](crate::Cabinet::style) to apply your app's design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// Primary text and icons (file kind glyphs).
    pub text: Color,
    /// Secondary text and icons (markers, carets, breadcrumb ancestors, cells).
    pub muted: Color,
    /// Selected markers and the sort column's arrow.
    pub accent: Color,
    /// Text on accent-filled surfaces (the action button's label).
    pub on_accent: Color,
    /// Hover variant of [`accent`](Self::accent) (the action button under the pointer).
    pub accent_hover: Color,
    /// Fill of a destructive action's button — the confirm button of a [`ConfirmTone::Danger`](crate::ConfirmTone::Danger) confirmation.
    pub danger: Color,
    /// Hover variant of [`danger`](Self::danger).
    pub danger_hover: Color,
    /// Text on danger-filled surfaces.
    pub on_danger: Color,
    /// Tint of the folder glyphs of tinted icon packs.
    pub folder: Color,
    /// Tint of the folder glyphs of tinted icon packs on a selected row, where [`folder`](Self::folder) may sink into the [`selection`](Self::selection) band (iced's palettes derive both from the accent).
    pub selected_folder: Color,
    /// Row background while selected. It gives way towards [`background`](Self::background) as far as the [`accent`](Self::accent) tick needs to keep a 3 : 1 contrast on it — the selection's counterpart of the header bounding the [`stripe`](Self::stripe).
    pub selection: Color,
    /// Row background under the pointer; keep it weaker than [`selection`](Self::selection).
    pub row_hover: Color,
    /// Background of the column header row.
    pub header: Color,
    /// Separator lines: under the toolbar and the header, and the optional row and column [`Separators`](crate::Separators).
    pub line: Color,
    /// Zebra striping: every second row's background darkens by this fraction of its own colour (0 = off, 1 = black); one factor therefore reads equally subtle on dark and light backgrounds. The header colour bounds the stripe, keeping the header the strongest surface: a factor reaching past the header stops at it, a subtle one (at most 0.2) at a third of the distance to it. Only a near-black background lightens its rows instead.
    pub stripe: f32,
    /// What the cabinet sits on, the base of the zebra stripe. The theme background by default, your card or panel colour when the cabinet is placed on one.
    pub background: Color,
    /// Background of the flat buttons (breadcrumbs, tools, markers, carets) under the pointer.
    pub hover: Color,
    /// Background of a flat button while it is pressed.
    pub pressed: Color,
    /// Corner radius of the hover and press backgrounds.
    pub radius: f32,
}

/// `a` moved towards `b` by `t` (0 = `a`, 1 = `b`), opaque.
pub(super) fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    }
}

/// The readable text colour on a surface filled with `fill`: white on a dark fill, black on a light one.
pub(super) fn on_colour(fill: Color) -> Color {
    if 0.299 * fill.r + 0.587 * fill.g + 0.114 * fill.b < 0.5 {
        Color::WHITE
    } else {
        Color::BLACK
    }
}

/// [`Style`] from the theme's palette.
impl Style {
    /// Every field from the active iced theme's palette — the widget's default (what [`style`](crate::Cabinet::style) starts with).
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Style {
        let palette = theme.palette();
        let extended = theme.extended_palette();
        let selection = extended.primary.weak.color;
        Style {
            text: palette.text,
            muted: Color {
                a: 0.6,
                ..palette.text
            },
            accent: palette.primary,
            on_accent: on_colour(palette.primary),
            accent_hover: mix(palette.primary, palette.text, 0.12),
            danger: palette.danger,
            on_danger: on_colour(palette.danger),
            danger_hover: mix(palette.danger, palette.text, 0.12),
            folder: mix(palette.primary, palette.background, 0.3),
            // The colour iced's palette pairs with this very background for content on it.
            selected_folder: extended.primary.weak.text,
            selection,
            row_hover: Color {
                a: 0.35,
                ..selection
            },
            header: mix(palette.background, Color::BLACK, 0.06),
            line: Color {
                a: 0.15,
                ..palette.text
            },
            stripe: 0.0,
            background: palette.background,
            hover: Color {
                a: 0.08,
                ..palette.text
            },
            pressed: Color {
                a: 0.14,
                ..palette.text
            },
            radius: 4.0,
        }
    }
}
