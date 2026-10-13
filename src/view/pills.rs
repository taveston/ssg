//! The "+N" count pill beside a parent with folded satellites (GRP-05 to
//! GRP-07): the count, an outline in the colour of the most pressing
//! affiliation folded into it, and a blue dot if own ships are parked
//! there. What is folded,
//! and the flags, come from `fold.rs`; this module only draws them.

use bevy::prelude::*;

use super::colors;
use super::fold::FoldSummary;
use super::fonts::Fonts;
use super::hover;
use super::screen::ScreenPositions;
use super::selection::Selection;
use super::targets::GroupPillFor;
use crate::sim::orbit::Orbit;
use crate::{Boot, Stage};

/// GRP-05: where the pill sits relative to its parent's marker centre -
/// down and to the right, clear of the largest marker and its label.
const PILL_OFFSET_PX: Vec2 = Vec2::new(10.0, 8.0);
/// Half the pill's height: its left-middle is where the
/// frame brackets meet it (INS-06).
const PILL_HALF_HEIGHT_PX: f32 = 8.0;
/// The white ring on a pill that holds a selected satellite (INS-06), the
/// same as a marker's (MRK-01).
const RING_PX: f32 = 2.0;
const RING_OFFSET_PX: f32 = 2.0;

/// INS-06: where a folded satellite is shown - the left-middle of its
/// parent's pill, given the parent's screen position.
pub fn pill_anchor(parent_screen: Vec2) -> Vec2 {
    parent_screen + PILL_OFFSET_PX + Vec2::new(0.0, PILL_HALF_HEIGHT_PX)
}
/// The pill's own dark ground, a touch lighter than the map so it reads as
/// a chip over it.
const PILL_GROUND: Color = Color::srgba(0.07, 0.09, 0.11, 0.9);
const PILL_FONT_PX: f32 = 11.0;
const OWN_SHIP_DOT_PX: f32 = 5.0;

#[derive(Component)]
pub struct PillCountText;

/// GRP-06: the small blue dot on a pill, shown when own ships are parked
/// among the folded satellites.
#[derive(Component)]
pub struct PillOwnShipDot;

pub struct PillsPlugin;

impl Plugin for PillsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_pills.in_set(Boot::Overlay))
            .add_systems(Update, (update_pills, ring_pills).in_set(Stage::Overlay));
    }
}

/// GRP-05: a pill for every body that has satellites - which includes
/// moons with moons of their own, not just planets.
fn spawn_pills(
    mut commands: Commands,
    fonts: Res<Fonts>,
    satellites: Query<&ChildOf, With<Orbit>>,
) {
    let mut parents: Vec<Entity> = satellites.iter().map(ChildOf::parent).collect();
    parents.sort();
    parents.dedup();
    for parent in parents {
        commands
            .spawn((
                GroupPillFor(parent),
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    align_items: AlignItems::Center,
                    column_gap: px(4),
                    padding: UiRect::axes(px(5), px(1)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(PILL_GROUND),
                BorderColor::all(Color::NONE),
                Outline {
                    width: px(0),
                    offset: px(RING_OFFSET_PX),
                    color: Color::NONE,
                },
                children![
                    (
                        PillCountText,
                        Text::new(""),
                        fonts.mono(PILL_FONT_PX),
                        TextColor(colors::NEUTRAL)
                    ),
                    (
                        PillOwnShipDot,
                        Node {
                            width: px(OWN_SHIP_DOT_PX),
                            height: px(OWN_SHIP_DOT_PX),
                            display: Display::None,
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BackgroundColor(colors::OWN),
                    )
                ],
            ))
            // GRP-07: hover lists the folded satellites; click is the
            // parent's click.
            .observe(hover::on_pill_over)
            .observe(hover::on_pill_out)
            .observe(hover::on_pill_click);
    }
}

/// INS-06: a pill carries the white selection ring while a selected
/// object is folded into it.
fn ring_pills(
    selection: Res<Selection>,
    folds: Res<FoldSummary>,
    mut pills: Query<(&GroupPillFor, &mut Outline)>,
) {
    for (pill_for, mut outline) in &mut pills {
        let holds_selection = folds
            .0
            .get(&pill_for.0)
            .is_some_and(|group| group.members.iter().any(|&m| selection.is_selected(m)));
        outline.width = px(if holds_selection { RING_PX } else { 0.0 });
        outline.color = if holds_selection {
            colors::SELECTION_RING
        } else {
            Color::NONE
        };
    }
}

fn update_pills(
    screen: Res<ScreenPositions>,
    folds: Res<FoldSummary>,
    mut pills: Query<(&GroupPillFor, &mut Node, &mut BorderColor, &Children)>,
    mut counts: Query<&mut Text, With<PillCountText>>,
    mut dots: Query<&mut Node, (With<PillOwnShipDot>, Without<GroupPillFor>)>,
) {
    for (pill_for, mut node, mut border, children) in &mut pills {
        let parent = pill_for.0;
        let group = folds
            .0
            .get(&parent)
            .filter(|group| !group.members.is_empty());
        let pos = screen.get(parent).filter(|pos| screen.in_viewport(*pos));
        let (Some(group), Some(pos)) = (group, pos) else {
            node.display = Display::None;
            continue;
        };

        node.display = Display::Flex;
        node.left = px(pos.x + PILL_OFFSET_PX.x);
        node.top = px(pos.y + PILL_OFFSET_PX.y);
        // GRP-06: the outline in the most pressing affiliation's colour.
        border.set_all(group.pressing.map_or(Color::NONE, colors::symbol_color));

        for child in children.iter() {
            if let Ok(mut text) = counts.get_mut(child) {
                *text = Text::new(format!("+{}", group.members.len()));
            }
            // GRP-06: blue dot when own ships are parked there.
            if let Ok(mut dot) = dots.get_mut(child) {
                dot.display = if group.has_own_ship {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INS-06: a folded satellite is shown at its parent's pill, at the
    /// pill's left-middle, which follows the parent on screen.
    #[test]
    fn pill_anchor_is_the_pills_left_middle() {
        let parent = Vec2::new(400.0, 300.0);
        let anchor = pill_anchor(parent);
        assert_eq!(anchor.x, parent.x + PILL_OFFSET_PX.x);
        assert_eq!(anchor.y, parent.y + PILL_OFFSET_PX.y + PILL_HALF_HEIGHT_PX);
        assert_eq!(
            pill_anchor(parent + Vec2::X * 50.0),
            anchor + Vec2::X * 50.0
        );
    }
}
