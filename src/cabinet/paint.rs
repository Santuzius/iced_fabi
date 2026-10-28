//! The shared painting plumbing of the cabinet: layout geometry, the [`ColourField`] colour selector, flat-button styles, rules and the column [`template`](Cabinet::template) that header and rows alike run through, so their columns line up by construction.

use std::rc::Rc;

use iced_widget::{
    button, container,
    core::{Alignment, Background, Color, Element, Font, Length, Theme, border, font},
    text,
};

use super::Cabinet;
use crate::{column::ColumnWidth, icon, style::Style};

// --- geometry: one template for the header and every row, so the columns line up by construction ---

/// Height of the toolbar.
pub(super) const BAR_HEIGHT: f32 = 44.0;
/// Height of the column header and of every list row.
pub(super) const ROW_HEIGHT: f32 = 36.0;
/// Inset of the first and last cell from the cabinet's edges.
pub(super) const EDGE: f32 = 8.0;
/// The strip the cabinet keeps free of its own backgrounds at the left and right of its box. The host draws its frame there — a card's 1-px border — and iced paints a container's child over that border, so a row or header background reaching the edge would cut that line (see `doc/iced-notes.md`).
pub(super) const FRAME: f32 = 1.0;
/// The selection box column at the left edge.
pub(super) const MARKER_CELL: f32 = 36.0;
/// Horizontal padding inside every column cell, header and rows alike.
pub(super) const CELL_PAD: f32 = 8.0;
/// The tree caret's slot in front of the row glyph.
pub(super) const CARET: f32 = 20.0;
/// Tree indent per level.
pub(super) const INDENT: f32 = 20.0;
/// The gap between caret, glyph and name inside the Name cell.
pub(super) const GAP: f32 = 8.0;
/// Size of the file and folder glyphs.
pub(super) const GLYPH: f32 = 18.0;

/// Which [`Style`] colour a piece of chrome wants.
#[derive(Clone, Copy)]
pub(super) enum ColourField {
    Text,
    Muted,
    Accent,
    Folder,
    Line,
}

impl ColourField {
    fn get(self, style: Style) -> Color {
        match self {
            ColourField::Text => style.text,
            ColourField::Muted => style.muted,
            ColourField::Accent => style.accent,
            ColourField::Folder => style.folder,
            ColourField::Line => style.line,
        }
    }
}

