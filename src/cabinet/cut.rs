//! [`Cut`]: a label cut to the width its cell actually got.

use std::rc::Rc;

use iced_widget::core::{
    Color, Element, Font, Length, Pixels, Point, Rectangle, Size, Theme, Widget, alignment, layout,
    mouse, renderer,
    text::{self as core_text, Paragraph as _},
    widget::{Tree, tree},
};
use iced_widget::text;

/// The layout result a [`Cut`] caches for its draw pass.
#[derive(Default)]
struct CutState {
    text: String,
}

/// A label that is cut to the width its cell actually got: iced's `Text` neither clips nor ellipsizes (0.14). At layout time the final width is already known (flex columns are resolved before their children are laid out), so the full label is measured exactly — through the renderer's own paragraph implementation, i.e. the same text that gets drawn — and when it does not fit, the longest prefix that fits together with the ellipsis is handed to `Text`. No caller-side width bookkeeping; labels whose cell is wide enough stay whole. It fills its cell and centres the text vertically.
pub(crate) struct Cut<'c, Message, Renderer: core_text::Renderer> {
    label: String,
    colour: Rc<dyn Fn(&Theme) -> Color + 'c>,
    size: Option<Pixels>,
    font: Option<Font>,
    child: Element<'c, Message, Theme, Renderer>,
}

impl<'c, Message, Renderer: core_text::Renderer<Font = Font> + 'c> Cut<'c, Message, Renderer> {
    pub(crate) fn new(label: String, colour: Rc<dyn Fn(&Theme) -> Color + 'c>) -> Self {
        let child: text::Text<'c, Theme, Renderer> = text(label.clone());
        Cut {
            label,
            colour,
            size: None,
            font: None,
            child: Element::new(child),
        }
    }

    /// The text size; `None` is the renderer's default.
    pub(crate) fn size(mut self, size: Option<Pixels>) -> Self {
        self.size = size;
        self
    }

    /// The font; `None` is the renderer's default.
    pub(crate) fn font(mut self, font: Option<Font>) -> Self {
        self.font = font;
        self
    }
}

impl<'c, Message, Renderer> Widget<Message, Theme, Renderer> for Cut<'c, Message, Renderer>
where
    Renderer: core_text::Renderer<Font = Font> + 'c,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.child)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.child));
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<CutState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(CutState::default())
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<CutState>();
        // Measure as one unwrapped line at the label's size and font; the Text widget we delegate to sizes itself the same way.
        let size = self.size.unwrap_or_else(|| renderer.default_size());
        let font = self.font.unwrap_or_else(|| renderer.default_font());
        let mut paragraph = core_text::paragraph::Plain::<Renderer::Paragraph>::default();
        let avail = limits.max().width;
        state.text = self.label.clone();
        if avail.is_finite() && !self.label.is_empty() {
            let mut measured = |content: &str| {
                paragraph.update(core_text::Text {
                    content,
                    bounds: Size::new(f32::INFINITY, f32::INFINITY),
                    size,
                    line_height: core_text::LineHeight::default(),
                    font,
                    align_x: core_text::Alignment::Default,
                    align_y: alignment::Vertical::Top,
                    shaping: core_text::Shaping::default(),
                    wrapping: core_text::Wrapping::None,
                });
                paragraph.raw().min_width()
            };
            if measured(&self.label) > avail {
                // Binary search the longest prefix that fits with the ellipsis glued on, then trim what would sit right before it.
                let mut best = 0usize;
                let mut low = 1usize;
                let mut high = self.label.chars().count() - 1;
                while low <= high {
                    let keep = (low + high) / 2;
                    let prefix: String = self.label.chars().take(keep).collect();
                    if measured(&format!("{prefix}…")) <= avail {
                        best = keep;
                        low = keep + 1;
                    } else {
                        high = keep - 1;
                    }
                }
                let mut cut: String = self.label.chars().take(best).collect();
                while cut.ends_with(' ') || cut.ends_with('/') {
                    cut.pop();
                }
                cut.push('…');
                state.text = cut;
            }
        }
        // Hand the chosen prefix to the Text child (it diffs cheaply, and its paragraph diffs cheaply when the text did not change).
        let colour = Rc::clone(&self.colour);
        let inner_text: text::Text<'c, Theme, Renderer> = text::Text::new(state.text.clone())
            .size(size)
            .font(font)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(alignment::Vertical::Center)
            .wrapping(core_text::Wrapping::None)
            .style(move |theme: &Theme| text::Style {
                color: Some(colour(theme)),
            });
        self.child = Element::new(inner_text);
        let inner = self
            .child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(inner.size(), vec![inner])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let inner = layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y));
        self.child.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            defaults,
            layout::Layout::new(&inner),
            cursor,
            viewport,
        );
    }
}

impl<'c, Message: 'c, Renderer> From<Cut<'c, Message, Renderer>>
    for Element<'c, Message, Theme, Renderer>
where
    Renderer: core_text::Renderer<Font = Font> + 'c,
{
    fn from(cut: Cut<'c, Message, Renderer>) -> Self {
        Element::new(cut)
    }
}
