//! The "+N" count pill beside a parent with folded satellites (GRP-05 to
//! GRP-07): the count, an outline in the colour of the most pressing
//! affiliation folded into it, and a blue dot if own ships are parked
//! there. What is folded, and the flags, come from `fold.rs`; this module
//! only draws them. The pill's node and face are shared with the identity
//! pill on a shared exposure ring (`ring_marks.rs`, MOT-14), which is
//! styled and coloured the same way.

use bevy::prelude::*;

use super::layers;

use super::colors;
use super::fold::{FoldSummary, PillFlags};
use super::fonts::Fonts;
use super::hover;
use super::markers::{MarkersPlaced, ring_outer_half};
use super::screen::ScreenPositions;
use super::selection::Selection;
use super::symbol_draw::{Tint, TintSlot};
use super::targets::{GroupPillFor, Hovered};
use super::tunables::Tunables;
use crate::sim::body::BodyClass;
use crate::sim::orbit::Orbit;
use crate::{Boot, Stage};

/// GRP-05: how far below its parent's marker centre the pill's top sits.
/// Sideways it sits just clear of the parent's hover ring.
const PILL_DROP_PX: f32 = 8.0;
/// Half the pill's height: its left-middle is where the
/// frame brackets meet it (INS-06).
pub const PILL_HALF_HEIGHT_PX: f32 = 8.0;
/// The white ring on a pill that holds a selected satellite (INS-06) or a
/// hovered shared orbit (HOV-05), the same as a marker's (MRK-01).
const RING_PX: f32 = 2.0;
const RING_OFFSET_PX: f32 = 2.0;

/// GRP-05: where a pill's top-left sits from its parent's marker centre:
/// down and to the right, clear of a marker whose hover ring reaches
/// `parent_clear_px` from its centre.
pub fn pill_offset(parent_clear_px: f32) -> Vec2 {
    Vec2::new(parent_clear_px, PILL_DROP_PX)
}

/// INS-06: where a folded satellite is shown - the left-middle of its
/// parent's pill, given the parent's screen position and how far its
/// marker reaches.
pub fn pill_anchor(parent_screen: Vec2, parent_clear_px: f32) -> Vec2 {
    parent_screen + pill_offset(parent_clear_px) + Vec2::new(0.0, PILL_HALF_HEIGHT_PX)
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

/// GRP-05 / GRP-06: what a pill shows: its count, and the flags that colour
/// its outline and show its dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PillFace {
    pub count: usize,
    pub flags: PillFlags,
}

impl PillFace {
    /// GRP-05: the "+N" text.
    pub fn text(&self) -> String {
        format!("+{}", self.count)
    }

    /// GRP-06: the outline colour: the most pressing affiliation's, or
    /// none for the normal outline.
    pub fn outline(&self) -> Color {
        self.flags
            .pressing
            .map_or(Color::NONE, colors::symbol_color)
    }

    /// GRP-06: whether the blue dot shows.
    pub fn dot(&self) -> Display {
        if self.flags.has_own_ship {
            Display::Flex
        } else {
            Display::None
        }
    }
}

/// GRP-05: the pill's shape: a rounded chip, placed on the map by whoever
/// spawns it and hidden until it is.
pub fn pill_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        display: Display::None,
        align_items: AlignItems::Center,
        column_gap: px(4),
        padding: UiRect::axes(px(6), px(2)),
        border_radius: BorderRadius::MAX,
        ..default()
    }
}

/// GRP-06: the pill's outline, a child of the pill so it can carry its
/// own `Tint` apart from the ground's.
#[derive(Component)]
pub struct PillRim;

/// GRP-05 / GRP-06: a pill showing `face`, as `node` (from `pill_node`).
/// Every coloured part carries a `Tint`, so an identity pill can be faded
/// with its ring (MOT-14); a fold pill rewrites its face each frame
/// instead.
pub fn pill_bundle(fonts: &Fonts, face: PillFace, node: Node) -> impl Bundle {
    let outline = face.outline();
    (
        node,
        BackgroundColor(PILL_GROUND),
        Tint::new(PILL_GROUND, TintSlot::Background),
        selection_outline(false),
        children![
            (
                PillRim,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-1),
                    top: px(-1),
                    right: px(-1),
                    bottom: px(-1),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(outline),
                Tint::new(outline, TintSlot::Border),
                Pickable::IGNORE,
            ),
            (
                PillCountText,
                Text::new(face.text()),
                fonts.data(PILL_FONT_PX),
                TextColor(colors::NEUTRAL),
                Tint::new(colors::NEUTRAL, TintSlot::Text),
            ),
            (
                PillOwnShipDot,
                Node {
                    width: px(OWN_SHIP_DOT_PX),
                    height: px(OWN_SHIP_DOT_PX),
                    display: face.dot(),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(colors::OWN),
                Tint::new(colors::OWN, TintSlot::Background),
            )
        ],
    )
}

/// INS-06 / HOV-05: the white ring round a pill, on or off.
pub fn selection_outline(on: bool) -> Outline {
    Outline {
        width: px(if on { RING_PX } else { 0.0 }),
        offset: px(RING_OFFSET_PX),
        color: if on {
            colors::SELECTION_RING
        } else {
            Color::NONE
        },
    }
}