/// A colour resolved per frame from the theme.
pub(super) type Colour<'a> = Rc<dyn Fn(&Theme) -> Color + 'a>;

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    pub(super) fn colour(&self, field: ColourField) -> Colour<'a> {
        let style = Rc::clone(&self.style);
        Rc::new(move |theme: &Theme| field.get(style(theme)))
    }

    pub(super) fn glyph(
        &self,
        glyph: icondata::Icon,
        size: f32,
        field: ColourField,
    ) -> iced_widget::Svg<'a> {
        icon::glyph(glyph, size, self.colour(field))
    }

    /// A label at the cabinet's text size and font.
    pub(super) fn label(
        &self,
        content: impl text::IntoFragment<'a>,
    ) -> text::Text<'a, Theme, Renderer> {
        let mut label = text(content);
        if let Some(size) = self.text_size {
            label = label.size(size);
        }
        if let Some(font) = self.font {
            label = label.font(font);
        }
        label
    }

    /// A label in the semibold weight of the cabinet's font.
    pub(super) fn strong(
        &self,
        content: impl text::IntoFragment<'a>,
    ) -> text::Text<'a, Theme, Renderer> {
        self.label(content).font(Font {
            weight: font::Weight::Semibold,
            ..self.font.unwrap_or(Font::DEFAULT)
        })
    }

    /// A label cut with an ellipsis to the width of its cell.
    pub(super) fn cut(
        &self,
        label: String,
        field: ColourField,
    ) -> Element<'a, Message, Theme, Renderer> {
        super::cut::Cut::new(label, self.colour(field))
            .size(self.text_size)
            .font(self.font)
            .into()
    }

    /// A flat button's style: `colour` for its content, the hover and press backgrounds from the cabinet's [`Style`].
    pub(super) fn flat(
        &self,
        colour: Colour<'a>,
    ) -> impl Fn(&Theme, button::Status) -> button::Style + 'a {
        let style = Rc::clone(&self.style);
        move |theme, status| {
            let appearance = style(theme);
            let background = match status {
                button::Status::Hovered => Some(appearance.hover),
                button::Status::Pressed => Some(appearance.pressed),
                button::Status::Active | button::Status::Disabled => None,
            };
            button::Style {
                background: background.map(Background::Color),
                text_color: colour(theme),
                border: border::rounded(appearance.radius),
                ..button::Style::default()
            }
        }
    }

    /// A square icon button: the glyph centred in `side` px.
    pub(super) fn icon_button(
        &self,
        glyph: icondata::Icon,
        size: f32,
        side: f32,
        field: ColourField,
    ) -> iced_widget::Button<'a, Message, Theme, Renderer> {
        button(
            container(self.glyph(glyph, size, field))
                .center_x(Length::Fixed(side))
                .center_y(Length::Fixed(side)),
        )
        .padding(0)
        .style(self.flat(self.colour(field)))
    }

    /// A 1-px rule in the [`Style::line`] colour, across.
    pub(super) fn hrule(&self) -> Element<'a, Message, Theme, Renderer> {
        let line = self.colour(ColourField::Line);
        iced_widget::rule::horizontal(1.0)
            .style(move |theme: &Theme| iced_widget::rule::Style {
                color: line(theme),
                ..iced_widget::rule::default(theme)
            })
            .into()
    }

    /// A 1-px rule in the [`Style::line`] colour, upright.
    pub(super) fn vrule(&self) -> Element<'a, Message, Theme, Renderer> {
        let line = self.colour(ColourField::Line);
        iced_widget::rule::vertical(1.0)
            .style(move |theme: &Theme| iced_widget::rule::Style {
                color: line(theme),
                ..iced_widget::rule::default(theme)
            })
            .into()
    }

    /// A background layer sized to the row's box but kept [`FRAME`] clear of its left and right edge, so the frame line the host draws there stays visible. Header and rows place their colour (the header band, the stripe, the hover and selection fill) on such a layer behind their content.
    pub(super) fn framed(
        &self,
        height: Length,
        layer: Element<'a, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        container(layer)
            .width(Length::Fill)
            .height(height)
            .padding([0.0, FRAME])
            .into()
    }

    /// The width the column at `index` claims. Every index reaching here comes from [`column_count`](crate::Browser::column_count) or from the cells a caller built over that range, so it is in range.
    fn column_width(&self, index: usize) -> ColumnWidth {
        self.browser
            .column(index)
            .expect("column index taken from column_count")
            .width()
    }

    /// The row template: selection box (when the boxes are shown), the Name cell and the other columns. The header and every row go through here, so their columns always share the same boundaries. `rules` draws the upright rules between the cells and frames the row with them at its very left and right, so the table's edge lines are solid from the header to the last row — on the [`FRAME`](super::paint::FRAME) strip itself, where the host's frame line sits, and as overlays: a fixed-width child at the end of a filling row misplaces itself in iced's flex, while a container aligned to the edge of a stack cannot. The caller passes the flag, which lets the header carry a grid the rows do not ([`HeaderOnly`](crate::Separators::HeaderOnly)). A column the browser hides is skipped whole, cell and neighbouring rule.
    pub(super) fn template(
        &self,
        height: Length,
        rules: bool,
        marker: Element<'a, Message, Theme, Renderer>,
        name: Element<'a, Message, Theme, Renderer>,
        cells: Vec<Element<'a, Message, Theme, Renderer>>,
    ) -> Element<'a, Message, Theme, Renderer> {
        let cell = |content: Element<'a, Message, Theme, Renderer>, width: ColumnWidth| {
            container(content)
                .width(length(width))
                .height(Length::Fill)
                .padding([0.0, CELL_PAD])
                .align_y(Alignment::Center)
                .clip(true)
        };
        let mut row = iced_widget::Row::new()
            .height(height)
            .padding([0.0, EDGE])
            .align_y(Alignment::Center);
        if self.boxes_shown() {
            row = row.push(container(marker).center_x(Length::Fixed(MARKER_CELL)));
        }
        row = row.push(cell(name, self.column_width(0)));
        for (index, content) in cells.into_iter().enumerate() {
            // A hidden column is skipped whole: cell and the rule before it, so its neighbours meet with one rule between them and the width they get is what a hidden column never claimed.
            if self.browser.is_column_hidden_at(index + 1) {
                continue;
            }
            if rules {
                row = row.push(self.vrule());
            }
            row = row.push(cell(content, self.column_width(index + 1)));
        }
        if !rules {
            return row.into();
        }
        // The edge rules go on top as full-height overlays: a fixed child at the end of a filling row misplaces itself in iced's flex, but a container aligned to the edge of a stack cannot. The stack is as big as the row, so both rules span exactly the row's height.
        iced_widget::stack![
            row,
            container(self.vrule())
                .width(Length::Fill)
                .height(height)
                .align_x(Alignment::Start),
            container(self.vrule())
                .width(Length::Fill)
                .height(height)
                .align_x(Alignment::End),
        ]
        .width(Length::Fill)
        .height(height)
        .into()
    }

    /// Blank cells, one per column after the Name column — for rows that carry no data (status lines, the selection-count header) yet must still draw the column rules.
    pub(super) fn empty_cells(&self) -> Vec<Element<'a, Message, Theme, Renderer>> {
        (1..self.browser.column_count())
            .map(|_| iced_widget::Space::new().into())
            .collect()
    }
}

