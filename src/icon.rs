//! SVG glyphs of the widget: one cached [`svg::Handle`] per glyph, the glyph constants, and the [`RowIcons`] hook with its built-in [`TablerIcons`] implementation.
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

/// Wrap a glyph's inner path data in a complete `<svg>` document, respecting its paint metadata: Tabler outline icons declare `fill="none"` with a 2-px round stroke, while glyphs that carry no metadata at all (Boxicons) are solid silhouettes with the detail cut out by the fill rule, so they default to `fill="currentColor"` with `fill-rule="evenodd"` — a bare path with neither fill nor stroke would be invisible otherwise.
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
pub(crate) fn glyph<'a>(
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
pub(crate) fn natural<'a>(icon: icondata::Icon, size: f32) -> Svg<'a, Theme>
where
    Theme: iced_widget::svg::Catalog,
{
    Svg::new(handle(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(|_, _| Style { color: None })
}

// --- glyphs used by the widget: the three selection boxes from Boxicons (MIT, © Aniket Suvarna, https://boxicons.com), everything else Tabler (MIT, © 1000+ contributors, https://tabler.io/icons) unless custom ---

/// A Boxicons checkbox with its viewBox cropped to a 1-unit margin around the square (5–19 of 24), so the box fills most of the glyph instead of 14/24 of it.
const fn boxed(icon: icondata::Icon) -> icondata_core::IconData {
    icondata_core::IconData { view_box: Some("4 4 16 16"), ..*icon }
}

/// Unticked box: Boxicons' regular checkbox, an outlined square.
pub const SQUARE_OUTLINE: icondata::Icon = &boxed(icondata::BiCheckboxRegular);

/// Ticked box: Boxicons' solid checked checkbox (filled square with the checkmark cut out).
pub const CHECKBOX: icondata::Icon = &boxed(icondata::BiCheckboxCheckedSolid);

/// The partial marker: Boxicons' solid minus checkbox (filled square with a bar cut out), Material's indeterminate state.
pub const SQUARE_MINUS: icondata::Icon = &boxed(icondata::BiCheckboxMinusSolid);
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

// --- the row-icon hook an app configures on the cabinet ---

use crate::browser::node::Node;

/// Chooses the file and folder glyphs shown on the rows; see [`icons`](crate::Cabinet::icons).
///
/// The built-in default is [`TablerIcons`]. An app with its own icon theme implements this and hands it to the cabinet; the demo wires the same up to a dropdown.
pub trait RowIcons {
    /// The folder row's glyph; `open` when its branch is expanded (tree mode).
    fn folder(&self, node: &Node, open: bool) -> icondata::Icon;

    /// The file row's glyph for this entry — pick by extension or MIME, whatever your theme keys on.
    fn file(&self, node: &Node) -> icondata::Icon;

    /// Whether glyphs are repainted in the widget's colours ([`folder`](crate::Style::folder), [`text`](crate::Style::text)). `true` suits monochrome packs (Tabler); `false` keeps a multicolour pack's own fills (Material Icon Theme).
    fn tinted(&self) -> bool {
        true
    }
}

/// The built-in [`RowIcons`]: Tabler folder / open-folder / file, tinted like the rest of the widget.
pub struct TablerIcons;

impl RowIcons for TablerIcons {
    fn folder(&self, _node: &Node, open: bool) -> icondata::Icon {
        if open { FOLDER_OPEN } else { FOLDER }
    }

    fn file(&self, _node: &Node) -> icondata::Icon {
        FILE
    }
}
