//! The "+N" count pill beside a parent with folded satellites (GRP-05 to
//! GRP-07): the count, an outline in the colour of the most pressing
//! affiliation folded into it, and a blue dot if own ships are parked
//! there. What is folded, and the flags, come from `fold.rs`; this module
//! only draws them. The pill's node and face are shared with the identity
//! pill on a shared exposure ring (`rings/marks.rs`, MOT-14), which is
//! styled and coloured the same way. The brackets round a hovered pill, or
//! one holding the selection, are `brackets.rs` (HOV-06).

use bevy::prelude::*;

use super::layers;

use super::colors;
use super::fold::{FoldSummary, PillFlags};
use super::fonts::Fonts;
use super::hover;
use super::markers::{
    MarkerShape, MarkersPlaced, box_half, drawn_half, marker_half_size, marker_shape,
};
use super::screen::ScreenPositions;
use super::symbols::draw::{Tint, TintSlot};
use super::targets::GroupPillFor;
use super::true_size::DiscRadii;
use super::tunables::Tunables;
use crate::sim::body::BodyClass;
use crate::sim::orbit::Orbit;
use crate::sim::track::ShipRecord;
use crate::{Boot, Stage};

/// GRP-05: how far below its parent's marker centre the pill's top sits.
/// Sideways it sits just clear of the parent's bracket box (HOV-07).
const PILL_DROP_PX: f32 = 8.0;
/// Half the pill's height, about which an identity pill is centred on its
/// ring point (MOT-14).
pub const PILL_HALF_HEIGHT_PX: f32 = 8.0;

/// GRP-05: where a pill's top-left sits from its parent's marker centre:
/// down and to the right, clear of a parent whose bracket box reaches
/// `parent_clear_px` from its centre, whether or not the box is showing.
pub fn pill_offset(parent_clear_px: f32) -> Vec2 {
    Vec2::new(parent_clear_px, PILL_DROP_PX)
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

pub struct PillsPlugin;

impl Plugin for PillsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_pills.in_set(Boot::Overlay))
            .add_systems(
                Update,
                update_pills.in_set(MarkersPlaced).in_set(Stage::Overlay),
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
            // GRP-07: hover brackets the pill as it does a marker; click
            // is the parent's click.
            .observe(hover::on_marker_over)
            .observe(hover::on_marker_out)
            .observe(hover::on_pill_click);
    }
}

/// The parts of a fold pill's face, rewritten each frame.
type PillParts<'w, 's> = (
    Query<'w, 's, &'static mut BorderColor, With<PillRim>>,
    Query<'w, 's, &'static mut Text, With<PillCountText>>,
    Query<'w, 's, &'static mut Node, (With<PillOwnShipDot>, Without<GroupPillFor>)>,
);

/// What a pill's parent looks like, for how far its bracket box reaches.
type ParentLooks<'w, 's> = Query<'w, 's, (&'static BodyClass, Has<ShipRecord>)>;

/// HOV-07 / GRP-05: how far a parent's bracket box reaches from its
/// centre: its drawn marker, grown to a true-size disc when close (MRK-07),
/// and the margin.
pub fn parent_clear_px(
    class: BodyClass,
    has_record: bool,
    true_radius_px: f32,
    tunables: &Tunables,
) -> f32 {
    let is_disc = marker_shape(class, has_record) == MarkerShape::Disc;
    let drawn = drawn_half(marker_half_size(class, tunables), is_disc, true_radius_px);
    box_half(drawn, tunables)
}

fn update_pills(
    (screen, discs): (Res<ScreenPositions>, Res<DiscRadii>),
    tunables: Res<Tunables>,
    parents: ParentLooks,
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
        let clear = parents.get(parent).map_or(0.0, |(class, record)| {
            parent_clear_px(*class, record, discs.get(parent), &tunables)
        });
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

    /// GRP-05 / HOV-07: a pill sits clear of its parent's bracket box,
    /// which grows with the parent's true-size disc.
    #[test]
    fn a_pill_clears_its_parents_bracket_box() {
        let tunables = Tunables::default();
        let margin = tunables.bracket_margin_px;
        let planet = tunables.planet_marker_px / 2.0;
        let far = parent_clear_px(BodyClass::Planet, false, 1.0, &tunables);
        assert_eq!(far, planet + margin);
        let close = parent_clear_px(BodyClass::Planet, false, 90.0, &tunables);
        assert_eq!(close, 90.0 + margin);
        assert_eq!(pill_offset(close).x, close);
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