/// A [`Style::stripe`] factor up to this reads as a subtle stripe, which stays strictly lighter than the header (see [`stripe_of`]).
const SUBTLE_STRIPE_MAX: f32 = 0.2;
/// How much of the background-to-header distance a subtle stripe may use (a third, so its lightness step is of the same order on light and dark backgrounds); the rest keeps the header distinct from the rows.
const SUBTLE_HEADER_SHARE: f32 = 1.0 / 3.0;

/// The zebra background for one background colour at the given [`Style::stripe`] factor: the row darkens by this fraction of its own colour — proportional, so one factor reads with equal strength on dark and on light backgrounds. Only a background so dark that it has no room left to darken shifts towards white instead.
///
/// The header bounds the stripe, so the header stays the strongest surface of the table: a factor reaching past the [`Style::header`] colour stops right at it, a subtle factor (at most [`SUBTLE_STRIPE_MAX`]) uses at most a third of the background-to-header distance — a subtle stripe in the header's own colour would read as a stray header row, and near it would read almost as dark on a light background. A header that does not stand off the background in the stripe's direction bounds nothing.
pub(super) fn stripe_of(background: Color, header: Color, stripe: f32) -> Color {
    let average = |color: Color| (color.r + color.g + color.b) / 3.0;
    let lightening = average(background) < 0.12;
    let (towards, room) = if lightening {
        (Color::WHITE, 1.0 - average(background))
    } else {
        (Color::BLACK, average(background))
    };
    // How far the stripe would have to go towards `towards` to reach the header's own lightness.
    let to_header = if lightening {
        average(header) - average(background)
    } else {
        average(background) - average(header)
    };
    let factor = if to_header <= 0.0 {
        stripe
    } else {
        let to_header = to_header / room;
        let limit = if stripe <= SUBTLE_STRIPE_MAX {
            to_header * SUBTLE_HEADER_SHARE
        } else {
            to_header
        };
        stripe.min(limit)
    };
    crate::style::mix(background, towards, factor)
}

