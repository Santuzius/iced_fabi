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

/// Wrap a glyph's inner path data in a complete `<svg>` document, respecting its paint metadata: Tabler outline icons declare `fill="none"` with a 2-px round stroke, filled ones paint the path itself.
fn svg_document(glyph: icondata::Icon) -> String {
    let view_box = glyph.view_box.unwrap_or("0 0 24 24");
    let fill = glyph.fill.unwrap_or("none");
    let stroke = glyph.stroke.unwrap_or("none");
    let stroke_width = glyph.stroke_width.unwrap_or("2");
    let stroke_linecap = glyph.stroke_linecap.unwrap_or("round");
    let stroke_linejoin = glyph.stroke_linejoin.unwrap_or("round");
    let data = glyph.data;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" fill="{fill}" stroke="{stroke}" stroke-width="{stroke_width}" stroke-linecap="{stroke_linecap}" stroke-linejoin="{stroke_linejoin}">{data}</svg>"##,
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

// --- glyphs used by the widget, all Tabler (MIT, © 1000+ contributors, https://tabler.io/icons) unless custom ---

pub const SQUARE_OUTLINE: icondata::Icon = icondata::TbSquareOutline;

/// The partial marker: the selection box plus a filled centre dot — Material's indeterminate state, which no single Tabler glyph covers (box outline and dot are separate glyphs).
pub const SQUARE_DOT: icondata::Icon = &icondata_core::IconData {
    style: None,
    x: None,
    y: None,
    width: Some("24"),
    height: Some("24"),
    view_box: Some("0 0 24 24"),
    stroke_linecap: Some("round"),
    stroke_linejoin: Some("round"),
    stroke_width: Some("2"),
    stroke: Some("currentColor"),
    fill: Some("none"),
    data: concat!(
        // Box from TbSquareOutline (same path verbatim).
        r#"<path d="M3 3m0 2a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v14a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2z" />"#,
        // Filled dot, radius 2.5 at the centre, drawn as a path because resvg fills circles without a stroke only if they carry the fill explicitly.
        r#"<path d="M12 12m-2.5 0a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0 -5 0" fill="currentColor" stroke="none" />"#,
    ),
};

pub const CHECKBOX: icondata::Icon = icondata::TbCheckboxOutline;
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
