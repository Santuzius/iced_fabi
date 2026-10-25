//! SVG glyphs from the Tabler icon pack (via `icondata`), one cached [`svg::Handle`] per glyph.
//!
//! Each glyph's SVG document is built once from the icon's raw path data and its handle kept for the process lifetime (keyed by the static's address); colour comes from the svg widget's tint filter, so light/dark switches never rebuild a handle. Same approach syncative uses for its status icons.

use std::{cell::RefCell, collections::HashMap};

use iced_widget::{
    Svg,
    core::{Color, Length, Theme, svg as core_svg},
    svg::{self, Style},
};

thread_local! {
    static HANDLES: RefCell<HashMap<usize, core_svg::Handle>> = RefCell::new(HashMap::new());
}

fn handle(glyph: icondata::Icon) -> core_svg::Handle {
    let key = std::ptr::from_ref(glyph) as usize;
    HANDLES.with_borrow_mut(|cache| {
        cache
            .entry(key)
            .or_insert_with(|| svg::Handle::from_memory(svg_document(glyph).into_bytes()))
            .clone()
    })
}

/// Wrap a glyph's inner path data in a complete `<svg>` document, respecting its paint metadata: Tabler outline icons declare `fill="none"` with a 2-px round stroke, while glyphs that carry no metadata at all (IcoMoon) are solid silhouettes with the detail cut out by the fill rule, so they default to `fill="currentColor"` with `fill-rule="evenodd"` — a bare path with neither fill nor stroke would be invisible otherwise.
fn svg_document(glyph: icondata::Icon) -> String {
    let view_box = glyph.view_box.unwrap_or("0 0 24 24");
    let (fill, fill_rule) = match (glyph.fill, glyph.stroke) {
        (Some(fill), _) => (fill.to_string(), String::new()),
        (None, Some(_)) => (String::from("none"), String::new()),
        (None, None) => (
            String::from("currentColor"),
            String::from(r#" fill-rule="evenodd""#),
        ),
    };
    let stroke = glyph.stroke.unwrap_or("none");
    let stroke_width = glyph.stroke_width.unwrap_or("2");
    let stroke_linecap = glyph.stroke_linecap.unwrap_or("round");
    let stroke_linejoin = glyph.stroke_linejoin.unwrap_or("round");
    let data = glyph.data;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" fill="{fill}"{fill_rule} stroke="{stroke}" stroke-width="{stroke_width}" stroke-linecap="{stroke_linecap}" stroke-linejoin="{stroke_linejoin}">{data}</svg>"##,
    )
}

/// Glyph at `size` px, tinted by `colour`.
pub fn glyph<'a>(
    icon: icondata::Icon,
    size: f32,
    colour: std::rc::Rc<dyn Fn(&Theme) -> Color + 'a>,
) -> Svg<'a, Theme>
where
    Theme: iced_widget::svg::Catalog,
{
    Svg::new(handle(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |theme: &Theme, _| Style {
            color: Some(colour(theme)),
        })
}

/// Glyph at `size` px keeping the SVG's own colours — for multicolour icon themes (tiny-skia's style colour is a post-render filter, so `None` leaves the artwork untouched).
pub fn natural<'a>(icon: icondata::Icon, size: f32) -> Svg<'a, Theme>
where
    Theme: iced_widget::svg::Catalog,
{
    Svg::new(handle(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(|_, _| Style { color: None })
}

// --- glyphs used by the widget: the three selection boxes from IcoMoon Free (https://github.com/Keyamoon/IcoMoon-Free; artwork GPL-3.0-or-later © Keyamoon, even though the `icondata_im` crate metadata says MIT — flag this to any licensee), everything else Tabler (MIT, © 1000+ contributors, https://tabler.io/icons) unless custom ---

/// Unticked box: IcoMoon's unchecked checkbox (filled square with an evenodd-cut interior, so it reads as an outline but is one solid shape).
pub const SQUARE_OUTLINE: icondata::Icon = icondata::ImCheckboxUnchecked;

/// Ticked box: IcoMoon's checked checkbox (filled square with the checkmark cut out).
pub const CHECKBOX: icondata::Icon = icondata::ImCheckboxChecked;

/// The partial marker: IcoMoon's checkbox box with its checkmark cut replaced by a rounded horizontal bar — Material's indeterminate state. IcoMoon Free ships no indeterminate glyph, so the box is `ImCheckboxChecked`'s outer subpath verbatim (byte-identical to `ImCheckboxUnchecked`'s) and only the cut changes: a Material-style bar of 10/24 of the box width and 2/24 thickness scaled into the 16 viewBox, with 0.75 round ends. Both subpaths sit in one `<path>`, so the document-level `fill-rule="evenodd"` (see `svg_document`) punches the bar out of the filled box exactly as the checkmark is cut — the glyph reads as a checked box in the accent colour with a dash instead of a tick.
pub const SQUARE_DOT: icondata::Icon = &icondata_core::IconData {
    style: None,
    x: None,
    y: None,
    width: Some("16"),
    height: Some("16"),
    view_box: Some("0 0 16 16"),
    stroke_linecap: None,
    stroke_linejoin: None,
    stroke_width: None,
    stroke: None,
    fill: None,
    data: concat!(
        // SVG path data derived from IcoMoon Free (GPL-3.0-or-later, © Keyamoon) https://github.com/Keyamoon/IcoMoon-Free
        // Box, copied verbatim from ImCheckboxChecked (first subpath).
        r#"<path d="M14 0h-12c-1.1 0-2 0.9-2 2v12c0 1.1 0.9 2 2 2h12c1.1 0 2-0.9 2-2v-12c0-1.1-0.9-2-2-2z"#,
        // Indeterminate bar cut: 6.7×1.5 centred at (8,8), radius-0.75 round ends.
        r#"M5.4 7.25h5.2a.75.75 0 0 1 0 1.5h-5.2a.75.75 0 0 1 0-1.5z" />"#,
    ),
};
pub const CHEVRON_RIGHT: icondata::Icon = icondata::TbChevronRightOutline;
pub const CHEVRON_DOWN: icondata::Icon = icondata::TbChevronDownOutline;
pub const FOLDER: icondata::Icon = icondata::TbFolderFilled;
/// Tree mode's expanded folder. (Tabler has no filled open-folder glyph, hence outline.)
pub const FOLDER_OPEN: icondata::Icon = icondata::TbFolderOpenOutline;
pub const FILE: icondata::Icon = icondata::TbFileOutline;
pub const LIST: icondata::Icon = icondata::TbListOutline;
pub const TREE: icondata::Icon = icondata::TbSitemapOutline;
pub const SORT_ASCENDING: icondata::Icon = icondata::TbSortAscendingOutline;
pub const SORT_DESCENDING: icondata::Icon = icondata::TbSortDescendingOutline;
pub const ERROR: icondata::Icon = icondata::TbAlertCircleOutline;
