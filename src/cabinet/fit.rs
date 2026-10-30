//! Fitting the columns to the cabinet's width: every value column keeps at least its [`min_width`](crate::Column::min_width), the Name column gives way down to a floor, and only where even that does not fit are the rightmost value columns left out. While every column's share of the width is above its minimum, the shares stay the columns' own [`ColumnWidth`]s, so a wide cabinet lays out exactly as without fitting.

use crate::column::ColumnWidth;

/// What the Name column may shrink to before a value column is left out, in multiples of the text size: room for the glyph and a few characters.
pub(super) const NAME_FLOOR: f32 = 5.0;

/// One visible column as the fitting sees it, the Name column first.
#[derive(Debug, Clone, Copy)]
pub(super) struct Want {
    pub width: ColumnWidth,
    /// Narrowest readable width in pixels, cell padding included.
    pub min: f32,
}

/// The width each column gets, in the order of `wants`; `None` for a column left out. `available` is what the cells may share: the row without its edges, box cell and rules. `floor` is the Name column's [`NAME_FLOOR`] in pixels, padding included. `rule` is the width a column's rule takes (0 without vertical separators), freed along with a column that is left out.
pub(super) fn fit(wants: &[Want], available: f32, floor: f32, rule: f32) -> Vec<Option<ColumnWidth>> {
    let mut shown = wants.len();
    // Leave out value columns from the right until their minimums and the Name floor fit.
    while shown > 1 {
        let needed: f32 = floor.min(wants[0].min) + wants[1..shown].iter().map(|w| w.min).sum::<f32>();
        if needed <= available + rule * (wants.len() - shown) as f32 {
            break;
        }
        shown -= 1;
    }
    let available = available + rule * (wants.len() - shown) as f32;
    let mut result: Vec<Option<ColumnWidth>> = wants.iter().map(|w| Some(w.width)).collect();
    for slot in &mut result[shown..] {
        *slot = None;
    }

    // Columns pinned to a pixel width so far: fixed ones from the start, value columns whose share fell below their minimum later.
    let mut pinned: Vec<Option<f32>> = wants[..shown]
        .iter()
        .map(|w| match w.width {
            ColumnWidth::Fixed(px) => Some(px),
            _ => None,
        })
        .collect();
    let portion = |w: &Want| match w.width {
        ColumnWidth::Fill => 1.0,
        ColumnWidth::Portion(p) => f32::from(p),
        ColumnWidth::Fixed(_) => 0.0,
    };
    loop {
        let rest = available - pinned.iter().flatten().sum::<f32>();
        let portions: f32 = wants[..shown].iter().zip(&pinned).filter(|(_, p)| p.is_none()).map(|(w, _)| portion(w)).sum();
        // The first value column whose share would fall below its minimum is pinned at the minimum; then the rest is shared again.
        let short = (1..shown).find(|&i| pinned[i].is_none() && portions > 0.0 && rest * portion(&wants[i]) / portions < wants[i].min);
        match short {
            Some(i) => pinned[i] = Some(wants[i].min),
            None => break,
        }
    }
    if pinned[1..].iter().any(Option::is_some) {
        for (i, px) in pinned.iter().enumerate().skip(1) {
            if let Some(px) = px {
                result[i] = Some(ColumnWidth::Fixed(*px));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const EM: f32 = 14.0;
    const PAD: f32 = 16.0;

    fn name() -> Want {
        Want { width: ColumnWidth::Portion(4), min: 10.0 * EM + PAD }
    }
    fn modified() -> Want {
        Want { width: ColumnWidth::Portion(2), min: 10.5 * EM + PAD }
    }
    fn size() -> Want {
        Want { width: ColumnWidth::Portion(2), min: 5.5 * EM + PAD }
    }
    const FLOOR: f32 = NAME_FLOOR * EM + PAD;

    /// Wide enough for every share: the columns keep their own widths.
    #[test]
    fn wide_cabinets_keep_the_columns_own_widths() {
        let fitted = fit(&[name(), modified(), size()], 1200.0, FLOOR, 0.0);
        assert!(matches!(fitted[..], [Some(ColumnWidth::Portion(4)), Some(ColumnWidth::Portion(2)), Some(ColumnWidth::Portion(2))]));
    }

    /// A phone picker: the date's third of 300 px would cut it, so it keeps its minimum and the name takes the rest.
    #[test]
    fn a_short_share_is_pinned_at_the_minimum() {
        let fitted = fit(&[name(), modified()], 300.0, FLOOR, 0.0);
        let Some(ColumnWidth::Fixed(px)) = fitted[1] else { panic!("{fitted:?}") };
        assert_eq!(px, modified().min);
        assert!(matches!(fitted[0], Some(ColumnWidth::Portion(4))));
    }

    /// Too narrow for every minimum: the rightmost column goes first, the date stays.
    #[test]
    fn the_rightmost_column_is_left_out_first() {
        let fitted = fit(&[name(), modified(), size()], 300.0, FLOOR, 0.0);
        assert!(fitted[1].is_some());
        assert!(fitted[2].is_none());
    }

    /// The Name column alone always stays.
    #[test]
    fn the_name_column_always_stays() {
        let fitted = fit(&[name(), modified()], 50.0, FLOOR, 0.0);
        assert!(fitted[0].is_some());
        assert!(fitted[1].is_none());
    }

    /// A rule freed by a left-out column counts towards the room of the others.
    #[test]
    fn freed_rules_count() {
        let needed = FLOOR + modified().min;
        let fitted = fit(&[name(), modified(), size()], needed - 1.0, FLOOR, 1.0);
        assert!(fitted[1].is_some());
    }
}
