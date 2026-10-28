//! The Syncative look: the palette, the [`iced_fabi::Style`] for the cabinet and the widget styles the demo draws its bar and card with — a reproduction of `syncative/src/theme.rs`, bare colour values only.

use std::sync::LazyLock;

use iced::Theme;

const fn hex(hex: u32) -> iced::Color {
    iced::Color::from_rgb(
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    )
}

/// The colours Syncative's widgets draw with: a cut of the `Tones` in `syncative/src/theme.rs` (bare colour values carry no copyright).
struct Tones {
    background: iced::Color,
    nav: iced::Color,
    surface: iced::Color,
    surface_hover: iced::Color,
    border: iced::Color,
    text: iced::Color,
    muted: iced::Color,
    accent: iced::Color,
    accent_hover: iced::Color,
    on_accent: iced::Color,
    success: iced::Color,
    warning: iced::Color,
    danger: iced::Color,
    /// Syncative's `danger_fill`/`danger_fill_hover`: the fills of its destructive buttons (`theme::button_danger`), darker than the text `danger` in dark mode for contrast.
    danger_fill: iced::Color,
    danger_fill_hover: iced::Color,
    selection: iced::Color,
    /// Not in Syncative's `Tones`: the blue-grey of the folder glyphs in the cabinet.
    folder: iced::Color,
}

const SYNCATIVE_TONES_LIGHT: Tones = Tones {
    background: hex(0xf3f3f4),
    nav: hex(0xebebed),
    surface: hex(0xffffff),
    surface_hover: hex(0xf5f6f7),
    border: hex(0xdcdde0),
    text: hex(0x1c1c1f),
    muted: hex(0x606368),
    accent: hex(0x2a73c5),
    accent_hover: hex(0x2464ad),
    on_accent: hex(0xffffff),
    success: hex(0x2a8a4f),
    warning: hex(0xb7790f),
    danger: hex(0xc4372f),
    danger_fill: hex(0xc4372f),
    danger_fill_hover: hex(0xa92e27),
    selection: hex(0xdde7f4),
    folder: hex(0x5b7bb5),
};

const SYNCATIVE_TONES_DARK: Tones = Tones {
    background: hex(0x1f2023),
    nav: hex(0x18191b),
    surface: hex(0x2a2b2f),
    surface_hover: hex(0x323337),
    border: hex(0x3a3b40),
    text: hex(0xe6e6e8),
    muted: hex(0xa0a2a8),
    accent: hex(0x4c9be8),
    accent_hover: hex(0x63a9ee),
    on_accent: hex(0x0e1520),
    success: hex(0x4cbb73),
    warning: hex(0xe3a93a),
    danger: hex(0xe5584f),
    danger_fill: hex(0xc0392f),
    danger_fill_hover: hex(0xd24a40),
    selection: hex(0x2c3b4f),
    folder: hex(0x6a7db0),
};

/// Syncative's corner radius of cards, buttons and pick lists.
const SYNCATIVE_RADIUS: f32 = 6.0;
/// Syncative's body text size; it sets it on every widget, so the cabinet gets it through [`iced_fabi::Cabinet::text_size`].
pub(crate) const SYNCATIVE_TEXT: f32 = 14.0;
/// Syncative's UI font on the desktop (found through fontconfig).
pub(crate) const SYNCATIVE_FONT: iced::Font = iced::Font::with_name("Noto Sans");

fn syncative_tones(theme: &Theme) -> &'static Tones {
    if theme.extended_palette().is_dark {
        &SYNCATIVE_TONES_DARK
    } else {
        &SYNCATIVE_TONES_LIGHT
    }
}

fn syncative_palette(t: &Tones) -> iced::theme::Palette {
    iced::theme::Palette {
        background: t.background,
        text: t.text,
        primary: t.accent,
        success: t.success,
        warning: t.warning,
        danger: t.danger,
    }
}

pub(crate) static SYNCATIVE_LIGHT: LazyLock<Theme> =
    LazyLock::new(|| Theme::custom("Syncative Light", syncative_palette(&SYNCATIVE_TONES_LIGHT)));
pub(crate) static SYNCATIVE_DARK: LazyLock<Theme> =
    LazyLock::new(|| Theme::custom("Syncative Dark", syncative_palette(&SYNCATIVE_TONES_DARK)));

