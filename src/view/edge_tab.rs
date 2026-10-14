//! The edge tab (EDGE-02, EDGE-03, MRK-15): the look and placement shared
//! by a gate's edge arrow and the frame parent's tab. A tab has a
//! ground-colour fill and a neutral outline `button_stroke_px` wide, and
//! holds a chevron turned towards what it points at, then its content (a
//! label, or a marker and a label). The chevron sits on the pinned point
//! at the map area's edge and the content on its inner side, so the label
//! always reads towards the middle of the map; the tab then slides along
//! the edge as needed to lie whole inside the map area, never clipped.

use bevy::prelude::*;

use super::colors;
use super::fonts::Fonts;
use super::layers;
use super::screen::MapArea;

/// Half the chevron's box. The same footprint as a gate's own marker, so
/// the hand-over between the two doesn't change size.
pub const ARROW_HALF_PX: f32 = 5.5;
/// The chevron glyph is drawn a little larger than its box so it reads as
/// a solid head rather than a caret.
pub const ARROW_GLYPH_SCALE: f32 = 2.2;
pub const LABEL_FONT_PX: f32 = 12.0;
/// Space inside the tab's outline, across and along its row.
const TAB_PAD_PX: f32 = 4.0;
const TAB_PAD_ALONG_PX: f32 = 6.0;
/// Space between the chevron and the content, and inside the content.
const TAB_GAP_PX: f32 = 5.0;
/// The tab never comes closer than this to the map area's edge.
const TAB_INSET_PX: f32 = 4.0;

/// The chevron in a tab, turned towards what the tab points at.
#[derive(Component)]
pub struct TabChevron;

/// Which end of a tab its chevron is at. A tab is always one row, so its
/// label never sits on a second line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeSide {
    /// The chevron first, the label to its right.
    Left,
    /// The chevron last, the label to its left.
    Right,
}

/// EDGE-02: which end of the tab the chevron goes at: the end nearer the
/// side of the map area the point is on, so the label reaches towards the
/// middle. On the top and bottom edges that is the nearer half.
pub fn edge_side(at: Vec2, area: &MapArea) -> EdgeSide {
    if at.x < area.offset.x + area.size.x / 2.0 {
        EdgeSide::Left
    } else {
        EdgeSide::Right
    }
}

/// EDGE-02: lay the tab out as one row with the chevron at its outer end
/// and the content on its inner side.
pub fn tab_direction(side: EdgeSide) -> FlexDirection {
    match side {
        EdgeSide::Left => FlexDirection::Row,
        EdgeSide::Right => FlexDirection::RowReverse,
    }
}

/// EDGE-02: the tab's top left, for a tab of `size` whose chevron should
/// sit on the pinned point `at`, moved along the edge so the whole tab
/// lies inside the map area. `stroke` is the outline width.
pub fn tab_origin(at: Vec2, size: Vec2, side: EdgeSide, stroke: f32, area: &MapArea) -> Vec2 {
    let across = TAB_PAD_ALONG_PX + stroke + ARROW_HALF_PX;
    let origin = match side {
        EdgeSide::Left => Vec2::new(at.x - across, at.y - size.y / 2.0),
        EdgeSide::Right => Vec2::new(at.x + across - size.x, at.y - size.y / 2.0),
    };
    let low = area.offset + Vec2::splat(TAB_INSET_PX);
    let high = (area.offset + area.size - size - Vec2::splat(TAB_INSET_PX)).max(low);
    origin.clamp(low, high)
}

/// EDGE-03: a hidden tab: ground fill, neutral outline `stroke` wide, a
/// chevron, and a content row filled by `content`. Nothing in it is a
/// pick target.
pub fn spawn_tab<'a>(
    commands: &'a mut Commands,
    fonts: &Fonts,
    stroke: f32,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) -> EntityCommands<'a> {
    let mut tab = commands.spawn((
        GlobalZIndex(layers::EDGE),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            align_items: AlignItems::Center,
            row_gap: px(TAB_GAP_PX),
            column_gap: px(TAB_GAP_PX),
            padding: UiRect::axes(px(TAB_PAD_ALONG_PX), px(TAB_PAD_PX)),
            border: UiRect::all(px(stroke)),
            ..default()
        },
        BackgroundColor(colors::GROUND),
        BorderColor::all(colors::NEUTRAL),
    ));
    tab.with_children(|tab| {
        tab.spawn((
            TabChevron,
            Pickable::IGNORE,
            Node {
                width: px(ARROW_HALF_PX * 2.0),
                height: px(ARROW_HALF_PX * 2.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            UiTransform::IDENTITY,
        ))
        .with_children(|chevron| {
            // Points up unturned; turned each frame towards the target.
            chevron.spawn((
                Text::new("^"),
                fonts.label(ARROW_HALF_PX * ARROW_GLYPH_SCALE),
                TextColor(colors::NEUTRAL),
                Pickable::IGNORE,
            ));
        });
        tab.spawn((
            Pickable::IGNORE,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TAB_GAP_PX),
                ..default()
            },
        ))
        .with_children(content);
    });
    tab
}