/// WCAG relative luminance of an sRGB colour (alpha ignored).
fn luminance(color: Color) -> f32 {
    let linear = |c: f32| {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

/// WCAG contrast ratio of two colours, 1 (same luminance) to 21 (black on white).
fn contrast(a: Color, b: Color) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// The band behind a selected row, bounded by the tick on it the way [`stripe_of`] bounds the stripe by the header: the [`Style::selection`] colour (laid over `background`, since an app's may be translucent) may sit at most halfway from the background to the [`Style::accent`], halfway measured in contrast — the tick keeps at least the square root of its contrast to the background, so it never stands off its row less than the row stands off the background. iced's palettes derive the selection from the accent, so on a dark theme the band can come out nearly in the tick's own colour and the tick vanishes into it; a band past halfway gives way towards the background by the smallest step that gets there. A band nearer the background stays as it is.
pub(super) fn selection_of(background: Color, selection: Color, accent: Color) -> Color {
    let background = Color {
        a: 1.0,
        ..background
    };
    let accent = Color { a: 1.0, ..accent };
    let band = crate::style::mix(
        background,
        Color {
            a: 1.0,
            ..selection
        },
        selection.a,
    );
    let floor = contrast(accent, background).sqrt();
    if contrast(accent, band) >= floor {
        return band;
    }
    // Searched on a grid rather than solved: the contrast need not change monotonically along the way, and the first grid step that clears the floor is the smallest one. At the background itself the tick has its full contrast, which is never below its square root, so the search always ends.
    const STEPS: u16 = 64;
    (1..=STEPS)
        .map(|step| crate::style::mix(band, background, f32::from(step) / f32::from(STEPS)))
        .find(|candidate| contrast(accent, *candidate) >= floor)
        .unwrap_or(background)
}

fn length(width: ColumnWidth) -> Length {
    match width {
        ColumnWidth::Fill => Length::Fill,
        ColumnWidth::Portion(portion) => Length::FillPortion(portion),
        ColumnWidth::Fixed(pixels) => Length::Fixed(pixels),
    }
}

#[cfg(test)]
mod tests {
    use super::{contrast, selection_of, stripe_of};
    use iced_widget::core::Color;

    /// A band the tick already reads on stays the app's colour.
    #[test]
    fn a_band_the_tick_reads_on_stays() {
        let background = Color::WHITE;
        let selection = Color::from_rgb8(0xdd, 0xe7, 0xf4);
        let accent = Color::from_rgb8(0x2a, 0x73, 0xc5);
        assert_eq!(selection_of(background, selection, accent), selection);
    }

    /// The bug this prevents: on iced's Default dark the tick (the primary, 88/101/242) sat on the band (primary.weak, 74/84/195) at 1.36 : 1 and vanished — pixels measured on the picker example, background 43/45/49. The band must give way to halfway in contrast, and no further.
    #[test]
    fn a_band_of_the_ticks_own_hue_gives_way_to_halfway() {
        let background = Color::from_rgb8(43, 45, 49);
        let selection = Color::from_rgb8(74, 84, 195);
        let accent = Color::from_rgb8(88, 101, 242);
        let halfway = contrast(accent, background).sqrt();
        assert!(contrast(accent, selection) < halfway, "the case is real");
        let band = selection_of(background, selection, accent);
        assert!(contrast(accent, band) >= halfway);
        assert!(
            contrast(band, background) > 1.6,
            "the row still stands off the background about as much"
        );
        // The step the search took, read back off the blue channel, and the grid step before it.
        let taken = (selection.b - band.b) / (selection.b - background.b);
        let one_step_less = crate::style::mix(selection, background, taken - 1.0 / 64.0);
        assert!(
            contrast(accent, one_step_less) < halfway,
            "the smallest step that reaches halfway"
        );
    }

    /// A translucent selection is judged as it shows: over the background.
    #[test]
    fn a_translucent_selection_is_laid_over_the_background() {
        let background = Color::WHITE;
        let selection = Color {
            a: 0.5,
            ..Color::BLACK
        };
        let accent = Color::BLACK;
        let band = selection_of(background, selection, accent);
        assert!((band.r - 0.5).abs() < 1e-6 && band.a == 1.0);
    }

    /// The bug this prevents: stripes mixed towards white vanished on light backgrounds.
    #[test]
    fn stripes_stay_visible_on_light_and_dark_backgrounds() {
        let white = Color::from_rgb(1.0, 1.0, 1.0);
        let light = stripe_of(white, Color::from_rgb(0.5, 0.5, 0.5), 0.12);
        assert!(
            light.r < 1.0 && light.g < 1.0 && light.b < 1.0,
            "light stripes must darken"
        );

        let dark = Color::from_rgb(0.16, 0.17, 0.18);
        let darker = stripe_of(dark, Color::from_rgb(0.1, 0.11, 0.12), 0.12);
        assert!(
            darker.r < dark.r && darker.g < dark.g && darker.b < dark.b,
            "dark stripes keep darkening as they always did"
        );

        let near_black = Color::from_rgb(0.02, 0.02, 0.02);
        assert!(
            stripe_of(near_black, near_black, 0.12).r > near_black.r,
            "a near-black background must lighten its rows instead"
        );
    }

    /// No row may darken past the header, so the header stays the strongest surface: a strong factor reaching past it lands exactly on the header colour.
    #[test]
    fn strong_stripes_stop_at_the_header() {
        let white = Color::from_rgb(1.0, 1.0, 1.0);
        let header = Color::from_rgb(0.92, 0.92, 0.92);
        let stripe = stripe_of(white, header, 0.35);
        assert!(
            (stripe.r - 0.92).abs() < 1e-6
                && (stripe.g - 0.92).abs() < 1e-6
                && (stripe.b - 0.92).abs() < 1e-6,
            "a strong stripe must land on the header, not past it"
        );
    }

    /// The other half of the rule: a subtle factor must stay strictly lighter than the header — in the header's own colour it would read as a stray header row — and it stops at a third of the background-to-header distance, which keeps its lightness step small on light backgrounds.
    #[test]
    fn subtle_stripes_stay_lighter_than_the_header() {
        let white = Color::from_rgb(1.0, 1.0, 1.0);
        let header = Color::from_rgb(0.92, 0.92, 0.92);
        let stripe = stripe_of(white, header, 0.12);
        assert!(
            stripe.r > 0.92 && stripe.g > 0.92 && stripe.b > 0.92,
            "a subtle stripe must stay lighter than the header"
        );
        let third = 1.0 - (1.0 - 0.92) / 3.0;
        assert!(
            (stripe.r - third).abs() < 1e-3
                && (stripe.g - third).abs() < 1e-3
                && (stripe.b - third).abs() < 1e-3,
            "a capped subtle stripe sits a third of the way to the header"
        );
    }

    /// A header lighter than the background (the usual dark-theme band) leaves a darkening stripe unbounded.
    #[test]
    fn a_lighter_header_bounds_nothing() {
        let dark = Color::from_rgb(0.16, 0.17, 0.18);
        let header = Color::from_rgb(0.2, 0.21, 0.22);
        let stripe = stripe_of(dark, header, 0.35);
        assert!(
            stripe.r < dark.r * 0.66 && stripe.g < dark.g * 0.66,
            "the full factor must still apply"
        );
    }
}