/// The cabinet colours Syncative's own widgets use, showing what [`iced_fabi::Cabinet::style`] is for: the palette alone would derive `muted` and the row tint differently, and the cabinet sits on a card, not on the theme background. Hover and press backgrounds are those of Syncative's flat buttons.
pub(crate) fn syncative_style(theme: &Theme) -> iced_fabi::Style {
    let t = syncative_tones(theme);
    iced_fabi::Style {
        text: t.text,
        muted: t.muted,
        accent: t.accent,
        on_accent: t.on_accent,
        accent_hover: t.accent_hover,
        danger: t.danger_fill,
        danger_hover: t.danger_fill_hover,
        on_danger: hex(0xffffff),
        selection: t.selection,
        folder: t.folder,
        row_hover: iced::Color {
            a: 0.5,
            ..t.selection
        },
        header: t.nav,
        line: t.border,
        stripe: 0.0,
        background: t.surface,
        hover: iced::Color { a: 0.08, ..t.text },
        pressed: iced::Color { a: 0.14, ..t.text },
        radius: SYNCATIVE_RADIUS,
    }
}

/// Syncative's page header (`theme::header_bar`).
pub(crate) fn syncative_header_bar(theme: &Theme) -> iced::widget::container::Style {
    let t = syncative_tones(theme);
    iced::widget::container::Style {
        background: Some(t.nav.into()),
        text_color: Some(t.text),
        ..Default::default()
    }
}

/// Syncative's page background (`theme::page`).
pub(crate) fn syncative_page(theme: &Theme) -> iced::widget::container::Style {
    let t = syncative_tones(theme);
    iced::widget::container::Style {
        background: Some(t.background.into()),
        text_color: Some(t.text),
        ..Default::default()
    }
}

/// Syncative's raised card (`theme::card`). Its `radius` is the one the cabinet's own style paints its corners with, so the modal scrim's darkening follows the card's rounded corners; the card hugs the cabinet (no padding) for the same reason.
pub(crate) fn syncative_card(theme: &Theme, radius: f32) -> iced::widget::container::Style {
    let t = syncative_tones(theme);
    iced::widget::container::Style {
        background: Some(t.surface.into()),
        text_color: Some(t.text),
        border: iced::Border {
            width: 1.0,
            radius: radius.into(),
            color: t.border,
        },
        ..Default::default()
    }
}

/// Syncative's hairline under the header (`theme::separator`).
pub(crate) fn syncative_separator(theme: &Theme) -> iced::widget::rule::Style {
    iced::widget::rule::Style {
        color: syncative_tones(theme).border,
        radius: 0.0.into(),
        fill_mode: iced::widget::rule::FillMode::Full,
        snap: true,
    }
}

/// Syncative's pick list (`theme::pick_list`).
pub(crate) fn syncative_pick_list(
    theme: &Theme,
    status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    use iced::widget::pick_list::Status;
    let t = syncative_tones(theme);
    iced::widget::pick_list::Style {
        text_color: t.text,
        placeholder_color: t.muted,
        handle_color: t.muted,
        background: match status {
            Status::Hovered => t.surface_hover.into(),
            _ => t.surface.into(),
        },
        border: iced::Border {
            width: 1.0,
            radius: SYNCATIVE_RADIUS.into(),
            color: if matches!(status, Status::Opened { .. }) {
                t.accent
            } else {
                t.border
            },
        },
    }
}

/// Syncative's drop-down menu (`theme::menu`).
pub(crate) fn syncative_menu(theme: &Theme) -> iced::overlay::menu::Style {
    let t = syncative_tones(theme);
    iced::overlay::menu::Style {
        background: t.surface.into(),
        border: iced::Border {
            width: 1.0,
            radius: SYNCATIVE_RADIUS.into(),
            color: t.border,
        },
        text_color: t.text,
        selected_text_color: t.text,
        selected_background: t.selection.into(),
        shadow: iced::Shadow {
            color: iced::Color {
                a: 0.25,
                ..iced::Color::BLACK
            },
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
    }
}

/// Syncative's slim overlay scrollbar (`theme::scrollbar`): no rail, a muted thumb that firms up on hover and drag.
pub(crate) fn syncative_scrollbar(
    theme: &Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    use iced::widget::scrollable::{self, Status};
    let t = syncative_tones(theme);
    let alpha = match status {
        Status::Active { .. } => 0.30,
        Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        }
        | Status::Dragged {
            is_vertical_scrollbar_dragged: true,
            ..
        } => 0.65,
        _ => 0.42,
    };
    let rail = scrollable::Rail {
        background: None,
        border: iced::Border::default(),
        scroller: scrollable::Scroller {
            background: iced::Color {
                a: alpha,
                ..t.muted
            }
            .into(),
            border: iced::border::rounded(4),
        },
    };
    scrollable::Style {
        container: iced::widget::container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        ..scrollable::default(theme, status)
    }
}

/// Geometry matching [`syncative_scrollbar`] (`theme::slim_scrollbar`).
pub(crate) fn syncative_scroll_direction() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new()
            .width(6)
            .scroller_width(6)
            .margin(2),
    )
}