/// The text of a tab's label, in the neutral colour, never wrapped.
pub fn tab_label(fonts: &Fonts, text: &str) -> impl Bundle {
    (
        Pickable::IGNORE,
        Text::new(text),
        TextLayout::linebreak(LineBreak::NoWrap),
        fonts.label(LABEL_FONT_PX),
        TextColor(colors::NEUTRAL),
    )
}

/// EDGE-02: show a tab with its chevron on `at`, laid out for the edge it
/// is on and kept whole inside the map area.
pub fn place_tab(node: &mut Node, computed: &ComputedNode, at: Vec2, stroke: f32, area: &MapArea) {
    let side = edge_side(at, area);
    let size = computed.size() * computed.inverse_scale_factor();
    let origin = tab_origin(at, size, side, stroke, area);
    node.display = Display::Flex;
    node.flex_direction = tab_direction(side);
    node.left = px(origin.x);
    node.top = px(origin.y);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> MapArea {
        MapArea::of(Vec2::new(1600.0, 900.0))
    }

    /// EDGE-02: the chevron goes at the end nearer the point's side, on
    /// the top and bottom edges too.
    #[test]
    fn the_chevron_goes_at_the_outer_end() {
        let area = area();
        let mid = area.offset + area.size / 2.0;
        let side = |d: Vec2| edge_side(mid + d, &area);
        assert_eq!(side(Vec2::new(-780.0, 10.0)), EdgeSide::Left);
        assert_eq!(side(Vec2::new(780.0, -10.0)), EdgeSide::Right);
        assert_eq!(side(Vec2::new(30.0, -400.0)), EdgeSide::Right);
        assert_eq!(side(Vec2::new(-30.0, 400.0)), EdgeSide::Left);
    }

    /// EDGE-02: the label is on the inner side of the arrow: the chevron
    /// comes first from the edge it is pinned to.
    #[test]
    fn the_label_sits_on_the_inner_side_of_the_arrow() {
        let area = area();
        let size = Vec2::new(80.0, 22.0);
        let mid = area.offset + area.size / 2.0;
        let right = Vec2::new(area.offset.x + area.size.x - 28.0, mid.y);
        let origin = tab_origin(right, size, EdgeSide::Right, 1.0, &area);
        assert_eq!(tab_direction(EdgeSide::Right), FlexDirection::RowReverse);
        // The chevron is at the tab's right end, on the pinned point, and
        // the label reaches inwards to its left.
        assert!((origin.x + size.x - (6.0 + 1.0 + ARROW_HALF_PX) - right.x).abs() < 1e-3);
        let left = Vec2::new(area.offset.x + 28.0, mid.y);
        let origin = tab_origin(left, size, EdgeSide::Left, 1.0, &area);
        assert_eq!(tab_direction(EdgeSide::Left), FlexDirection::Row);
        assert!((origin.x + 6.0 + 1.0 + ARROW_HALF_PX - left.x).abs() < 1e-3);
    }

    /// EDGE-02 / MRK-15: the whole tab stays inside the map area, however
    /// near a corner its point is, so its label is never clipped.
    #[test]
    fn the_tab_lies_whole_inside_the_map_area() {
        let area = area();
        let size = Vec2::new(120.0, 24.0);
        let corners = [
            area.offset,
            area.offset + area.size,
            area.offset + Vec2::new(area.size.x, 0.0),
            area.offset + Vec2::new(0.0, area.size.y),
        ];
        for corner in corners {
            for side in [EdgeSide::Left, EdgeSide::Right] {
                let origin = tab_origin(corner, size, side, 1.0, &area);
                assert!(origin.cmpge(area.offset).all(), "{corner} {side:?}");
                assert!(
                    (origin + size).cmple(area.offset + area.size).all(),
                    "{corner} {side:?}"
                );
            }
        }
    }
}