pub struct PillsPlugin;

impl Plugin for PillsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_pills.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (update_pills.in_set(MarkersPlaced), ring_pills).in_set(Stage::Overlay),
            );
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
                GlobalZIndex(layers::LABELS),
                pill_bundle(&fonts, PillFace::default(), pill_node()),
            ))
            // GRP-07: hover rings the pill as it does a marker; click is
            // the parent's click.
            .observe(hover::on_marker_over)
            .observe(hover::on_marker_out)
            .observe(hover::on_pill_click);
    }
}

/// INS-06 / GRP-07: a pill carries the white ring while a selected object
/// is folded into it, or while it is hovered, as a marker does.
fn ring_pills(
    selection: Res<Selection>,
    folds: Res<FoldSummary>,
    mut pills: Query<(&GroupPillFor, Has<Hovered>, &mut Outline)>,
) {
    for (pill_for, hovered, mut outline) in &mut pills {
        let holds_selection = folds
            .0
            .get(&pill_for.0)
            .is_some_and(|group| group.members.iter().any(|&m| selection.is_selected(m)));
        outline.set_if_neq(selection_outline(pill_ringed(holds_selection, hovered)));
    }
}

/// INS-06 / GRP-07: whether a fold pill shows the white ring.
pub fn pill_ringed(holds_selection: bool, hovered: bool) -> bool {
    holds_selection || hovered
}

/// The parts of a fold pill's face, rewritten each frame.
type PillParts<'w, 's> = (
    Query<'w, 's, &'static mut BorderColor, With<PillRim>>,
    Query<'w, 's, &'static mut Text, With<PillCountText>>,
    Query<'w, 's, &'static mut Node, (With<PillOwnShipDot>, Without<GroupPillFor>)>,
);

fn update_pills(
    screen: Res<ScreenPositions>,
    tunables: Res<Tunables>,
    classes: Query<&BodyClass>,
    folds: Res<FoldSummary>,
    mut pills: Query<(&GroupPillFor, &mut Node, &Children)>,
    (mut rims, mut counts, mut dots): PillParts,
) {
    for (pill_for, mut node, children) in &mut pills {
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
        let clear = classes
            .get(parent)
            .map_or(0.0, |class| ring_outer_half(*class, &tunables));
        let offset = pill_offset(clear);
        node.left = px(pos.x + offset.x);
        node.top = px(pos.y + offset.y);
        let face = PillFace {
            count: group.members.len(),
            flags: PillFlags {
                pressing: group.pressing,
                has_own_ship: group.has_own_ship,
            },
        };
        for child in children.iter() {
            // GRP-06: the outline in the most pressing affiliation's
            // colour.
            if let Ok(mut rim) = rims.get_mut(child) {
                rim.set_all(face.outline());
            }
            if let Ok(mut text) = counts.get_mut(child) {
                *text = Text::new(face.text());
            }
            // GRP-06: blue dot when own ships are parked there.
            if let Ok(mut dot) = dots.get_mut(child) {
                dot.display = face.dot();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::body::Affiliation;

    /// GRP-07 / HOV-01: hovering a fold pill rings it in white, as for a
    /// marker; leaving it clears the ring.
    #[test]
    fn hovering_a_pill_rings_it() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<Selection>();
        world.init_resource::<FoldSummary>();
        let parent = world.spawn_empty().id();
        let pill = world
            .spawn((GroupPillFor(parent), Hovered, selection_outline(false)))
            .id();
        world.run_system_once(ring_pills).unwrap();
        assert_eq!(
            *world.get::<Outline>(pill).unwrap(),
            selection_outline(true)
        );
        world.entity_mut(pill).remove::<Hovered>();
        world.run_system_once(ring_pills).unwrap();
        assert_eq!(
            *world.get::<Outline>(pill).unwrap(),
            selection_outline(false)
        );
    }

    /// INS-06: a folded satellite is shown at its parent's pill, at the
    /// pill's left-middle, which follows the parent on screen.
    #[test]
    fn pill_anchor_is_the_pills_left_middle() {
        let parent = Vec2::new(400.0, 300.0);
        let anchor = pill_anchor(parent, 19.0);
        assert_eq!(anchor.x, parent.x + 19.0);
        assert_eq!(anchor.y, parent.y + PILL_DROP_PX + PILL_HALF_HEIGHT_PX);
        assert_eq!(
            pill_anchor(parent + Vec2::X * 50.0, 19.0),
            anchor + Vec2::X * 50.0
        );
    }

    /// GRP-05 / GRP-06: a pill's face reads "+N", takes the pressing
    /// affiliation's colour for its outline, and shows its dot only for
    /// own ships.
    #[test]
    fn a_pill_face_shows_count_outline_and_dot() {
        let face = PillFace {
            count: 3,
            flags: PillFlags {
                pressing: Some(Affiliation::Hostile),
                has_own_ship: true,
            },
        };
        assert_eq!(face.text(), "+3");
        assert_eq!(face.outline(), colors::symbol_color(Affiliation::Hostile));
        assert_eq!(face.dot(), Display::Flex);
        let plain = PillFace::default();
        assert_eq!(plain.outline(), Color::NONE);
        assert_eq!(plain.dot(), Display::None);
    }
}
