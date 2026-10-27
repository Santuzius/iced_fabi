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
    /// Tint of the folder glyphs of tinted icon packs.
    pub folder: Color,
    /// Row background while selected.
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
            on_accent: if 0.299 * palette.primary.r
                + 0.587 * palette.primary.g
                + 0.114 * palette.primary.b
                < 0.5
            {
                Color::WHITE
            } else {
                Color::BLACK
            },
            accent_hover: mix(palette.primary, palette.text, 0.12),
            folder: mix(palette.primary, palette.background, 0.3),
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
