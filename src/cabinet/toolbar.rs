//! The toolbar: breadcrumbs, the mode switch and the tool groups, including the rule that separators only ever divide two tools.

use iced_widget::{
    Space, button, container,
    core::{Alignment, Element, Font, Length, Theme},
    row,
};

use super::paint::{BAR_HEIGHT, ColourField, EDGE};
use super::{ActionPlacement, Cabinet, ToolbarItem};
use crate::browser::{Mode, NodeId};
use crate::icon;

/// Drop separators that do not divide two tools: leading, doubled-up and trailing ones fall away on their own, so a group whose tools were all left out takes its divider with them. Pure, so the collapsing rule is testable without a widget.
fn prune_separators<'a, Message: Clone>(
    items: &'a [ToolbarItem<'a, Message>],
) -> Vec<&'a ToolbarItem<'a, Message>> {
    let mut kept: Vec<&ToolbarItem<'a, Message>> = Vec::new();
    for item in items {
        match item {
            ToolbarItem::Tool(_) => kept.push(item),
            ToolbarItem::Separator => {
                if matches!(kept.last(), Some(ToolbarItem::Tool(_))) {
                    kept.push(item);
                }
            }
        }
    }
    if matches!(kept.last(), Some(ToolbarItem::Separator)) {
        kept.pop();
    }
    kept
}

impl<'a, Message: Clone + 'a, Renderer> Cabinet<'a, Message, Renderer>
where
    Renderer: iced_widget::core::text::Renderer<Font = Font>
        + iced_widget::core::svg::Renderer
        + iced_widget::core::Renderer
        + 'a,
{
    /// Breadcrumbs behind the mode switch and a divider on the left, the tool groups on the right.
    pub(super) fn toolbar(&self) -> Element<'a, Message, Theme, Renderer> {
        let mut crumbs = row![].spacing(2).align_y(Alignment::Center);
        // Tree mode is positionless (the rows are the position), so its breadcrumb stays at the root.
        let path: &[NodeId] = match self.browser.mode() {
            Mode::Tree => &self.browser.path()[..1],
            Mode::Browse => self.browser.path(),
        };
        for (depth, id) in path.iter().enumerate() {
            if depth > 0 {
                crumbs = crumbs.push(self.glyph(icon::CHEVRON_RIGHT, 14.0, ColourField::Muted));
            }
            let name = self
                .browser
                .node(*id)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            let (label, field) = if depth + 1 == path.len() {
                (self.strong(name), ColourField::Text)
            } else {
                (self.label(name), ColourField::Muted)
            };
            crumbs = crumbs.push(
                button(label)
                    .padding([4, 6])
                    .style(self.flat(self.colour(field)))
                    .on_press((self.on_event)(crate::Event::CrumbPressed(*id))),
            );
        }
        let mut toolbar = row![].spacing(2).align_y(Alignment::Center);
        // Separators only ever divide two tools: collapsed groups take their divider with them.
        let items = prune_separators(&self.tools);
        for item in &items {
            toolbar = toolbar.push(match item {
                ToolbarItem::Separator => self.divider(),
                ToolbarItem::Tool(tool) => {
                    let enabled = tool.enabled(self.browser);
                    let field = if enabled {
                        ColourField::Text
                    } else {
                        ColourField::Muted
                    };
                    let button = self.icon_button(tool.glyph(), 18.0, 30.0, field);
                    if enabled {
                        button.on_press(tool.message(self.browser)).into()
                    } else {
                        button.into()
                    }
                }
            });
        }
        let has_tools = items
            .iter()
            .any(|item| matches!(item, ToolbarItem::Tool(_)));
        let mut mode_switch = row![].spacing(2).align_y(Alignment::Center);
        if self.mode_toggle {
            let mode_glyph = match self.browser.mode() {
                Mode::Browse => icon::TREE,
                Mode::Tree => icon::LIST,
            };
            mode_switch = mode_switch.push(
                self.icon_button(mode_glyph, 18.0, 30.0, ColourField::Text)
                    .on_press((self.on_event)(crate::Event::ModeSwitchPressed)),
            );
            // The mode switch leads the bar and is divided from the breadcrumbs; the tools keep their own leading divider.
            mode_switch = mode_switch.push(self.divider());
        }
        // The toolbar variant of the action button lands after the tool groups.
        if let Some(action) = self
            .action
            .as_ref()
            .filter(|a| a.placement == ActionPlacement::ToolbarEnd)
        {
            if has_tools || self.mode_toggle {
                toolbar = toolbar.push(self.divider());
            }
            toolbar = toolbar.push(self.action_button_element(action, ActionPlacement::ToolbarEnd));
        }
        row![
            mode_switch,
            crumbs,
            Space::new().width(Length::Fill),
            toolbar
        ]
        .spacing(8)
        .height(Length::Fixed(BAR_HEIGHT))
        .padding([0.0, EDGE])
        .align_y(Alignment::Center)
        .into()
    }

    /// The upright line between two tool groups.
    fn divider(&self) -> Element<'a, Message, Theme, Renderer> {
        container(self.vrule())
            .height(Length::Fixed(20.0))
            .padding([0, 6])
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::prune_separators;
    use crate::cabinet::ToolbarItem;

    fn tool_item() -> ToolbarItem<'static, ()> {
        ToolbarItem::Tool(Box::new(crate::tool(icondata::TbCopyOutline, |_| ())))
    }

    fn separator() -> ToolbarItem<'static, ()> {
        ToolbarItem::Separator
    }

    /// Assemble a toolbar list: `tools` groups of `per_group` tools each, one separator between the groups.
    fn groups(tools_per_group: &[usize]) -> Vec<ToolbarItem<'static, ()>> {
        let mut items = Vec::new();
        for (group, count) in tools_per_group.iter().enumerate() {
            if group > 0 {
                items.push(separator());
            }
            for _ in 0..*count {
                items.push(tool_item());
            }
        }
        items
    }

    /// How many tools and dividers survive the collapsing rule.
    fn kept(items: &[ToolbarItem<'static, ()>]) -> (usize, usize) {
        let kept = prune_separators(items);
        (
            kept.iter()
                .filter(|i| matches!(i, ToolbarItem::Tool(_)))
                .count(),
            kept.iter()
                .filter(|i| matches!(i, ToolbarItem::Separator))
                .count(),
        )
    }

    #[test]
    fn leading_doubled_and_trailing_separators_drop_out() {
        // ─ A ─ ─ B ─ : leading, doubled and trailing separators collapse into the one real divider.
        let mut items = vec![separator()];
        items.extend(groups(&[1, 1]));
        items.insert(2, separator());
        items.push(separator());
        assert_eq!(kept(&items), (2, 1));
    }

    #[test]
    fn an_empty_group_takes_its_divider_with_it() {
        // ─ A ─ : a lone group between separators keeps only the tool.
        let items = vec![separator(), tool_item(), separator()];
        assert_eq!(kept(&items), (1, 0));
        // A ─ ─ B : two separators between the groups leave one divider.
        let items = vec![tool_item(), separator(), separator(), tool_item()];
        assert_eq!(kept(&items), (2, 1));
    }

    #[test]
    fn groups_of_tools_keep_one_divider_each() {
        // A B ─ C D: one divider, two tools each side.
        assert_eq!(kept(&groups(&[2, 2])), (4, 1));
        // A ─ ─ B: the empty middle group of three leaves one divider between A and B.
        assert_eq!(kept(&groups(&[1, 0, 1])), (2, 1));
    }
}
